fn main() {
    windows::build!(
        Windows::Win32::System::Com::{
            COINIT, CLSCTX,
            CLSIDFromProgID, CoInitializeEx, CoUninitialize, CoCreateInstance,
        },

        Windows::Win32::Foundation::{
            PWSTR, BSTR,
            SysFreeString,
        },

        Windows::Win32::Media::Audio::CoreAudio::{MMDeviceEnumerator, IMMDeviceEnumerator, IMMDevice},
    );
}
