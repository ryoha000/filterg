use super::device::get_default_device;
use super::event::create_event;
use super::utils::message_to_windows_error;
use super::utils::{CoUninitializeOnExit, AUDCLNT_BUFFERFLAGS_SILENT};
use bindings::Windows::Win32::Media::Audio::CoreAudio::IMMDevice;
use bindings::Windows::Win32::Media::Audio::CoreAudio::{
    IAudioClient3, IAudioRenderClient, AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_EVENTCALLBACK,
};
use bindings::Windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
use bindings::Windows::Win32::System::Threading::{WaitForMultipleObjects, WAIT_OBJECT_0};
use rustfft::num_complex::Complex32;
use std::collections::VecDeque;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::{mem, ptr};
use windows::Interface;

struct Args {
    mm_device: IMMDevice,
    queue: Arc<Mutex<RenderQueue>>,
    is_stopped: Arc<AtomicBool>,
    is_silence: Arc<AtomicBool>,
}

pub struct RenderQueue {
    queue: Vec<VecDeque<f32>>,
}

impl RenderQueue {
    pub fn new(n_chan: u16) -> RenderQueue {
        let mut queue = Vec::new();
        for _ in 0..n_chan {
            queue.push(VecDeque::new());
        }
        RenderQueue { queue }
    }

    pub fn push(&mut self, n_chan: usize, pcm: &[Complex32]) {
        for sample in pcm {
            self.queue[n_chan].push_back(sample.re);
        }
    }

    pub fn read(&mut self, n_chan: usize) -> Option<f32> {
        self.queue[n_chan].pop_front()
    }
}

pub fn render_thread_func(
    queue: Arc<Mutex<RenderQueue>>,
    is_stopped: Arc<AtomicBool>,
    is_silence: Arc<AtomicBool>,
) -> windows::Result<u8> {
    unsafe { CoInitializeEx(ptr::null_mut(), COINIT_MULTITHREADED)? };
    let _com = CoUninitializeOnExit {};

    // args を作成。TODO: 入力を受け取るようにする

    let default_device = get_default_device()?;

    let args = Args {
        mm_device: default_device,
        queue,
        is_stopped,
        is_silence,
    };

    println!("setup args. ");

    render(args)?;
    Ok(0)
}

fn render(args: Args) -> windows::Result<u8> {
    // TODO: https://docs.microsoft.com/en-us/windows-hardware/drivers/audio/low-latency-audio#windows-audio-session-api-wasapi
    let audio_client: IAudioClient3 = unsafe {
        let mut audio_client = ptr::null_mut();

        args.mm_device
            .Activate(&IAudioClient3::IID, 0x17, ptr::null(), &mut audio_client)?;
        mem::transmute::<_, IAudioClient3>(audio_client)
    };

    let wfx = unsafe { audio_client.GetMixFormat()? };

    unsafe {
        audio_client.Initialize(
            AUDCLNT_SHAREMODE_SHARED,
            AUDCLNT_STREAMFLAGS_EVENTCALLBACK,
            0,
            0,
            wfx,
            ptr::null(),
        )?
    };

    let frames_in_buffer = unsafe { audio_client.GetBufferSize()? };

    let audio_render_client = unsafe {
        let mut audio_render_client = ptr::null_mut();

        audio_client.GetService(&IAudioRenderClient::IID, &mut audio_render_client)?;
        mem::transmute::<_, IAudioRenderClient>(audio_render_client)
    };

    let h_feed_me = create_event()?;

    unsafe {
        audio_client.SetEventHandle(h_feed_me)?;
    };

    let _data = unsafe { audio_render_client.GetBuffer(frames_in_buffer)? };

    unsafe { audio_render_client.ReleaseBuffer(frames_in_buffer, AUDCLNT_BUFFERFLAGS_SILENT)? };

    // TODO: AvSetMmThreadCharacteristics を呼ぶか work queue を使うようにする(非オーディオサブシステムによる干渉のムラをなくす？)

    // let task = unsafe {
    //     winapi::um::avrt::AvSetMmThreadCharacteristicsW(
    //         OsStr::new("Audio")
    //             .encode_wide()
    //             .chain(std::iter::once(0))
    //             .collect::<Vec<_>>()
    //             .as_ptr(),
    //         &mut 0,
    //     )
    // };
    // println!("task.isnull: {}", task.is_null());
    // if task.is_null() {
    //     println!("{:#?}", unsafe { GetLastError() })
    // }
    // let _task = AvRevertMmThreadCharacteristicsOnExit { h: task };

    unsafe { audio_client.Start()? };

    let mut is_done = false;
    let mut passes = 0;
    while !is_done {
        // event をまつ
        let wait_result = unsafe { WaitForMultipleObjects(1, &h_feed_me, false, u32::MAX) };
        if wait_result != WAIT_OBJECT_0 {
            return Err(message_to_windows_error(&format!(
                "Unexpected WaitForMultipleObjects return value {:#?} on pass {}",
                wait_result, passes
            )));
        }

        let frames_of_padding = unsafe { audio_client.GetCurrentPadding()? };

        if frames_of_padding == frames_in_buffer {
            return Err(message_to_windows_error(&
                "Got \"feed me\" event but IAudioClient::GetCurrentPadding reports buffer is full - glitch?"
            ));
        }

        let mut data =
            unsafe { audio_render_client.GetBuffer(frames_in_buffer - frames_of_padding)? };

        // TODO: data に値を入れる(float32)

        // TODO: data に値を入れたら AUDCLNT_BUFFERFLAGS_SILENT を 0 にする
        unsafe { audio_render_client.ReleaseBuffer(frames_in_buffer, AUDCLNT_BUFFERFLAGS_SILENT)? };

        // main thread から stop event が来たかどうか
        let is_stopped = args.is_stopped.load(std::sync::atomic::Ordering::SeqCst);
        if is_stopped {
            is_done = true;
        }
        passes += 1;
    }

    // TODO: ここに終了処理

    Ok(0)
}
