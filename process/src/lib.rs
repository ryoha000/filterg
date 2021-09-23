mod device;
mod event;
mod utils;

use bindings::Windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
use device::get_default_device;
use event::create_event;
use std::ptr;
use utils::{CloseHandleOnExit, CoUninitializeOnExit};

pub fn wmain() -> windows::Result<u8> {
    unsafe { CoInitializeEx(ptr::null_mut(), COINIT_MULTITHREADED)? };
    let _com = CoUninitializeOnExit {};

    return do_everything();
}

fn do_everything() -> windows::Result<u8> {
    let default_device = get_default_device()?;
    println!("{:#?}", unsafe { default_device.GetId().unwrap() });

    let h_started_event = create_event()?;
    let _h_started_event = CloseHandleOnExit {
        handle: h_started_event,
    };
    println!("{:#?}", h_started_event);

    Ok(0)
}
