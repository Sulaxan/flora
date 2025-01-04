//! Creates and manages webviews.
//!
//! A lot of this code comes from the webview2-rs sample, but with some modifications for our needs:
//! https://github.com/wravery/webview2-rs/blob/main/crates/webview2-com/examples/sample.rs
use std::{cell::RefCell, collections::HashMap, fmt, mem, ptr, rc::Rc, sync::mpsc};

use anyhow::{anyhow, Result};
use serde::Deserialize;
use serde_json::Value;
use tracing::info;
use windows::{
    core::*,
    Win32::{
        Foundation::{COLORREF, E_POINTER, HWND, LPARAM, LRESULT, RECT, SIZE, WPARAM},
        Graphics::Gdi::CreateSolidBrush,
        System::{
            LibraryLoader::GetModuleHandleW,
            Threading::{self, GetStartupInfoW, STARTUPINFOW},
            WinRT::EventRegistrationToken,
        },
        UI::WindowsAndMessaging::{
            self, CreateWindowExW, RegisterClassW, ShowWindow, MSG, SW_SHOWNORMAL,
            WINDOW_LONG_PTR_INDEX, WNDCLASSW, WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_POPUP,
            WS_VISIBLE,
        },
    },
};

use webview2_com::{Microsoft::Web::WebView2::Win32::*, *};

use crate::color::Color;

#[derive(Debug)]
pub enum Error {
    WebView2Error(webview2_com::Error),
    WindowsError(windows::core::Error),
    JsonError(serde_json::Error),
    LockError,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl From<webview2_com::Error> for Error {
    fn from(err: webview2_com::Error) -> Self {
        Self::WebView2Error(err)
    }
}

impl From<windows::core::Error> for Error {
    fn from(err: windows::core::Error) -> Self {
        Self::WindowsError(err)
    }
}

impl From<HRESULT> for Error {
    fn from(err: HRESULT) -> Self {
        Self::WindowsError(windows::core::Error::from(err))
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Self::JsonError(err)
    }
}

impl<'a, T: 'a> From<std::sync::PoisonError<T>> for Error {
    fn from(_: std::sync::PoisonError<T>) -> Self {
        Self::LockError
    }
}

impl<'a, T: 'a> From<std::sync::TryLockError<T>> for Error {
    fn from(_: std::sync::TryLockError<T>) -> Self {
        Self::LockError
    }
}

struct WebViewController(ICoreWebView2Controller);

pub type FloraSender = mpsc::Sender<Box<dyn FnOnce(FloraWindow) + Send>>;
type FloraReceiver = mpsc::Receiver<Box<dyn FnOnce(FloraWindow) + Send>>;
type BindingCallback = Box<dyn FnMut(Vec<Value>) -> Result<Value>>;
type BindingsMap = HashMap<String, BindingCallback>;

#[derive(Default)]
pub struct FloraHandle(pub isize);

impl From<HWND> for FloraHandle {
    fn from(value: HWND) -> Self {
        Self(value.0 as isize)
    }
}

impl From<FloraHandle> for HWND {
    fn from(value: FloraHandle) -> Self {
        Self(value.0 as *mut std::ffi::c_void)
    }
}

#[derive(Clone)]
pub struct FloraWindow {
    controller: Rc<WebViewController>,
    webview: Rc<ICoreWebView2>,
    tx: FloraSender,
    rx: Rc<FloraReceiver>,
    pub thread_id: u32,
    bindings: Rc<RefCell<BindingsMap>>,
    pub hwnd: Rc<HWND>,
    // either a string or url
    content: Rc<RefCell<String>>,
    // whether the content is a url
    content_url: Rc<RefCell<bool>>,
}

impl Drop for WebViewController {
    fn drop(&mut self) {
        unsafe { self.0.Close() }.unwrap();
    }
}

#[derive(Debug, Deserialize)]
struct InvokeMessage {
    id: u64,
    method: String,
    params: Vec<Value>,
}

impl FloraWindow {
    fn create_window(x: i32, y: i32, width: i32, height: i32) -> Result<HWND> {
        let class_name = w!("flora");
        let h_inst = unsafe { GetModuleHandleW(None)? };
        let mut startup_info = STARTUPINFOW {
            cb: std::mem::size_of::<STARTUPINFOW>() as u32,
            ..Default::default()
        };
        unsafe { GetStartupInfoW(&mut startup_info) };

        let wc = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
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
            )?
        };

        // SetLayeredWindowAttributes(hwnd, COLORREF(Color::Transparent.bgr()), 25, LWA_COLORKEY).ok();

        let _ = unsafe { ShowWindow(hwnd, SW_SHOWNORMAL) };

