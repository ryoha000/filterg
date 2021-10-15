use plotters::prelude::*;
use rustfft::{num_complex::Complex32};
use std::sync::{mpsc::Receiver, Arc, Mutex};

use super::render::RenderQueue;
use super::utils::{WINDOW_SIZE, WINDOW_SIZE_MILLI_SECOND, get_now_milli_unix_time};

pub fn render_prepare_thread_func(
    fft_receiver: Receiver<(usize, usize, Complex32)>,
    render_queue: Arc<Mutex<RenderQueue>>,
) {
    let mut last_check_index = 0;

    let mut last_update_milli_second = 0;
    let buffer_milli_second = 1;
    let mut amplitude = 0.0;
    let mut angle = 0.0;

    // TODO: log 用、消す
    let mut log_amplitude_diff_vec = vec![];
    let mut log_angle_diff_vec = vec![];
    let mut log_original_amplitude_vec = vec![];
    let mut count = (0, 0);

    for (chan, index, fft_result) in fft_receiver {
        count.0 += 1;
        // TODO: 何らかの方法で iFFT するかどうか決めて、しないなら continue
        // 今は全てiFFTしてる
        if last_check_index != 0 && index != WINDOW_SIZE + last_check_index {
            continue;
        }
        last_check_index = index;
        count.1 += 1;

        let now = get_now_milli_unix_time();
        if now <= buffer_milli_second + WINDOW_SIZE_MILLI_SECOND as u128 + last_update_milli_second {
            // 出してるつもりの音か分からないため continue
            continue;
        }

        // 位相と振幅のずれを検出
        let (original_amplitude, original_angle) = diff(&fft_result, amplitude, angle);
        
        // TODO: 消す
        log_amplitude_diff_vec.push(original_amplitude - amplitude);
        log_angle_diff_vec.push(angle - original_angle);
        log_original_amplitude_vec.push(original_amplitude);

        amplitude = original_amplitude;
        let angle_diff = angle - original_angle;
        // angle は pi だけ位相が違うようにフィードバック制御したい
        angle += std::f32::consts::PI - angle_diff;


        render_queue.lock().unwrap().update(chan, amplitude, angle);
        last_update_milli_second = now;
    }
    println!("render_prepare: count: {:#?}", &count);
    plot(&log_amplitude_diff_vec, "log_amplitude_diff_vec".to_string());
    plot(&log_angle_diff_vec, "log_angle_diff_vec".to_string());
    plot(&log_original_amplitude_vec, "log_original_amplitude_vec".to_string());
}

/// 合成後の複素数と自分が加えた振幅、位相を受け取って元の振幅、位相を取得
fn diff(result: &Complex32, add_amplitude: f32, add_angle: f32) -> (f32, f32) {
    let result_amplitude = result.norm();
    let result_angle = result.arg();

    // https://detail.chiebukuro.yahoo.co.jp/qa/question_detail/q13190344118 (自分でも導出済み)
    let sin_diff = result_amplitude * result_angle.sin() - add_amplitude * add_angle.sin();
    let cos_diff = result_amplitude * result_angle.cos() - add_amplitude * add_angle.cos();
    let original_amplitude = (sin_diff.powi(2) + cos_diff.powi(2)).powf(0.5);
    let original_angle = (sin_diff / cos_diff).atan();

    (original_amplitude, original_angle)
}

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
