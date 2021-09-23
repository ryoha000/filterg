use bindings::Windows::Win32::Foundation::HANDLE;
use bindings::Windows::Win32::System::Diagnostics::Debug::GetLastError;
use bindings::Windows::Win32::System::Threading::CreateEventW;
use std::ptr;

use super::utils::win32_error_to_windows_error;

pub fn create_event() -> windows::Result<HANDLE> {
    let handle = unsafe { CreateEventW(ptr::null(), false, false, None) };
    if handle == HANDLE(0) {
        let err = unsafe { GetLastError() };
        return Err(win32_error_to_windows_error(err));
    }

    Ok(handle)
}
