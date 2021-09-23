use bindings::Windows::Win32::System::Com::CoUninitialize;

pub struct CoUninitializeOnExit {}

impl Drop for CoUninitializeOnExit {
    fn drop(&mut self) {
        unsafe { CoUninitialize() };
    }
}
