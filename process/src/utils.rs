use bindings::Windows::Win32::Foundation::{CloseHandle, HANDLE};
use bindings::Windows::Win32::System::Com::CoUninitialize;
use bindings::Windows::Win32::System::Diagnostics::Debug::WIN32_ERROR;

pub struct CoUninitializeOnExit {}

impl Drop for CoUninitializeOnExit {
    fn drop(&mut self) {
        unsafe { CoUninitialize() };
    }
}

pub struct CloseHandleOnExit {
    pub handle: HANDLE,
}

impl Drop for CloseHandleOnExit {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.handle) };
    }
}

pub fn win32_error_to_windows_error(err: WIN32_ERROR) -> windows::Error {
    windows::Error::new(windows::HRESULT(0), &format!("{:#?}", err))
}

pub fn message_to_windows_error(msg: &str) -> windows::Error {
    windows::Error::new(windows::HRESULT(0), msg)
}
