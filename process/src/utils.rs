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

pub struct AudioClientStopOnExit {
    pub client: IAudioClient3,
}

impl Drop for AudioClientStopOnExit {
    fn drop(&mut self) {
        unsafe { self.client.Stop() }.unwrap();
    }
}

pub fn message_to_windows_error(msg: &str) -> windows::Error {
    println!("ERROR!!!. msg: {}", msg);
    windows::Error::new(windows::HRESULT(0), msg)
}

pub const AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY: u32 = 1;
