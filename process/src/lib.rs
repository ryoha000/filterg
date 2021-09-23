mod capture;
mod device;
mod event;
mod file;
mod utils;

use bindings::Windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
use capture::Args;
use device::get_default_device;
use event::create_event;
use std::{ptr, thread};
use utils::{CloseHandleOnExit, CoUninitializeOnExit};

use crate::file::open_file;

pub fn wmain() -> windows::Result<u8> {
    unsafe { CoInitializeEx(ptr::null_mut(), COINIT_MULTITHREADED)? };
    let _com = CoUninitializeOnExit {};

    return do_everything();
}

fn do_everything() -> windows::Result<u8> {
    let capture_thread = thread::spawn(move || capture::capture());

    Ok(0)
}
