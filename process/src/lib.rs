mod capture;
mod device;
mod event;
mod fft;
mod file;
mod utils;

use bindings::Windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
use capture::CaptureEvent;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc};
use std::{ptr, thread};

use utils::{message_to_windows_error, CoUninitializeOnExit};

use crate::utils::from_wide_ptr;

pub fn wmain() -> windows::Result<u8> {
    unsafe { CoInitializeEx(ptr::null_mut(), COINIT_MULTITHREADED)? };
    let _com = CoUninitializeOnExit {};

    return do_everything();
}

fn do_everything() -> windows::Result<u8> {
    let (tx, rx): (Sender<CaptureEvent>, Receiver<CaptureEvent>) = mpsc::channel();
    let is_stopped = Arc::new(AtomicBool::new(false));
    let is_stopped_clone = is_stopped.clone();

    // TODO: 入力を処理して capture に渡すようにする
    let capture_thread = thread::spawn(move || capture::capture_thread_func(tx, is_stopped_clone));

    match rx.recv() {
        Ok(CaptureEvent::Start) => {}
        Ok(e) => {
            return Err(message_to_windows_error(&format!("{:#?}", e)));
        }
        Err(e) => {
            return Err(message_to_windows_error(&format!("{:#?}", e)));
        }
    }

    println!("start capture");

    let sleep_time = std::time::Duration::from_secs(1);
    thread::sleep(sleep_time);

    is_stopped.store(true, std::sync::atomic::Ordering::SeqCst);

    capture_thread.join().unwrap()?;

    Ok(0)
}

pub fn print_device_list() -> windows::Result<u8> {
    unsafe { CoInitializeEx(ptr::null_mut(), COINIT_MULTITHREADED)? };
    let _com = CoUninitializeOnExit {};

    let d = device::get_default_device()?;
    println!("{:#?}", from_wide_ptr(unsafe { d.GetId()?.0 }));
    return device::get_list_devices();
}
