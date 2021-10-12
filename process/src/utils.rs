use bindings::Windows::Win32::Foundation::{CloseHandle, HANDLE};
use bindings::Windows::Win32::Media::Audio::CoreAudio::IAudioClient3;
use bindings::Windows::Win32::System::Com::CoUninitialize;
use bindings::Windows::Win32::System::Diagnostics::Debug::GetLastError;
use bindings::Windows::Win32::System::Threading::CancelWaitableTimer;

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

pub struct CancelWaitableTimerOnExit {
    pub handle: HANDLE,
}

impl Drop for CancelWaitableTimerOnExit {
    fn drop(&mut self) {
        let result = unsafe { CancelWaitableTimer(self.handle) };
        if !result.as_bool() {
            panic!("panic in drop CancelWaitableTimerOnExit {:#?}", unsafe {
                GetLastError()
            });
        }
    }
}

pub struct AudioClientStopOnExit<'a> {
    pub client: &'a IAudioClient3,
}

impl<'a> Drop for AudioClientStopOnExit<'a> {
    fn drop(&mut self) {
        unsafe { self.client.Stop() }.unwrap();
    }
}

// pub struct AvRevertMmThreadCharacteristicsOnExit {
//     pub h: *mut winapi::ctypes::c_void,
// }

// impl Drop for AvRevertMmThreadCharacteristicsOnExit {
//     fn drop(&mut self) {
//         unsafe { winapi::um::avrt::AvRevertMmThreadCharacteristics(self.h) };
//     }
// }

pub fn message_to_windows_error(msg: &str) -> windows::Error {
    println!("ERROR!!!. msg: {}", msg);
    windows::Error::new(windows::HRESULT(0), msg)
}

pub fn from_wide_ptr(ptr: *const u16) -> String {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    unsafe {
        assert!(!ptr.is_null());
        let len = (0..std::isize::MAX)
            .position(|i| *ptr.offset(i) == 0)
            .unwrap();
        let slice = std::slice::from_raw_parts(ptr, len);
        OsString::from_wide(slice).to_string_lossy().into_owned()
    }
}

pub fn to_wide_chars(s: &str) -> Vec<u16> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    OsStr::new(s)
        .encode_wide()
        .chain(Some(0).into_iter())
        .collect::<Vec<_>>()
}

pub fn get_now_unix_time() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("back to the future")
        .as_nanos()
}

pub const AUDCLNT_BUFFERFLAGS_SILENT: u32 = 2;

pub const FS: usize = 48000;
const DIV_NUM: usize = 1000 * 5;
pub const WINDOW_SIZE: usize = FS / DIV_NUM; // 5ms
pub const HOP_SIZE: usize = FS / 1000 * 1; // 1ms
pub const MIN_TAEGET_FREQ: usize = 980;
pub const MAX_TAEGET_FREQ: usize = 1020;
pub const MIN_TARGET_FREQ_INDEX: usize = (MIN_TAEGET_FREQ as f32 * DIV_NUM as f32) as usize;
pub const MAX_TARGET_FREQ_INDEX: usize = (MAX_TAEGET_FREQ as f32 * DIV_NUM as f32) as usize;