        Ok(hwnd)
    }

    pub fn new(x: i32, y: i32, width: i32, height: i32, debug: bool) -> Result<FloraWindow> {
        let hwnd = Self::create_window(x, y, width, height)?;

        let environment = {
            let (tx, rx) = mpsc::channel();

            CreateCoreWebView2EnvironmentCompletedHandler::wait_for_async_operation(
                Box::new(|environmentcreatedhandler| unsafe {
                    CreateCoreWebView2Environment(&environmentcreatedhandler)
                        .map_err(webview2_com::Error::WindowsError)
                }),
                Box::new(move |error_code, environment| {
                    error_code?;
                    tx.send(environment.ok_or_else(|| windows::core::Error::from(E_POINTER)))
                        .expect("send over mpsc channel");
                    Ok(())
                }),
            )
            .map_err(|e| anyhow!("could not create webview environment: {e:#?}"))?;

            rx.recv()
                .map_err(|_| Error::WebView2Error(webview2_com::Error::SendError))
                .map_err(|e| anyhow!("could not receive environment: {e:#?}"))?
        }?;

        let controller: ICoreWebView2Controller = {
            let (tx, rx) = mpsc::channel();

            CreateCoreWebView2ControllerCompletedHandler::wait_for_async_operation(
                Box::new(move |handler| unsafe {
                    environment
                        .CreateCoreWebView2Controller(hwnd, &handler)
                        .map_err(webview2_com::Error::WindowsError)
                }),
                Box::new(move |error_code, controller| {
                    error_code?;
                    tx.send(controller.ok_or_else(|| windows::core::Error::from(E_POINTER)))
                        .expect("send over mpsc channel");
                    Ok(())
                }),
            )
            .map_err(|e| anyhow!("could not create controller: {e:#?}"))?;

            rx.recv()
                .map_err(|_| Error::WebView2Error(webview2_com::Error::SendError))
                .map_err(|e| anyhow!("could not receive controller: {e:#?}"))?
        }?;

        let size = get_window_size(hwnd);
        let mut client_rect = RECT::default();
        unsafe {
            let _ = WindowsAndMessaging::GetClientRect(hwnd, &mut client_rect);
            controller.SetBounds(RECT {
                left: 0,
                top: 0,
                right: size.cx,
                bottom: size.cy,
            })?;
            controller.SetIsVisible(true)?;
        }

        let webview = unsafe { controller.CoreWebView2()? };

        if !debug {
            unsafe {
                let settings = webview.Settings()?;
                settings.SetAreDefaultContextMenusEnabled(false)?;
                settings.SetAreDevToolsEnabled(false)?;
            }
        }

        let (tx, rx) = mpsc::channel();
        let rx = Rc::new(rx);
        let thread_id = unsafe { Threading::GetCurrentThreadId() };

        let webview = FloraWindow {
            controller: Rc::new(WebViewController(controller)),
            webview: Rc::new(webview),
            tx,
            rx,
            thread_id,
            bindings: Rc::new(RefCell::new(HashMap::new())),
            hwnd: Rc::new(hwnd),
            content: Rc::new(RefCell::new(String::new())),
            content_url: Rc::new(RefCell::new(false)),
        };

        // Inject the invoke handler.
        webview
            .init(r#"window.external = { invoke: s => window.chrome.webview.postMessage(s) };"#)?;

        let bindings = webview.bindings.clone();
        let bound = webview.clone();
        unsafe {
            let mut _token = EventRegistrationToken::default();
            webview.webview.add_WebMessageReceived(
                &WebMessageReceivedEventHandler::create(Box::new(move |_webview, args| {
                    if let Some(args) = args {
                        let mut message = PWSTR(ptr::null_mut());
                        if args.WebMessageAsJson(&mut message).is_ok() {
                            let message = CoTaskMemPWSTR::from(message);
                            if let Ok(value) =
                                serde_json::from_str::<InvokeMessage>(&message.to_string())
                            {
                                let mut bindings = bindings.borrow_mut();
                                if let Some(f) = bindings.get_mut(&value.method) {
                                    match (*f)(value.params) {
                                        Ok(result) => bound.resolve(value.id, 0, result),
                                        Err(err) => bound.resolve(
                                            value.id,
                                            1,
                                            Value::String(format!("{err:#?}")),
                                        ),
                                    }
                                    .unwrap();
                                }
                            }
                        }
                    }
                    Ok(())
                })),
                &mut _token,
            )?;
        }

        Ok(webview)
    }

    pub fn run(self) -> Result<()> {
        let content = self.content.borrow().clone();
        let content_url = self.content_url.borrow().clone();

        if !content.is_empty() {
            let (tx, rx) = mpsc::channel();

            let handler =
                NavigationCompletedEventHandler::create(Box::new(move |_sender, _args| {
                    tx.send(()).expect("send over mpsc channel");
                    Ok(())
                }));
            let mut token = EventRegistrationToken::default();

            let webview = self.webview.as_ref();
            unsafe {
                webview.add_NavigationCompleted(&handler, &mut token)?;
                let content = CoTaskMemPWSTR::from(content.as_str());
                match content_url {
                    true => webview.Navigate(*content.as_ref().as_pcwstr())?,
                    false => webview.NavigateToString(*content.as_ref().as_pcwstr())?,
                };
                let result = webview2_com::wait_with_pump(rx);
                webview.remove_NavigationCompleted(token)?;
                result.map_err(|e| anyhow!("could not navigate: {e:#?}"))?;
            }

            info!(content, content_is_url = content_url, "loaded webview content");
        }

        let mut msg = MSG::default();
        let hwnd = HWND::default();

        info!("starting window process loop");
        loop {
            while let Ok(f) = self.rx.try_recv() {
                (f)(self.clone());
            }

            unsafe {
                let result = WindowsAndMessaging::GetMessageW(&mut msg, hwnd, 0, 0).0;

                match result {
                    -1 => break Err(windows::core::Error::from_win32().into()),
                    0 => break Ok(()),
                    _ => match msg.message {
                        WindowsAndMessaging::WM_APP => (),
                        _ => {
                            let _ = WindowsAndMessaging::TranslateMessage(&msg);
                            WindowsAndMessaging::DispatchMessageW(&msg);
                        }
                    },
                }
            }
        }
    }

    pub fn terminate(self) -> Result<()> {
        self.dispatch(|_webview| unsafe {
            WindowsAndMessaging::PostQuitMessage(0);
        })?;

        FloraWindow::set_window_webview(self.get_window(), None);

        Ok(())
    }

    pub fn get_sender(&self) -> FloraSender {
        self.tx.clone()
    }

    pub fn get_window(&self) -> HWND {
        *self.hwnd.clone()
    }

    pub fn navigate(&self, url: &str) -> &Self {
        *self.content.borrow_mut() = url.into();
        *self.content_url.borrow_mut() = true;
        self
    }

    pub fn navigate_to_string(&self, str: &str) -> &Self {
        *self.content.borrow_mut() = str.into();
        *self.content_url.borrow_mut() = false;
        self
    }

    pub fn init(&self, js: &str) -> Result<&Self> {
        let webview = self.webview.clone();
        let js = String::from(js);
        AddScriptToExecuteOnDocumentCreatedCompletedHandler::wait_for_async_operation(
            Box::new(move |handler| unsafe {
                let js = CoTaskMemPWSTR::from(js.as_str());
                webview
                    .AddScriptToExecuteOnDocumentCreated(*js.as_ref().as_pcwstr(), &handler)
                    .map_err(webview2_com::Error::WindowsError)
            }),
            Box::new(|error_code, _id| error_code),
        )
        .map_err(|e| anyhow!("could not add script: {e:#?}"))?;

        Ok(self)
    }

    pub fn eval(&self, js: &str) -> Result<&Self> {
        let webview = self.webview.clone();
        let js = String::from(js);
        ExecuteScriptCompletedHandler::wait_for_async_operation(
            Box::new(move |handler| unsafe {
                let js = CoTaskMemPWSTR::from(js.as_str());
                webview
                    .ExecuteScript(*js.as_ref().as_pcwstr(), &handler)
                    .map_err(webview2_com::Error::WindowsError)
            }),
            Box::new(|error_code, _result| error_code),
        )
        .map_err(|e| anyhow!("could not execute script: {e:#?}"))?;
        Ok(self)
    }

    pub fn dispatch<F>(&self, f: F) -> Result<&Self>
    where
        F: FnOnce(FloraWindow) + Send + 'static,
    {
        self.tx.send(Box::new(f)).expect("send the fn");

        unsafe {
            WindowsAndMessaging::PostThreadMessageW(
                self.thread_id,
                WindowsAndMessaging::WM_APP,
                WPARAM::default(),
                LPARAM::default(),
            )?;
        }
        Ok(self)
    }

    pub fn bind<F>(&self, name: &str, f: F) -> Result<&Self>
    where
        F: FnMut(Vec<Value>) -> Result<Value> + 'static,
    {
        self.bindings
            .borrow_mut()
            .insert(String::from(name), Box::new(f));

        let js = String::from(
            r#"
            (function() {
                var name = '"#,
        ) + name
            + r#"';
                var RPC = window._rpc = (window._rpc || {nextSeq: 1});
                window[name] = function() {
                    var seq = RPC.nextSeq++;
                    var promise = new Promise(function(resolve, reject) {
                        RPC[seq] = {
                            resolve: resolve,
                            reject: reject,
                        };
                    });
                    window.external.invoke({
                        id: seq,
                        method: name,
                        params: Array.prototype.slice.call(arguments),
                    });
                    return promise;
                }
            })()"#;

        self.init(&js)
    }

    pub fn resolve(&self, id: u64, status: i32, result: Value) -> Result<&Self> {
        let result = result.to_string();

        self.dispatch(move |webview| {
            let method = match status {
                0 => "resolve",
                _ => "reject",
            };
            let js = format!(
                r#"
                window._rpc[{id}].{method}({result});
                window._rpc[{id}] = undefined;"#
            );

            webview.eval(&js).expect("eval return script");
        })
    }

    fn set_window_webview(
        hwnd: HWND,
        webview: Option<Box<FloraWindow>>,
    ) -> Option<Box<FloraWindow>> {
        unsafe {
            match SetWindowLong(
                hwnd,
                WindowsAndMessaging::GWLP_USERDATA,
                match webview {
                    Some(webview) => Box::into_raw(webview) as _,
                    None => 0_isize,
                },
            ) {
                0 => None,
                ptr => Some(Box::from_raw(ptr as *mut _)),
            }
        }
    }

    fn get_window_webview(hwnd: HWND) -> Option<Box<FloraWindow>> {
        unsafe {
            let data = GetWindowLong(hwnd, WindowsAndMessaging::GWLP_USERDATA);

            match data {
                0 => None,
                _ => {
                    let webview_ptr = data as *mut FloraWindow;
                    let raw = Box::from_raw(webview_ptr);
                    let webview = raw.clone();
                    mem::forget(raw);

                    Some(webview)
                }
            }
        }
    }
}

