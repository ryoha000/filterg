use std::{
    collections::VecDeque,
    sync::{atomic::AtomicBool, mpsc::Receiver, Arc},
    thread, time,
};

use rustfft::{num_complex::Complex32, FftPlanner};

use plotters::prelude::*;

pub struct FftQueue {
    next_chan: usize,
    queue: Vec<VecDeque<f32>>,
}

impl FftQueue {
    pub fn new(n_chan: usize) -> FftQueue {
        let mut queue = Vec::new();
        for _ in 0..n_chan {
            queue.push(VecDeque::new());
        }
        FftQueue {
            queue,
            next_chan: 0,
        }
    }

    pub fn push(&mut self, sample: f32) {
        self.queue[self.next_chan].push_back(sample);

        self.next_chan += 1;
        if self.next_chan >= self.get_n_chan() {
            self.next_chan = 0;
        }
    }

    pub fn read(&mut self, n_chan: usize) -> Option<f32> {
        self.queue[n_chan].pop_front()
    }

    pub fn get_n_chan(&self) -> usize {
        self.queue.len()
    }

    pub fn get_queue_min_size(&self) -> usize {
        let length = self.queue.len();
        let mut res: usize = usize::MAX;
        for i in 0..length {
            res = res.min(self.queue[i].len());
        }
        res
    }
}

/// sender が drop されるまで終わらない
pub fn fft_thread_func(receiver: Receiver<f32>) -> Result<(), Box<dyn std::error::Error>> {
    let fs = 48000;
    let target_freq = 10;
    let chan_count = 2;

    let mut planner = FftPlanner::new();
    // TODO: WaveFormat を受け取る
    let mut queue = FftQueue::new(chan_count);
    let window_size = fs / 1000 * 1000; // 1000ms
    let hop_size = fs / 1000 * 1; // 1ms

    // 初回は window_size 分だけ queue につめる
    let mut is_end = queueing_from_recv(&mut queue, &receiver, window_size);

    while !is_end {
        // ここでフーリエ変換

        // hop_size 分だけ queue を詰め替え
        is_end = queueing_from_recv(&mut queue, &receiver, hop_size);
    }

    let fft = planner.plan_fft_forward(window_size);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("back to the future")
        .as_nanos();

    for chan in 0..queue.get_n_chan() {
        let mut buffer = Vec::<Complex32>::new();
        for _ in 0..window_size {
            buffer.push(Complex32::new(queue.read(chan).unwrap(), 0.0));
        }
        fft.process(&mut buffer);
        plot(buffer, format!("chan-{}-{}", chan, now));
    }

    Ok(())
}

/// 返り値は sender が drop されたかどうか。既定の数だけ sample が送られるのを**待つ**
fn queueing_from_recv(queue: &mut FftQueue, receiver: &Receiver<f32>, frame_count: usize) -> bool {
    if frame_count == 0 {
        return false;
    }

    let goal_count = frame_count * queue.get_n_chan();
    let mut now_count = 0;

    for sample in receiver {
        queue.push(sample);
        now_count += 1;
        if now_count >= goal_count {
            return false;
        }
    }

    return true;
}

pub fn aaa() {
    let freq = 48000;
    let window_size = freq / 1000 * 5; // 5ms
    let hop_size = freq / 1000 * 1; // 1ms

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(1);
    let mut buffer = vec![Complex32::new(0.0, 0.0); window_size];
    fft.process(&mut buffer);
}

pub fn kakko_kari(pcms: Vec<Vec<f32>>) {
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(pcms[0].len());
    for (chan, pcm) in pcms.iter().enumerate() {
        let mut buffer = pcm
            .into_iter()
            .map(move |v| Complex32::new(*v, 0.0))
            .collect::<Vec<Complex32>>();
        fft.process(&mut buffer);

        plot(buffer, format!("{}", chan));
    }
}

// debug 用の関数。plot-${chan}.png に fft の結果を plot する
fn plot(buffer: Vec<Complex32>, title_suffix: String) {
    let x_freq = (0..buffer.len()).collect::<Vec<usize>>();
    let y_db = buffer
        .iter()
        .map(|v| 20.0 * (v.norm_sqr() / (buffer.len() as f32).sqrt()).log10())
        .collect::<Vec<f32>>();

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
