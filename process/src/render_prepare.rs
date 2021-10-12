use rustfft::{num_complex::Complex32, FftPlanner};
use std::sync::{mpsc::Receiver, Arc, Mutex};

use super::render::RenderQueue;
use super::utils::{MAX_TARGET_FREQ_INDEX, MIN_TARGET_FREQ_INDEX, WINDOW_SIZE};

pub fn render_prepare_thread_func(
    fft_receiver: Receiver<(usize, usize, Vec<Complex32>)>,
    render_queue: Arc<Mutex<RenderQueue>>,
) {
    let mut last_ifft_index = 0;
    let mut buffer = vec![Complex32::new(0.0, 0.0); WINDOW_SIZE];

    let mut fft = FftPlanner::new();
    let planner = fft.plan_fft_inverse(WINDOW_SIZE);

    for (chan, index, fft_result) in fft_receiver {
        // 何らかの方法で iFFT するかどうか決めて、しないなら continue
        // 今は全てiFFTしてる
        if last_ifft_index != 0 && index - last_ifft_index != WINDOW_SIZE {
            continue;
        }
        last_ifft_index = index;

        if fft_result.len() != MAX_TARGET_FREQ_INDEX - MIN_TARGET_FREQ_INDEX + 1 {
            panic!(
                "not match fft_result len!!! expected: {}, actual: {}",
                MAX_TARGET_FREQ_INDEX - MIN_TARGET_FREQ_INDEX + 1,
                fft_result.len()
            );
        }

        for (i, result) in fft_result.iter().enumerate() {
            // ここで位相をずらす
            buffer[i + MIN_TARGET_FREQ_INDEX].re = result.re;
            buffer[i + MIN_TARGET_FREQ_INDEX].im = result.im;
        }

        planner.process(&mut buffer);
        render_queue.lock().unwrap().push(chan, &buffer);
    }
}