pub extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    let webview = match FloraWindow::get_window_webview(hwnd) {
        Some(webview) => webview,
        None => return unsafe { WindowsAndMessaging::DefWindowProcW(hwnd, msg, w_param, l_param) },
    };

    match msg {
        WindowsAndMessaging::WM_SIZE => {
            let size = get_window_size(hwnd);
            unsafe {
                webview
                    .controller
                    .0
                    .SetBounds(RECT {
                        left: 0,
                        top: 0,
                        right: size.cx,
                        bottom: size.cy,
                    })
                    .unwrap();
            }
            LRESULT::default()
        }

        WindowsAndMessaging::WM_CLOSE => {
            unsafe {
                let _ = WindowsAndMessaging::DestroyWindow(hwnd);
            }
            LRESULT::default()
        }

        WindowsAndMessaging::WM_DESTROY => {
            webview.terminate().expect("window is gone");
            LRESULT::default()
        }

        WindowsAndMessaging::WM_QUERYENDSESSION => {
            LRESULT(1) // TRUE
        }

        WindowsAndMessaging::WM_ENDSESSION => {
            webview.terminate().expect("window is gone");
            let _ = unsafe { WindowsAndMessaging::DestroyWindow(hwnd) };
            LRESULT::default()
        }

        _ => unsafe { WindowsAndMessaging::DefWindowProcW(hwnd, msg, w_param, l_param) },
    }
}

