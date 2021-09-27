mod capture;
mod device;
mod event;
mod fft;
mod file;
mod render;
mod utils;

use bindings::Windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
use capture::CaptureEvent;
use hound::WavSpec;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::JoinHandle;
use std::{ptr, thread};

use utils::{message_to_windows_error, CoUninitializeOnExit};

use crate::fft::FftQueue;
use crate::render::RenderQueue;
use crate::utils::from_wide_ptr;

pub fn wmain() -> windows::Result<u8> {
    unsafe { CoInitializeEx(ptr::null_mut(), COINIT_MULTITHREADED)? };
    let _com = CoUninitializeOnExit {};

    return do_everything();
}

fn do_everything() -> windows::Result<u8> {
    let (tx, rx): (Sender<CaptureEvent>, Receiver<CaptureEvent>) = mpsc::channel();
    let (tx_wf, rx_wf): (Sender<WavSpec>, Receiver<WavSpec>) = mpsc::channel();

    let fft_queue = Arc::new(Mutex::new(FftQueue::new(0)));

    let fft_queue_capture = fft_queue.clone();
    let is_stopped = Arc::new(AtomicBool::new(false));
    let is_stopped_capture = is_stopped.clone();

    // TODO: 入力を処理して渡すようにする
    // TODO: fft_queue を渡す実装カスだから何とかする。チャンネルで sample をもらって queue に詰め替えるだけのスレッドがあると良い？
    let capture_thread = thread::spawn(move || {
        capture::capture_thread_func(tx, tx_wf, is_stopped_capture, fft_queue_capture)
    });

    let fft_queue_fft = fft_queue.clone();
    let is_stopped_fft = is_stopped.clone();
    let fft_thread = thread::spawn(move || fft::fft_thread_func(fft_queue_fft, is_stopped_fft));

    // capture_thread の準備を待つ
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

    // capture_thread の準備ができたら
    let wf: WavSpec;
    match rx_wf.recv() {
        Ok(e) => {
            wf = e;
        }
        Err(e) => {
            return Err(message_to_windows_error(&format!("{:#?}", e)));
        }
    }

    let render_queue = Arc::new(Mutex::new(RenderQueue::new(wf.channels)));
    let is_stopped_render = is_stopped.clone();
    let is_silence = Arc::new(AtomicBool::new(true));
    let is_silence_clone = is_silence.clone();

    // let render_thread = thread::spawn(move || {
    //     render::render_thread_func(render_queue, is_stopped_render, is_silence_clone)
    // });

    let sleep_time = std::time::Duration::from_secs(5);
    thread::sleep(sleep_time);

    is_stopped.store(true, std::sync::atomic::Ordering::SeqCst);

    capture_thread.join().unwrap()?;
    // render_thread.join().unwrap()?;
    fft_thread.join().unwrap();

    Ok(0)
}

pub fn print_device_list() -> windows::Result<u8> {
    unsafe { CoInitializeEx(ptr::null_mut(), COINIT_MULTITHREADED)? };
    let _com = CoUninitializeOnExit {};

    let d = device::get_default_device()?;
    println!("{:#?}", from_wide_ptr(unsafe { d.GetId()?.0 }));
    return device::get_list_devices();
}

pub fn test_thread() -> windows::Result<u8> {
    let (tx, rx): (Sender<u8>, Receiver<u8>) = mpsc::channel();

    let threads = (0..5)
        .map(|i| {
            let tx_c = tx.clone();
            thread::spawn(move || {
                for vv in 0..5 {
                    tx_c.send(vv + 5 * i).unwrap();
                    println!("send thread: {}, vv: {}", i, vv);
                }
                0
            })
        })
        .collect::<Vec<JoinHandle<_>>>();

    let _ = threads.into_iter().map(|j| j.join().unwrap());

    for r in rx {
        println!("receie: {}", r)
    }
    Ok(0)
}
