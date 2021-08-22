pub use crate::{Microsoft::Web::WebView2::Win32::*, Windows};

use std::sync::mpsc;
use Windows::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{self, MSG},
};

use windows::HRESULT;

#[derive(Debug)]
pub enum Error {
    WindowsError(windows::Error),
    CallbackError(String),
    TaskCanceled,
    SendError,
}

impl From<windows::Error> for Error {
    fn from(err: windows::Error) -> Self {
        Self::WindowsError(err)
    }
}

impl From<HRESULT> for Error {
    fn from(err: HRESULT) -> Self {
        Self::WindowsError(windows::Error::fast_error(err))
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// The WebView2 threading model runs everything on the UI thread, including callbacks which it triggers
/// with `PostMessage`, and we're using this here because it's waiting for some async operations in WebView2
/// to finish before starting the main message loop in `WebView::run`. As long as there are no pending
/// results in `rx`, it will pump Window messages and check for a result after each message is dispatched.
///
/// `GetMessage` is a blocking call, so if we want to send results from another thread, senders from other
/// threads should "kick" the message loop after sending the result by calling `PostThreadMessage` with an
/// ignorable/unhandled message such as `WM_APP`.
pub fn wait_with_pump<T>(rx: mpsc::Receiver<T>) -> Result<T> {
    let mut msg = MSG::default();
    let hwnd = HWND::default();

    loop {
        if let Ok(result) = rx.try_recv() {
            return Ok(result);
        }

        unsafe {
            match WindowsAndMessaging::GetMessageA(&mut msg, hwnd, 0, 0).0 {
                -1 => {
                    return Err(HRESULT::from_thread().into());
                }
                0 => return Err(Error::TaskCanceled),
                _ => {
                    WindowsAndMessaging::TranslateMessage(&msg);
                    WindowsAndMessaging::DispatchMessageA(&msg);
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn compare_eq() {
        let mut result = 1;
        unsafe { CompareBrowserVersions("1.0.0", "1.0.0", &mut result) }.unwrap();
        assert_eq!(0, result);
    }

    #[test]
    fn compare_lt() {
        let mut result = 0;
        unsafe { CompareBrowserVersions("1.0.0", "1.0.1", &mut result) }.unwrap();
        assert_eq!(-1, result);
    }

    #[test]
    fn compare_gt() {
        let mut result = 0;
        unsafe { CompareBrowserVersions("2.0.0", "1.0.1", &mut result) }.unwrap();
        assert_eq!(1, result);
    }
}