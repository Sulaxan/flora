use std::mem;

use anyhow::{anyhow, Result};
use windows::Win32::{
    Foundation::{BOOL, HWND, LPARAM, RECT},
    UI::WindowsAndMessaging::{EnumWindows, GetClassNameW, GetWindowLongPtrW, GetWindowRect, GWLP_USERDATA},
};

use crate::webview::WebView;

#[derive(Debug)]
pub struct FloraProcess {
    pub hwnd: HWND,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
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

        let mut rect = RECT::default();
        GetWindowRect(hwnd, &mut rect).ok();

        windows.push(FloraProcess {
            hwnd,
            x: rect.left,
            y: rect.top,
            width: rect.right - rect.left,
            height: rect.bottom - rect.top,
        });
        mem::forget(windows);
    }

    return true.into();
}
