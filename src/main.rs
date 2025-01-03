use std::{
    sync::{
        atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering},
        Arc, Mutex,
    },
    thread,
};

use anyhow::{anyhow, Result};
use clap::Parser;
use cli::{FloraCli, FloraSubcommand};
use lazy_static::lazy_static;
use pipe::protocol::{ServerRequest, ServerResponse};
use process::get_all_flora_processes;
use tabled::{builder::Builder, settings::Style};
use tokio::runtime;
use tracing::info;
use tracing_subscriber::util::SubscriberInitExt;
use window::{FloraHandle, FloraSender, FloraWindow};
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
mod pipe;
mod process;
mod window;
mod windows_api;

static POS_X: AtomicI32 = AtomicI32::new(0);
static POS_Y: AtomicI32 = AtomicI32::new(0);
static WIDTH: AtomicI32 = AtomicI32::new(200);
static HEIGHT: AtomicI32 = AtomicI32::new(20);
static CONTENT_URL: AtomicBool = AtomicBool::new(false);

static WINDOW_THREAD_ID: AtomicU32 = AtomicU32::new(0);

lazy_static! {
    static ref NAME: Arc<Mutex<String>> = Arc::new(Mutex::new("Generic Flora Widget".to_string()));
    static ref CONTENT: Arc<Mutex<String>> =
        Arc::new(Mutex::new(include_str!("../default.html").to_string()));
    static ref SENDER: Arc<Mutex<Option<FloraSender>>> = Arc::new(Mutex::new(None));
    static ref HANDLE: Arc<Mutex<FloraHandle>> = Arc::new(Mutex::new(FloraHandle::default()));
}

fn create_window() -> Result<FloraWindow> {
    let x = POS_X.load(Ordering::SeqCst);
    let y = POS_Y.load(Ordering::SeqCst);
    let width = WIDTH.load(Ordering::SeqCst);
    let height = HEIGHT.load(Ordering::SeqCst);
    let content = {
        let c = CONTENT.lock().unwrap();
        c.clone()
    };
    let content_url = CONTENT_URL.load(Ordering::SeqCst);

    let window = FloraWindow::new(x, y, width, height, false)?;
    if content_url {
        window.navigate(&content);
    } else {
        window.navigate_to_string(&content);
    }

    Ok(window)
}

fn start_named_pipe_server() {
    thread::spawn(move || {
        let rt = runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            pipe::server::start_server().await.unwrap();
        });
    });
}

fn start() -> Result<()> {
    tracing_subscriber::fmt().init();

    info!("initializing flora");

    let _ = unsafe { SetConsoleCtrlHandler(Some(ctrl_c_handler), true).ok() };

    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
    }
    set_process_dpi_awareness()?;

    let window = create_window()?;
    {
        let mut handle = HANDLE.lock().unwrap();
        *handle = window.get_window().into();
    }
    {
        let mut sender = SENDER.lock().unwrap();
        *sender = Some(window.get_sender());
    }
    WINDOW_THREAD_ID.store(window.thread_id, Ordering::SeqCst);

    start_named_pipe_server();

    window
        .run()
        .map_err(|e| anyhow!("error running widget window: {}", e))
}

fn main() -> Result<()> {
    let cli = FloraCli::parse();
    match cli.command {
        FloraSubcommand::Start { config_path } => {
            if !config_path.is_file() {
                println!("Specified config path is not a valid file");
                return Ok(());
            }
            match config_path.extension() {
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

            let config = config::read(&config_path)?;
            config::load_config(config);

            return start();
        }
        FloraSubcommand::List => {
            let processes = process::get_all_flora_processes();
            if processes.is_empty() {
                println!("There are currently no flora processes");
                return Ok(());
            }

            let mut table_display: Vec<Vec<String>> = processes
                .iter()
                .map(|process| {
                    vec![
                        process.pid.to_string(),
                        (process.hwnd.0 as isize).to_string(),
                        process.name.clone(),
                        process.x.to_string(),
                        process.y.to_string(),
                        process.width.to_string(),
                        process.height.to_string(),
                    ]
                })
                .collect();
            table_display.insert(
                0,
                vec![
                    "PID".to_string(),
                    "HWND".to_string(),
                    "name".to_string(),
                    "x".to_string(),
                    "y".to_string(),
                    "width".to_string(),
                    "height".to_string(),
                ],
            );

            let mut table = Builder::from_iter(table_display.iter()).build();
            table.with(Style::modern());

            println!("Currently running flora processes:\n{}", table.to_string());

            return Ok(());
        }
        FloraSubcommand::Show { all, name } => {
            if !all && name.is_none() {
                println!("Please specify the --all flag or the name of a widget");
                return Ok(());
            }

            let processes = get_all_flora_processes();

            if all {
                for process in processes {
                    let res = process.send(ServerRequest::ShowWindow)?;
                    if let ServerResponse::Err(e) = res {
                        println!("Could not show widget {}: {e}\nContinuing...", process.name);
                    }
                }

                return Ok(());
            }

            let name = name.unwrap();

            if let Some(target) = processes.iter().find(|process| process.name == name) {
                let res = target.send(ServerRequest::ShowWindow)?;
                if let ServerResponse::Err(e) = res {
                    println!("Could not show widget {}: {e}", target.name);
                }
            } else {
                println!("Could not find the specified widget name {}", name);
            }

            return Ok(());
        }
        FloraSubcommand::Hide { all, name } => {
            if !all && name.is_none() {
                println!("Please specify the --all flag or the name of a widget");
                return Ok(());
            }

            let processes = get_all_flora_processes();

            if all {
                for process in processes {
                    let res = process.send(ServerRequest::HideWindow)?;
                    if let ServerResponse::Err(e) = res {
                        println!("Could not hide widget {}: {e}\nContinuing...", process.name);
                    }
                }

                return Ok(());
            }

            let name = name.unwrap();

            if let Some(target) = processes.iter().find(|process| process.name == name) {
                let res = target.send(ServerRequest::HideWindow)?;
                if let ServerResponse::Err(e) = res {
                    println!("Could not hide widget {}: {e}", target.name);
                }
            } else {
                println!("Could not find the specified widget name {}", name);
            }

            return Ok(());
        }
    }
}

fn set_process_dpi_awareness() -> Result<()> {
    unsafe { HiDpi::SetProcessDpiAwareness(HiDpi::PROCESS_PER_MONITOR_DPI_AWARE)? };
    Ok(())
}

/// Executes a function on the webview thread.
pub fn execute<F>(f: F) -> Result<()>
where
    F: FnOnce(FloraWindow) + Send + 'static,
{
    {
        let s = SENDER.lock().unwrap();
        if let Some(sender) = s.as_ref() {
            sender.send(Box::new(f)).expect("send the function");
        }
    }

    // notify the thread to process the function we just sent
    windows_api::send_app_message(WINDOW_THREAD_ID.load(Ordering::SeqCst))?;

    Ok(())
}

pub extern "system" fn ctrl_c_handler(ctrltype: u32) -> BOOL {
    match ctrltype {
        CTRL_C_EVENT => {
            execute(|_| unsafe { WindowsAndMessaging::PostQuitMessage(0) })
                .expect("send ctrl+c quit message");

            true.into()
        }
        _ => false.into(),
    }
}
