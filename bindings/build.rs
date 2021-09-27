fn main() {
    windows::build!(
        Windows::Win32::System::Diagnostics::Debug::{GetLastError},

        Windows::Win32::System::Com::{
            COINIT, CLSCTX,
            CLSIDFromProgID, CoInitializeEx, CoUninitialize, CoCreateInstance,
        },

        Windows::Win32::System::Threading::{CreateEventW, CreateWaitableTimerW, SetWaitableTimer, CancelWaitableTimer, WaitForMultipleObjects},

        Windows::Win32::Foundation::{
            PWSTR, BSTR,
            SysFreeString,
            CloseHandle
        },

        Windows::Win32::Media::Audio::CoreAudio::{MMDeviceEnumerator, IMMDeviceEnumerator, IMMDevice, IMMDeviceCollection, IAudioClient3, IAudioRenderClient, IAudioCaptureClient, AUDCLNT_STREAMFLAGS_LOOPBACK, AUDCLNT_STREAMFLAGS_EVENTCALLBACK, DEVICE_STATE_ACTIVE},

        Windows::Win32::Storage::StructuredStorage::STGM_READ,

        Windows::Win32::System::PropertiesSystem::IPropertyStore,

        Windows::Win32::System::SystemServices::DEVPKEY_Device_FriendlyName,

        Windows::Win32::Media::Multimedia::{HMMIO, mmioOpenW, mmioWrite, mmioAscend, mmioCreateChunk, MMIO_CREATERIFF, MMIO_WRITE, MMIO_CREATE, MMCKINFO, WAVEFORMATEX, MMSYSERR_NOERROR},
    );
}
