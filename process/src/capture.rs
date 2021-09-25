use super::device::get_default_device;
use super::event::create_event;
use super::file::{open_file, write_wave_header};
use super::utils::{CloseHandleOnExit, CoUninitializeOnExit};
use bindings::Windows::Win32::Media::Audio::CoreAudio::IAudioClient;
use bindings::Windows::Win32::Media::Multimedia::{HMMIO, MMCKINFO};
use bindings::Windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
use bindings::Windows::Win32::{Foundation::HANDLE, Media::Audio::CoreAudio::IMMDevice};
use std::panic::panic_any;
use std::sync::mpsc::Sender;
use std::{mem, ptr};
use windows::Interface;

pub struct Args {
    pub mm_device: IMMDevice,
    pub h_file: HMMIO,
    pub h_stop_event: HANDLE,
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

pub fn capture_thread_func(tx: Sender<CaptureEvent>) -> windows::Result<u8> {
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
        h_stop_event,
        n_frames: 0,
    };

    println!("stup args. ");

    capture(tx, args)?;
    Ok(0)
}

fn capture(tx: Sender<CaptureEvent>, args: Args) -> windows::Result<u8> {
    let audio_client: IAudioClient = unsafe {
        let mut audio_client = ptr::null_mut();

        args.mm_device
            .Activate(&IAudioClient::IID, 0x17, ptr::null(), &mut audio_client)?;
        mem::transmute::<_, IAudioClient>(audio_client)
    };

    let mut hns_default_device_period = 0;
    unsafe { audio_client.GetDevicePeriod(&mut hns_default_device_period, &mut 0)? };
    println!("hns_default_device_period: {}", hns_default_device_period);

    let wfx = unsafe { audio_client.GetMixFormat()? };
    println!("wfx.nAvgBytesPerSec: {:#?}", unsafe {
        (*wfx).nAvgBytesPerSec
    });

    Ok(0)
}
