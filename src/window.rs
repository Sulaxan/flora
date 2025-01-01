use windows::{
    core::w,
    Win32::{
        Foundation::{COLORREF, HWND},
        Graphics::Gdi::CreateSolidBrush,
        System::{
            LibraryLoader::GetModuleHandleW,
            Threading::{GetStartupInfoW, STARTUPINFOW},
        },
        UI::WindowsAndMessaging::{
            CreateWindowExW, RegisterClassW, ShowWindow, SW_SHOWNORMAL, WNDCLASSW, WS_EX_LAYERED,
            WS_EX_TOOLWINDOW, WS_POPUP, WS_VISIBLE,
        },
    },
};

use crate::{color::Color, webview};

pub struct WidgetWindow {
    pub hwnd: HWND,
}

impl WidgetWindow {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        let class_name = w!("flora");
        let h_inst = unsafe { GetModuleHandleW(None).unwrap() };
        let mut startup_info = STARTUPINFOW {
            cb: std::mem::size_of::<STARTUPINFOW>() as u32,
            ..Default::default()
        };
        unsafe { GetStartupInfoW(&mut startup_info) };

        let wc = WNDCLASSW {
            lpfnWndProc: Some(webview::window_proc),
            hInstance: h_inst.into(),
            lpszClassName: class_name,
            hbrBackground: unsafe { CreateSolidBrush(COLORREF(Color::Transparent.bgr())) },
            ..Default::default()
        };

        unsafe { RegisterClassW(&wc) };

        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_LAYERED,
                class_name,
                w!("flora"),
                WS_POPUP | WS_VISIBLE,
                x,
                y,
                width,
                height,
                None,
                None,
                h_inst,
                None,
            )
            .unwrap()
        };

        // SetLayeredWindowAttributes(hwnd, COLORREF(Color::Transparent.bgr()), 25, LWA_COLORKEY).ok();

        let _ = unsafe { ShowWindow(hwnd, SW_SHOWNORMAL) };

        Self { hwnd }
    }
}
