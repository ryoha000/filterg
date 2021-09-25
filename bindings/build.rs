fn main() {
    windows::build!(
        Windows::Win32::System::Diagnostics::Debug::{GetLastError},

        Windows::Win32::System::Com::{
            COINIT, CLSCTX,
            CLSIDFromProgID, CoInitializeEx, CoUninitialize, CoCreateInstance,
        },

        Windows::Win32::System::Threading::{CreateEventW, CreateWaitableTimerW, SetWaitableTimer, CancelWaitableTimer},

        Windows::Win32::Foundation::{
            PWSTR, BSTR,
            SysFreeString,
            CloseHandle
        },

        Windows::Win32::Media::Audio::CoreAudio::{MMDeviceEnumerator, IMMDeviceEnumerator, IMMDevice, IAudioClient3, IAudioCaptureClient, AUDCLNT_STREAMFLAGS_LOOPBACK},

        Windows::Win32::Media::Multimedia::{HMMIO, mmioOpenW, mmioWrite, mmioAscend, mmioCreateChunk, MMIO_CREATERIFF, MMIO_WRITE, MMIO_CREATE, MMCKINFO, WAVEFORMATEX, MMSYSERR_NOERROR},
    );
}
