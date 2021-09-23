mod capture;
mod device;
mod event;
mod file;
mod utils;

use bindings::Windows::Win32::Foundation::HANDLE;
use bindings::Windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
use capture::Args;
use device::get_default_device;
use event::create_event;
use std::ptr;
use utils::{CloseHandleOnExit, CoUninitializeOnExit};

use crate::file::open_file;

pub fn wmain() -> windows::Result<u8> {
    unsafe { CoInitializeEx(ptr::null_mut(), COINIT_MULTITHREADED)? };
    let _com = CoUninitializeOnExit {};

    return do_everything();
}

fn do_everything() -> windows::Result<u8> {
    let default_device = get_default_device()?;
    println!("default_device.GetId(): {:#?}", unsafe {
        default_device.GetId().unwrap()
    });

    let h_started_event = create_event()?;
    let _h_started_event = CloseHandleOnExit {
        handle: h_started_event,
    };
    println!("h_started_event: {:#?}", h_started_event);

    let h_stop_event = create_event()?;
    let _h_stop_event = CloseHandleOnExit {
        handle: h_stop_event,
    };
    println!("h_stop_event: {:#?}", h_stop_event);

    let args = Args {
        hr: windows::HRESULT(0),
        mm_device: default_device,
        b_int16: false,
        h_file: open_file("filterg_save.wav")?,
        h_started_event,
        h_stop_event,
        n_frames: 0,
    };

    Ok(0)
}
