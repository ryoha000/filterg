use bindings::{
    Windows::Win32::Media::Audio::CoreAudio::{
        eConsole, eRender, IMMDevice, IMMDeviceEnumerator, MMDeviceEnumerator,
    },
    Windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL},
};

pub fn get_default_device() -> windows::Result<IMMDevice> {
    let mm_device_enumerator: IMMDeviceEnumerator =
        unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)? };

    let default_device =
        unsafe { mm_device_enumerator.GetDefaultAudioEndpoint(eRender, eConsole)? };

    Ok(default_device)
}
