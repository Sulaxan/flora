use windows::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{ShowWindow, SW_HIDE, SW_SHOWNORMAL},
};

pub fn show_window(hwnd: HWND) -> bool {
    unsafe { ShowWindow(hwnd, SW_SHOWNORMAL).into() }
}

pub fn hide_window(hwnd: HWND) -> bool {
    unsafe { ShowWindow(hwnd, SW_HIDE).into() }
}
