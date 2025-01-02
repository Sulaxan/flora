use std::sync::{
    atomic::{AtomicBool, AtomicI32, Ordering},
    Arc, Mutex,
};

use anyhow::{anyhow, Result};
use clap::Parser;
use cli::Cli;
use lazy_static::lazy_static;
use webview::{WebView, WebViewSender};
use window::WidgetWindow;
use windows::Win32::{
    Foundation::BOOL,
    System::{
        Com::{CoInitializeEx, COINIT_APARTMENTTHREADED},
        Console::{SetConsoleCtrlHandler, CTRL_C_EVENT},
    },
    UI::{
        HiDpi,
        WindowsAndMessaging::{self},
    },
};

mod cli;
mod color;
mod config;
mod webview;
mod window;

static POS_X: AtomicI32 = AtomicI32::new(0);
static POS_Y: AtomicI32 = AtomicI32::new(0);
static WIDTH: AtomicI32 = AtomicI32::new(200);
static HEIGHT: AtomicI32 = AtomicI32::new(20);
static CONTENT_URL: AtomicBool = AtomicBool::new(false);

lazy_static! {
    static ref CONTENT: Arc<Mutex<String>> =
        Arc::new(Mutex::new(include_str!("../default.html").to_string()));
    static ref WEBVIEW_SENDER: Arc<Mutex<Option<WebViewSender>>> = Arc::new(Mutex::new(None));
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    if !cli.config_path.is_file() {
        println!("Specified config path is not a valid file");
        return Ok(());
    }
    match cli.config_path.extension() {
        Some(ext) => {
            if ext != "flora" {
                println!("Config extension must be .flora");
                return Ok(());
            }
        }
        None => {
            println!("Config extension must be .flora");
            return Ok(());
        }
    }

    let config = config::read(&cli.config_path)?;
    config::load_config(config);

    let x = POS_X.load(Ordering::SeqCst);
    let y = POS_Y.load(Ordering::SeqCst);
    let width = WIDTH.load(Ordering::SeqCst);
    let height = HEIGHT.load(Ordering::SeqCst);
    let content = {
        let c = CONTENT.lock().unwrap();
        c.clone()
    };
    let content_url = CONTENT_URL.load(Ordering::SeqCst);

    let _ = unsafe { SetConsoleCtrlHandler(Some(ctrl_c_handler), true).ok() };

    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
    }
    set_process_dpi_awareness()?;

    let window = WidgetWindow::new(x, y, width, height)?;
    if content_url {
        window.webview.navigate(&content);
    } else {
        window.webview.navigate_to_string(&content);
    }

    {
        let mut sender = WEBVIEW_SENDER.lock().unwrap();
        *sender = Some(window.webview.get_sender());
    }

    window
        .run()
        .map_err(|e| anyhow!("error running widget window: {}", e))
}

pub fn set_process_dpi_awareness() -> Result<()> {
    unsafe { HiDpi::SetProcessDpiAwareness(HiDpi::PROCESS_PER_MONITOR_DPI_AWARE)? };
    Ok(())
}

pub extern "system" fn ctrl_c_handler(ctrltype: u32) -> BOOL {
    match ctrltype {
        CTRL_C_EVENT => {
            let sender = WEBVIEW_SENDER.lock().unwrap();
            if let Some(s) = sender.as_ref() {
                s.send(Box::new(|_webview| unsafe {
                    WindowsAndMessaging::PostQuitMessage(0)
                }))
                .expect("send ctrl+c quit message");
            }

            true.into()
        }
        _ => false.into(),
    }
}
