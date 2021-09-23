mod device;
mod utils;

use bindings::Windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
use device::get_default_device;
use std::ptr;
use utils::CoUninitializeOnExit;

pub fn wmain() -> windows::Result<u8> {
    unsafe { CoInitializeEx(ptr::null_mut(), COINIT_MULTITHREADED)? };
    let _com = CoUninitializeOnExit {};

    return do_everything();
}

fn do_everything() -> windows::Result<u8> {
    let default_device = get_default_device()?;
    println!("{:#?}", unsafe { default_device.GetId().unwrap() });

    Ok(0)
}
