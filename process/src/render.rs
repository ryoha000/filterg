use super::device::get_default_device;
use super::event::create_event;
use super::utils::{
    message_to_windows_error, AudioClientStopOnExit, CoUninitializeOnExit,
    AUDCLNT_BUFFERFLAGS_SILENT,
};
use bindings::Windows::Win32::Media::Audio::CoreAudio::IMMDevice;
use bindings::Windows::Win32::Media::Audio::CoreAudio::{
    IAudioClient3, IAudioRenderClient, AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_EVENTCALLBACK,
};
use bindings::Windows::Win32::Media::Multimedia::WAVE_FORMAT_IEEE_FLOAT;
use bindings::Windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
use bindings::Windows::Win32::System::Threading::{WaitForMultipleObjects, WAIT_OBJECT_0};
use rustfft::num_complex::Complex32;
use std::collections::VecDeque;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::{Arc, Mutex};
use std::{mem, ptr};
use windows::Interface;

struct Args {
    mm_device: IMMDevice,
    queue: Arc<Mutex<RenderQueue>>,
    is_stopped: Arc<AtomicBool>,
    is_silence: Arc<AtomicBool>,
}

struct CosGenerator {
    time: f64,
    freq: f64,
    delta_t: f64,
    amplitude: f64,
    angle: f64
}

impl CosGenerator {
    fn new(freq: f64, fs: f64, amplitude: f64, angle: f64) -> Self {
        CosGenerator {
            time: 0.0,
            freq,
            delta_t: 1.0 / fs,
            amplitude,
            angle
        }
    }
    fn next(&mut self) -> f32 {
        let output = ((self.freq * self.time * std::f64::consts::PI * 2. + self.angle).cos() * self.amplitude) as f32;
        self.time += self.delta_t;
        output
    }
    fn update(&mut self, amplitude: f64, angle: f64) {
        self.time = 0.0;
        self.amplitude = amplitude;
        self.angle = angle;
    }
}

pub struct RenderQueue {
    queue: Vec<VecDeque<f32>>,
    generators: Vec<CosGenerator>
}

impl RenderQueue {
    pub fn new(n_chan: u16) -> RenderQueue {
        let mut queue = Vec::new();
        for _ in 0..n_chan {
            // ↓は10Hzの音を再生するサンプル
            // let mut v = VecDeque::new();
            // for _ in 0..3 {
            //     for t in (0..48000).map(|x| x as f32 / 48000.0) {
            //         let sample = (t * 1000.0 * 2.0 * std::f32::consts::PI).sin();
            //         let amplitude = 0.8;
            //         v.push_back(sample * amplitude);
            //     }
            // }
            // queue.push(v);
            queue.push(VecDeque::new());
        }
        let mut generators = Vec::new();
        for _ in 0..n_chan {
            generators.push(CosGenerator::new(1.0, 44100.0, 0.0, 0.0));
        }
        RenderQueue {
            queue,
            generators,
        }
    }

    pub fn next(&mut self, n_chan: usize) -> f32 {
        self.generators[n_chan].next()
    }

    pub fn update(&mut self, n_chan: usize, amplitude: f32, angle: f32) {
        self.generators[n_chan].update(amplitude as f64, angle as f64)
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

    println!("render: setup args");

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
    let _audio_client = AudioClientStopOnExit {
        client: &audio_client,
    };

    let wfx = unsafe { audio_client.GetMixFormat()? };

    unsafe { (*wfx).wFormatTag = WAVE_FORMAT_IEEE_FLOAT as u16 };
    unsafe { (*wfx).cbSize = 0 };

    let blockalign = unsafe { (*wfx).nBlockAlign };
    let channel_count = unsafe { (*wfx).nChannels };

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
        let available_frames = frames_in_buffer - frames_of_padding;

        if available_frames == 0 {
            println!("[ERROR?] Got \"feed me\" event but IAudioClient::GetCurrentPadding reports buffer is full - glitch?");
            continue;
            // return Err(message_to_windows_error(&
            //     "Got \"feed me\" event but IAudioClient::GetCurrentPadding reports buffer is full - glitch?"
            // ));
        }

        let data = unsafe { audio_render_client.GetBuffer(available_frames)? };

        // TODO: data に値を入れる(float32)
        let mut q = args.queue.lock().unwrap();
        let data_slice = unsafe {
            std::slice::from_raw_parts_mut(
                data,
                (available_frames * (*wfx).nBlockAlign as u32) as usize,
            )
        };

        let mut is_exist_sample = false;
        for frame in data_slice.chunks_exact_mut(blockalign as usize) {
            for (channel_index, value) in frame
                .chunks_exact_mut((blockalign / channel_count) as usize)
                .enumerate()
            {
                let sample_option = q.read(channel_index);
                if let Some(sample) = sample_option {
                    let sample_bytes = sample.to_le_bytes();
                    for (bufbyte, Cosbyte) in value.iter_mut().zip(sample_bytes.iter()) {
                        *bufbyte = *Cosbyte;
                    }
                    is_exist_sample = true;
                }
            }
        }

        let flag = if !is_exist_sample || args.is_silence.load(SeqCst) {
            AUDCLNT_BUFFERFLAGS_SILENT
        } else {
            0
        };

        // TODO: data に値を入れたら AUDCLNT_BUFFERFLAGS_SILENT を 0 にする
        unsafe { audio_render_client.ReleaseBuffer(available_frames, flag)? };

        // main thread から stop event が来たかどうか
        let is_stopped = args.is_stopped.load(SeqCst);
        if is_stopped {
            is_done = true;
        }
        passes += 1;
    }

    // TODO: ここに終了処理

    Ok(0)
}
