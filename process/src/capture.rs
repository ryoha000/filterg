use super::device::get_default_device;
use super::event::create_event;
use super::file::open_file;
use super::utils::{CloseHandleOnExit, CoUninitializeOnExit};
use bindings::Windows::Win32::Media::Multimedia::HMMIO;
use bindings::Windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
use bindings::Windows::Win32::{Foundation::HANDLE, Media::Audio::CoreAudio::IMMDevice};
use std::ptr;

pub struct Args {
    pub hr: windows::HRESULT,
    pub mm_device: IMMDevice,
    pub b_int16: bool,
    pub h_file: HMMIO,
    pub h_started_event: HANDLE,
    pub h_stop_event: HANDLE,
    pub n_frames: u32,
}

pub fn capture() -> windows::Result<u8> {
    unsafe { CoInitializeEx(ptr::null_mut(), COINIT_MULTITHREADED)? };
    let _com = CoUninitializeOnExit {};

    let args = prepare()?;

    println!("capture. ");
    Ok(0)
}

fn prepare() -> windows::Result<Args> {
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

    Ok(args)
}
