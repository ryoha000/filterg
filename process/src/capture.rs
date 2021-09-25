use crate::utils::{message_to_windows_error, CancelWaitableTimerOnExit};

use super::device::get_default_device;
use super::event::create_event;
use super::file::open_file;
use super::utils::{CloseHandleOnExit, CoUninitializeOnExit};
use bindings::Windows::Win32::Media::Audio::CoreAudio::{
    IAudioCaptureClient, IAudioClient3, AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_LOOPBACK,
};
use bindings::Windows::Win32::Media::Multimedia::HMMIO;
use bindings::Windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
use bindings::Windows::Win32::System::Threading::{CreateWaitableTimerW, SetWaitableTimer};
use bindings::Windows::Win32::{Foundation::HANDLE, Media::Audio::CoreAudio::IMMDevice};
use std::panic::panic_any;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;
use std::{mem, ptr};
use windows::Interface;

pub struct Args {
    pub mm_device: IMMDevice,
    pub h_file: HMMIO,
    pub is_stopped: AtomicBool,
    pub n_frames: u32,
}

#[derive(Debug)]
pub enum CaptureEvent {
    Start,
    Exit,
}
struct DeferChan {
    tx: Sender<CaptureEvent>,
}

impl Drop for DeferChan {
    fn drop(&mut self) {
        let s = self.tx.send(CaptureEvent::Exit);
        match s {
            Ok(_) => {}
            Err(e) => panic_any(e),
        }
    }
}

pub fn capture_thread_func(
    tx: Sender<CaptureEvent>,
    is_stopped: AtomicBool,
) -> windows::Result<u8> {
    let _defer = DeferChan { tx: tx.clone() };

    unsafe { CoInitializeEx(ptr::null_mut(), COINIT_MULTITHREADED)? };
    let _com = CoUninitializeOnExit {};

    // args を作成。TODO: 入力を受け取るようにする

    let default_device = get_default_device()?;
    println!("default_device.GetId(): {:#?}", unsafe {
        default_device.GetId().unwrap()
    });

    let h_stop_event = create_event()?;
    let _h_stop_event = CloseHandleOnExit {
        handle: h_stop_event,
    };
    println!("h_stop_event: {:#?}", h_stop_event);

    let args = Args {
        mm_device: default_device,
        h_file: open_file("filterg_save.wav")?,
        is_stopped,
        n_frames: 0,
    };

    println!("stup args. ");

    capture(tx, args)?;
    Ok(0)
}

fn capture(tx: Sender<CaptureEvent>, args: Args) -> windows::Result<u8> {
    // TODO: https://docs.microsoft.com/en-us/windows-hardware/drivers/audio/low-latency-audio#windows-audio-session-api-wasapi
    let audio_client: IAudioClient3 = unsafe {
        let mut audio_client = ptr::null_mut();

        args.mm_device
            .Activate(&IAudioClient3::IID, 0x17, ptr::null(), &mut audio_client)?;
        mem::transmute::<_, IAudioClient3>(audio_client)
    };

    let mut hns_default_device_period = 0;
    unsafe { audio_client.GetDevicePeriod(&mut hns_default_device_period, &mut 0)? };
    println!("hns_default_device_period: {}", hns_default_device_period);

    let wfx = unsafe { audio_client.GetMixFormat()? };
    println!("wfx.nAvgBytesPerSec: {:#?}", unsafe {
        (*wfx).nAvgBytesPerSec
    });

    let h_wake_up = unsafe { CreateWaitableTimerW(ptr::null(), false, None) };
    if h_wake_up == HANDLE(0) {
        return Err(windows::Error::from_win32());
    }
    let _h_wake_up = CloseHandleOnExit { handle: h_wake_up };

    let n_block_align = unsafe { (*wfx).nBlockAlign };

    unsafe {
        audio_client.Initialize(
            AUDCLNT_SHAREMODE_SHARED,
            AUDCLNT_STREAMFLAGS_LOOPBACK,
            0,
            0,
            wfx,
            ptr::null(),
        )?
    };

    let audio_capture_client = unsafe {
        let mut audio_capture_client = ptr::null_mut();

        audio_client.GetService(&IAudioCaptureClient::IID, &mut audio_capture_client)?;
        mem::transmute::<_, IAudioCaptureClient>(audio_capture_client)
    };

    // TODO: AvSetMmThreadCharacteristics を呼ぶか work queue を使うようにする(非オーディオサブシステムによる干渉のムラをなくす？)

    let b_ok = unsafe {
        SetWaitableTimer(
            h_wake_up,
            &(-hns_default_device_period / 2),
            hns_default_device_period as i32 / 2 / (10 * 1000), // per 0.5s
            None,
            ptr::null(),
            false,
        )
    };
    if !b_ok.as_bool() {
        return Err(windows::Error::from_win32());
    }
    let _cancel_timer = CancelWaitableTimerOnExit { handle: h_wake_up };

    unsafe { audio_client.Start()? };

    if let Err(e) = tx.send(CaptureEvent::Start) {
        return Err(message_to_windows_error(&format!(
            "send start error. {:#?}",
            e
        )));
    }
    Ok(0)
}
