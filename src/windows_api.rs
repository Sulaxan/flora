use anyhow::Result;
use windows::Win32::{
    Foundation::{HWND, LPARAM, WPARAM},
    UI::WindowsAndMessaging::{self, ShowWindow, SW_HIDE, SW_SHOWNORMAL},
};

pub fn show_window(hwnd: HWND) -> bool {
    unsafe { ShowWindow(hwnd, SW_SHOWNORMAL).into() }
}

pub fn hide_window(hwnd: HWND) -> bool {
    unsafe { ShowWindow(hwnd, SW_HIDE).into() }
}

pub fn send_app_message(thread_id: u32) -> Result<()> {
    unsafe {
        Ok(WindowsAndMessaging::PostThreadMessageW(
            thread_id,
            WindowsAndMessaging::WM_APP,
            WPARAM::default(),
            LPARAM::default(),
        )?)
    }
}
