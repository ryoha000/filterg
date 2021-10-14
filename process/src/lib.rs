mod capture;
mod device;
mod event;
mod fft;
mod render;
mod render_prepare;
mod utils;

use bindings::Windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
use capture::CaptureEvent;
use hound::WavSpec;
use rustfft::num_complex::Complex32;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, Mutex};
use std::{ptr, thread};

use utils::{message_to_windows_error, CoUninitializeOnExit};

use render::RenderQueue;
use utils::from_wide_ptr;

pub fn wmain() -> windows::Result<u8> {
    unsafe { CoInitializeEx(ptr::null_mut(), COINIT_MULTITHREADED)? };
    let _com = CoUninitializeOnExit {};

    return do_everything();
}

fn do_everything() -> windows::Result<u8> {
    // capture スレッドの状態をやりとりするチャンネル
    let (tx, rx): (Sender<CaptureEvent>, Receiver<CaptureEvent>) = mpsc::channel();
    // wave format をやりとりするチャンネル
    let (tx_wf, rx_wf): (Sender<WavSpec>, Receiver<WavSpec>) = mpsc::channel();
    // capture したパケットをやりとりするチャンネル
    let (tx_packet, rx_packet): (Sender<f32>, Receiver<f32>) = mpsc::channel();
    // fft した結果をやりとりするチャンネル
    let (tx_fft, rx_fft) = mpsc::channel::<(usize, usize, Vec<Complex32>)>();

    let is_stopped = Arc::new(AtomicBool::new(false));
    let is_stopped_capture = is_stopped.clone();

    // TODO: 入力を処理して渡すようにする
    let capture_thread = thread::spawn(move || {
        capture::capture_thread_func(tx, tx_wf, tx_packet, is_stopped_capture)
    });

    let is_stopped_fft = is_stopped.clone();
    let fft_thread = thread::spawn(move || {
        fft::fft_scheduler_thread_func(rx_packet, tx_fft, is_stopped_fft).unwrap()
    });

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
    let prepare_render_queue = render_queue.clone();
    let is_stopped_render = is_stopped.clone();
    let is_silence = Arc::new(AtomicBool::new(false));
    let is_silence_clone = is_silence.clone();

    let render_thread = thread::spawn(move || {
        render::render_thread_func(render_queue, is_stopped_render, is_silence_clone)
    });

    let render_prepare_thread = thread::spawn(move || {
        render_prepare::render_prepare_thread_func(rx_fft, prepare_render_queue)
    });

    let sleep_time = std::time::Duration::from_secs(10);
    thread::sleep(sleep_time);

    is_stopped.store(true, std::sync::atomic::Ordering::SeqCst);

    capture_thread.join().unwrap()?;
    render_thread.join().unwrap()?;
    fft_thread.join().unwrap();
    render_prepare_thread.join().unwrap();

    Ok(0)
}

pub fn print_device_list() -> windows::Result<u8> {
    unsafe { CoInitializeEx(ptr::null_mut(), COINIT_MULTITHREADED)? };
    let _com = CoUninitializeOnExit {};

    let d = device::get_default_device()?;
    println!("{:#?}", from_wide_ptr(unsafe { d.GetId()?.0 }));
    return device::get_list_devices();
}
