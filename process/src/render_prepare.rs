use plotters::prelude::*;
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

    let mut amplitudes = vec![0.0; WINDOW_SIZE];
    let mut angles = vec![0.0; WINDOW_SIZE];

    let mut logvec = vec![];
    let mut count = (0, 0);

    for (chan, index, fft_result) in fft_receiver {
        count.0 += 1;
        // TODO: 何らかの方法で iFFT するかどうか決めて、しないなら continue
        // 今は全てiFFTしてる
        if last_ifft_index != 0 && index != WINDOW_SIZE + last_ifft_index {
            continue;
        }
        last_ifft_index = index;
        count.1 += 1;

        if fft_result.len() != MAX_TARGET_FREQ_INDEX - MIN_TARGET_FREQ_INDEX + 1 {
            panic!(
                "not match fft_result len!!! expected: {}, actual: {}",
                MAX_TARGET_FREQ_INDEX - MIN_TARGET_FREQ_INDEX + 1,
                fft_result.len()
            );
        }

        for (i, result) in fft_result.iter().enumerate() {
            let angle = result.arg() + std::f32::consts::PI;
            let set_complex = result * Complex32::new(f32::cos(angle), f32::sin(angle));
            // TODO: ここで位相をずらす
            buffer[i + MIN_TARGET_FREQ_INDEX].re = set_complex.re;
            buffer[i + MIN_TARGET_FREQ_INDEX].im = set_complex.im;
        }
        logvec.push(fft_result[0].norm());

        planner.process(&mut buffer);
        render_queue.lock().unwrap().push(chan, &buffer);
    }
    println!("render_prepare: count: {:#?}", &count);
    plot(&logvec, "amplitude".to_string());
}

// debug 用の関数。plot-${chan}.png に fft の結果を plot する
fn plot(buffer: &Vec<f32>, title_suffix: String) {
    let x_freq = (0..buffer.len()).collect::<Vec<usize>>();
    let y_db = buffer.iter().map(|v| *v).collect::<Vec<f32>>();

    let image_width = 1080;
    let image_height = 720;
    let filename = format!("plot-{}.png", title_suffix);
    // 描画先を指定。画像出力する場合はBitMapBackend
    let root = BitMapBackend::new(&filename, (image_width, image_height)).into_drawing_area();
    root.fill(&WHITE).unwrap();

    let caption = "Sample Plot";
    let font = ("sans-serif", 20);

    let (y_min, y_max) = y_db
        .iter()
        .fold((0.0 / 0.0, 0.0 / 0.0), |(m, n), v| (v.min(m), v.max(n)));
    let mut chart = ChartBuilder::on(&root)
        .caption(caption, font.into_font()) // キャプションのフォントやサイズ
        .margin(10) // 上下左右全ての余白
        .x_label_area_size(16) // x軸ラベル部分の余白
        .y_label_area_size(42) // y軸ラベル部分の余白
        .build_cartesian_2d(
            // x軸とy軸の数値の範囲を指定する
            *x_freq.first().unwrap()..*x_freq.last().unwrap(), // x軸の範囲
            y_min..y_max,                                      // y軸の範囲
        )
        .unwrap();

    chart.configure_mesh().draw().unwrap();

    // 折れ線グラフの定義＆描画
    let line_series = LineSeries::new(x_freq.iter().zip(y_db.iter()).map(|(x, y)| (*x, *y)), &RED);
    chart.draw_series(line_series).unwrap();
}
