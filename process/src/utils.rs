use bindings::Windows::Win32::Foundation::{CloseHandle, HANDLE};
use bindings::Windows::Win32::System::Com::CoUninitialize;

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

pub fn message_to_windows_error(msg: &str) -> windows::Error {
    windows::Error::new(windows::HRESULT(0), msg)
}
