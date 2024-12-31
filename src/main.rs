use std::sync::{
    atomic::{AtomicI32, Ordering},
    Arc, Mutex,
};

use anyhow::{anyhow, Result};
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

mod color;
mod webview;
mod window;

const WIDTH: AtomicI32 = AtomicI32::new(200);
const HEIGHT: AtomicI32 = AtomicI32::new(20);

lazy_static! {
    static ref WEBVIEW_SENDER: Arc<Mutex<Option<WebViewSender>>> = Arc::new(Mutex::new(None));
}

fn main() -> Result<()> {
    let _ = unsafe { SetConsoleCtrlHandler(Some(ctrl_c_handler), true).ok() };

    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
    }
    set_process_dpi_awareness()?;

    let width = WIDTH.load(Ordering::SeqCst);
    let height = HEIGHT.load(Ordering::SeqCst);

    let window = WidgetWindow::new(0, 0, width, height);
    let webview = WebView::create(window.hwnd, false)
        .map_err(|e| anyhow!("could not create webview: {}", e))?;
    webview.navigate_to_string(include_str!("default.html"));

    {
        let mut sender = WEBVIEW_SENDER.lock().unwrap();
        *sender = Some(webview.get_sender());
    }

    webview
        .run()
        .map_err(|e| anyhow!("error running webview: {}", e))
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
