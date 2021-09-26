use bindings::{
    Windows::Win32::Media::Audio::CoreAudio::{
        eConsole, eRender, IMMDevice, IMMDeviceEnumerator, MMDeviceEnumerator,
    },
    Windows::Win32::{
        Media::Audio::CoreAudio::DEVICE_STATE_ACTIVE,
        Storage::StructuredStorage::STGM_READ,
        System::{
            Com::{CoCreateInstance, CLSCTX_ALL},
            SystemServices::DEVPKEY_Device_FriendlyName,
        },
    },
};

use super::utils::from_wide_ptr;

pub fn get_default_device() -> windows::Result<IMMDevice> {
    let mm_device_enumerator: IMMDeviceEnumerator =
        unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)? };

    let default_device =
        unsafe { mm_device_enumerator.GetDefaultAudioEndpoint(eRender, eConsole)? };

    Ok(default_device)
}

pub fn get_list_devices() -> windows::Result<u8> {
    let mm_device_enumerator: IMMDeviceEnumerator =
        unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)? };

    let mm_device_collection =
        unsafe { mm_device_enumerator.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)? };

    let count = unsafe { mm_device_collection.GetCount()? };

    for i in 0..count {
        let mm_device = unsafe { mm_device_collection.Item(i)? };
        let property_store = unsafe { mm_device.OpenPropertyStore(STGM_READ as u32)? };
        let pv = unsafe { property_store.GetValue(&DEVPKEY_Device_FriendlyName)? };
        unsafe {
            println!(
                "{:#?}",
                from_wide_ptr(pv.Anonymous.Anonymous.Anonymous.pwszVal.0)
            )
        };

        unsafe { println!("{:#?}", from_wide_ptr(mm_device.GetId()?.0)) };
    }

    Ok(0)
}
