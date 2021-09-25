mod capture;
mod device;
mod event;
mod file;
mod utils;

use bindings::Windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
use capture::CaptureEvent;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::{ptr, thread};

use utils::{message_to_windows_error, CoUninitializeOnExit};

pub fn wmain() -> windows::Result<u8> {
    unsafe { CoInitializeEx(ptr::null_mut(), COINIT_MULTITHREADED)? };
    let _com = CoUninitializeOnExit {};

    return do_everything();
}

fn do_everything() -> windows::Result<u8> {
    let (tx, rx): (Sender<CaptureEvent>, Receiver<CaptureEvent>) = mpsc::channel();
    let is_stopped = AtomicBool::new(false);

    // TODO: 入力を処理して capture に渡すようにする
    let capture_thread = thread::spawn(move || capture::capture_thread_func(tx, is_stopped));

    match rx.recv() {
        Ok(ev) => match ev {
            CaptureEvent::Start => {} // start イベントなら次に進む
            not_start_event => {
                return Err(message_to_windows_error(&format!("{:#?}", not_start_event)));
            }
        },
        Err(e) => {
            return Err(message_to_windows_error(&format!("{:#?}", e)));
        }
    }

    Ok(0)
}