fn get_window_size(hwnd: HWND) -> SIZE {
    let mut client_rect = RECT::default();
    let _ = unsafe { WindowsAndMessaging::GetClientRect(hwnd, &mut client_rect) };
    SIZE {
        cx: client_rect.right - client_rect.left,
        cy: client_rect.bottom - client_rect.top,
    }
}

#[allow(non_snake_case)]
#[cfg(target_pointer_width = "32")]
unsafe fn SetWindowLong(window: HWND, index: WINDOW_LONG_PTR_INDEX, value: isize) -> isize {
    WindowsAndMessaging::SetWindowLongW(window, index, value as _) as _
}

#[allow(non_snake_case)]
#[cfg(target_pointer_width = "64")]
unsafe fn SetWindowLong(window: HWND, index: WINDOW_LONG_PTR_INDEX, value: isize) -> isize {
    WindowsAndMessaging::SetWindowLongPtrW(window, index, value)
}

#[allow(non_snake_case)]
#[cfg(target_pointer_width = "32")]
unsafe fn GetWindowLong(window: HWND, index: WINDOW_LONG_PTR_INDEX) -> isize {
    WindowsAndMessaging::GetWindowLongW(window, index) as _
}

#[allow(non_snake_case)]
#[cfg(target_pointer_width = "64")]
unsafe fn GetWindowLong(window: HWND, index: WINDOW_LONG_PTR_INDEX) -> isize {
    WindowsAndMessaging::GetWindowLongPtrW(window, index)
}
