use super::device::get_default_device;
use super::fft::FftQueue;
use super::utils::{message_to_windows_error, CancelWaitableTimerOnExit};
use super::utils::{CloseHandleOnExit, CoUninitializeOnExit};
use bindings::Windows::Win32::Media::Audio::CoreAudio::{
    IAudioCaptureClient, IAudioClient3, AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_LOOPBACK,
};
use bindings::Windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
use bindings::Windows::Win32::System::Threading::{
    CreateWaitableTimerW, SetWaitableTimer, WaitForMultipleObjects, WAIT_OBJECT_0,
};
use bindings::Windows::Win32::{Foundation::HANDLE, Media::Audio::CoreAudio::IMMDevice};
use hound::WavSpec;
use std::ffi::OsStr;
use std::os::windows::prelude::OsStrExt;
use std::panic::panic_any;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::{mem, ptr};
use windows::Interface;

pub struct Args {
    pub mm_device: IMMDevice,
    pub is_stopped: Arc<AtomicBool>,
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
    tx_wf: Sender<WavSpec>,
    is_stopped: Arc<AtomicBool>,
    fft_queue: Arc<Mutex<FftQueue>>,
) -> windows::Result<u8> {
    let _defer = DeferChan { tx: tx.clone() };

    unsafe { CoInitializeEx(ptr::null_mut(), COINIT_MULTITHREADED)? };
    let _com = CoUninitializeOnExit {};

    // args を作成。TODO: 入力を受け取るようにする

    let default_device = get_default_device()?;
    println!("default_device.GetId(): {:#?}", unsafe {
        default_device.GetId().unwrap()
    });

    let args = Args {
        mm_device: default_device,
        is_stopped,
    };

    println!("stup args. ");

    capture(tx, tx_wf, fft_queue, args)?;
    Ok(0)
}

fn capture(
    tx: Sender<CaptureEvent>,
    tx_wf: Sender<WavSpec>,
    fft_queue: Arc<Mutex<FftQueue>>,
    args: Args,
) -> windows::Result<u8> {
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
    println!("wfx.nBlockAlign: {:#?}", unsafe { (*wfx).nBlockAlign });

    let block_align = unsafe { (*wfx).nBlockAlign };
    let n_channel = unsafe { (*wfx).nChannels };

    let spec = WavSpec {
        channels: n_channel,
        sample_rate: unsafe { (*wfx).nSamplesPerSec },
        bits_per_sample: unsafe { (*wfx).wBitsPerSample } as u16,
        sample_format: hound::SampleFormat::Float,
    };
    if let Err(e) = tx_wf.send(spec) {
        return Err(message_to_windows_error(&format!(
            "send wave format error. {:#?}",
            e
        )));
    }
    set_fft_queue_size(fft_queue.clone(), n_channel);

    let h_wake_up = unsafe { CreateWaitableTimerW(ptr::null(), false, None) };
    if h_wake_up == HANDLE(0) {
        return Err(windows::Error::from_win32());
    }
    let _h_wake_up = CloseHandleOnExit { handle: h_wake_up };

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

    let b_ok = unsafe {
        SetWaitableTimer(
            h_wake_up,
            &(-hns_default_device_period / 2),
            (hns_default_device_period / 2 / (10 * 1000)) as i32, // hns_default_device_period / 2ms
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

    let mut arr: Vec<Vec<f32>> = vec![vec![], vec![]];

    let mut is_done = false;
    let mut is_first_packet = true;
    let mut passes = 0;
    let mut frames: u64 = 0;
    while !is_done {
        loop {
            let next_packet_size = unsafe { audio_capture_client.GetNextPacketSize()? };
            if next_packet_size <= 0 {
                break;
            }

            let mut data = ptr::null_mut::<u8>();
            let mut num_frames_to_read = 0;
            let mut flags = 0;
            unsafe {
                audio_capture_client.GetBuffer(
                    &mut data,
                    &mut num_frames_to_read,
                    &mut flags,
                    ptr::null_mut(),
                    ptr::null_mut(),
                )?
            }
            // println!("flags: {}", flags);

            // if is_first_packet && AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY == flags {
            //     return Err(message_to_windows_error(
            //         &"Probably spurious glitch reported on first packet",
            //     ));
            // } else if 0 != flags {
            //     return Err(message_to_windows_error(&format!(
            //         "IAudioCaptureClient::GetBuffer set flags to {} on pass {} after {} frames",
            //         flags, passes, frames
            //     )));
            // }

            if 0 == num_frames_to_read {
                return Err(message_to_windows_error(&format!("IAudioCaptureClient::GetBuffer said to read 0 frames on pass {} after {} frames", passes, frames)));
            }

            let mut f = fft_queue.lock().unwrap();
            // TODO: ここに処理
            for data_index in 0..num_frames_to_read {
                for chan in 0..n_channel {
                    unsafe {
                        let sample_u8 = data.offset(
                            (data_index * block_align as u32
                                + (chan * block_align / n_channel) as u32)
                                as isize,
                        ) as *const f32;
                        // TODO: 分岐f32
                        let sample = ptr::read(sample_u8);
                        f.push(chan as usize, sample);
                        // writer.write_sample(sample).unwrap();

                        arr[chan as usize].push(sample);
                    }
                }
            }

            frames += num_frames_to_read as u64;

            unsafe {
                audio_capture_client.ReleaseBuffer(num_frames_to_read)?;
            }

            is_first_packet = false;
        }

        // timer をまつ
        let wait_result = unsafe { WaitForMultipleObjects(1, &h_wake_up, false, u32::MAX) };
        if wait_result != WAIT_OBJECT_0 {
            return Err(message_to_windows_error(&format!(
                "Unexpected WaitForMultipleObjects return value {:?} on pass {} after {} frames",
                wait_result, passes, frames
            )));
        }

        // main thread から stop event が来たかどうか
        let is_stopped = args.is_stopped.load(std::sync::atomic::Ordering::SeqCst);
        if is_stopped {
            is_done = true;
        }
        passes += 1;
    }

    // TODO: ここに終了処理
    // fft::kakko_kari(arr);
    // writer.finalize().unwrap();

    Ok(0)
}

fn set_fft_queue_size(fft_queue: Arc<Mutex<FftQueue>>, n_channel: u16) {
    let mut f = fft_queue.lock().unwrap();
    f.set_size(n_channel as u32);
}
