use bindings::Windows::Win32::Foundation::HANDLE;
use bindings::Windows::Win32::System::Threading::CreateEventW;
use std::ptr;

pub fn create_event() -> windows::Result<HANDLE> {
    let handle = unsafe { CreateEventW(ptr::null(), false, false, None) };
    if handle == HANDLE(0) {
        return Err(windows::Error::from_win32());
    }

    Ok(handle)
}
