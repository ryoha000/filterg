use std::{collections::VecDeque, sync::{Arc, RwLock, atomic::AtomicBool, mpsc::{self, Receiver, Sender}}, thread, time};

use rustfft::{Fft, FftPlanner, num_complex::Complex32};

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

enum ProcessEvent {
    End
}

enum QueueingEvent {
    Setup
}

const FS: usize = 48000;
const WINDOW_SIZE: usize = FS / 1000 * 1000; // 1000ms
const HOP_SIZE: usize = FS / 1000 * 1; // 1ms

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

/// sender が drop されるまで終わらない
pub fn fft_scheduler_thread_func(receiver: Receiver<f32>) -> Result<(), Box<dyn std::error::Error>> {
    // TODO: WaveFormat を受け取る
    let chan_count = 2;

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(WINDOW_SIZE);
    let mut queue = Arc::new(RwLock::new(FftQueue::new(chan_count)));

    let queueing_queue_clone = queue.clone();
    let (tx_queueing, rx_queueing) = mpsc::channel::<QueueingEvent>();
    // TODO: QueueingEvent の channel をわたす
    thread::spawn(move || queueing_thread_func(queueing_queue_clone, receiver, tx_queueing));

    // 実際にFFTを実行するスレッドを建てる
    let (tx_process_event, rx_process_event) = mpsc::channel::<(i32, ProcessEvent)>();
    let mut process_channels = Vec::new();
    // TODO: CPU のコア数ぶんだけスレッドをたてるようにする
    for id in 0..8 {
        let queue_clone = queue.clone();
        let fft_clone = fft.clone();
        let tx_process_event_clone = tx_process_event.clone();

        // (chan, index)をわたして、そのチャンネル、そのインデックスからの FFT を実行させる
        let (tx_process_target, rx_process_target) = mpsc::channel::<(usize, usize)>();
        process_channels.push(tx_process_target);

        // TODO: CPU を割り当てる
        thread::spawn(move || fft_process_thread_func(id, fft_clone, queue_clone, tx_process_event_clone, rx_process_target));
    }

    let mut next_index = 0;
    if let Ok(QueueingEvent::Setup) = rx_queueing.recv() {
        println!("fft: queueing setup");
        
    }



    Ok(())
}

fn queueing_thread_func(queue: Arc<RwLock<FftQueue>>, rx: Receiver<f32>, tx: Sender<QueueingEvent>) {
    let mut temp_queue = VecDeque::<f32>::new();
    let mut is_initiallized = false;

    for sample in rx {
        temp_queue.push_back(sample);

        // 初期化していないなら WINDOW_SIZE 分の長さで初期化する
        if !is_initiallized {
            if temp_queue.len() >= WINDOW_SIZE {
                let mut q = queue.write().unwrap();
                while let Some(sample) = temp_queue.pop_front() {
                    q.push(sample);
                }
                is_initiallized = true;
                tx.send(QueueingEvent::Setup).unwrap();
            }
        }

        // 初期化済みなら HOP_SIZE を超えると毎回 push できないかチェックする
        if is_initiallized {
            if HOP_SIZE < temp_queue.len() {
                if let Ok(mut q) = queue.try_write() {
                    while let Some(sample) = temp_queue.pop_front() {
                        q.push(sample);
                    }
                }
            }
        }
    }
    // capture_thread の tx が drop されると終了する
}

fn fft_process_thread_func(
    id: i32,
    planner: Arc<dyn Fft<f32>>,
    queue: Arc<RwLock<FftQueue>>,
    tx: Sender<(i32, ProcessEvent)>,
    rx: Receiver<(usize, usize)>,
) {
    let mut buffer = vec![Complex32::new(0.0, 0.0); WINDOW_SIZE];

    for (chan, index) in rx {
        let q = queue.read().unwrap();
        // q.set_buffer(&mut buffer, chan, index, WINDOW_SIZE);
        // 明示的に read lock を外す
        drop(q);

        planner.process(&mut buffer);

        // TODO: ここで FFT の結果に対する処理をする

        tx.send((id, ProcessEvent::End)).unwrap();
    }
    // fft_thread の tx が drop されると終了する
}
