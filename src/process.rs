use std::mem;

use anyhow::Result;
use lazy_static::lazy_static;
use tokio::runtime::Runtime;
use windows::Win32::{
    Foundation::{BOOL, HWND, LPARAM, RECT},
    UI::WindowsAndMessaging::{
        EnumWindows, GetClassNameW, GetWindowRect, GetWindowThreadProcessId,
    },
};

use crate::pipe::{
    self,
    protocol::{ServerRequest, ServerResponse},
};

lazy_static! {
    /// The global runtime for processes. This is used to run async tasks to retrieve information
    /// about the currently running flora processes.
    static ref RUNTIME: Runtime = Runtime::new().unwrap();
}

#[derive(Debug)]
pub struct FloraProcess {
    pub pid: u32,
    pub hwnd: HWND,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl FloraProcess {
    pub fn send(&self, request: ServerRequest) -> Result<ServerResponse> {
        RUNTIME.block_on(pipe::client::send(
            &pipe::create_pipe_name(self.pid),
            &request,
        ))
    }

    pub async fn send_async(&self, request: ServerRequest) -> Result<ServerResponse> {
        pipe::client::send(&pipe::create_pipe_name(self.pid), &request).await
    }
}

pub fn get_all_flora_processes() -> Vec<FloraProcess> {
    // the vector will get filled with flora processes by the callback
    let processes_ptr = Box::into_raw(Box::new(Vec::new()));

    unsafe { EnumWindows(Some(enum_windows_callback), LPARAM(processes_ptr as isize)).ok() };

    unsafe { *Box::from_raw(processes_ptr) }
}

pub unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let mut read_str = [0u16; 255];
    let len = GetClassNameW(hwnd, &mut read_str) as usize;
    let class_name = String::from_utf16_lossy(&read_str[0..len]);

    if class_name == "flora" {
        let processes_ptr = lparam.0 as *mut Vec<FloraProcess>;
        let mut windows = Box::from_raw(processes_ptr);

        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));

        let mut rect = RECT::default();
        GetWindowRect(hwnd, &mut rect).ok();

        let response = RUNTIME
            .block_on(pipe::client::send(
                &pipe::create_pipe_name(pid),
                &ServerRequest::GetName,
            ))
            .unwrap();

        match response {
            ServerResponse::Name(name) => {
                windows.push(FloraProcess {
                    pid,
                    hwnd,
                    name,
                    x: rect.left,
                    y: rect.top,
                    width: rect.right - rect.left,
                    height: rect.bottom - rect.top,
                });
            }
            _ => (),
        }
        mem::forget(windows);
    }

    return true.into();
}
