use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed},
        mpsc::{channel, Receiver, RecvTimeoutError, Sender},
        Arc, RwLock,
    },
    thread,
};

use rustfft::{num_complex::Complex32, Fft, FftPlanner};

use plotters::prelude::*;

use crate::utils::{HOP_SIZE, MAX_TARGET_FREQ_INDEX, MIN_TARGET_FREQ_INDEX, WINDOW_SIZE};

use super::utils::get_now_unix_time;

pub struct FftQueue {
    pop_count: usize, // 累計の index でアクセスするため、いくつ pop したか記録しておく
    next_chan: usize, // 次に push するときのチャンネルを持っておく
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
            pop_count: 0,
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
        self.pop_count += 1;
        self.queue[n_chan].pop_front()
    }

    pub fn get_n_chan(&self) -> usize {
        self.queue.len()
    }

    pub fn set_buffer(
        &self,
        buffer: &mut Vec<Complex32>,
        chan: usize,
        start_index: usize,
        window_size: usize,
    ) {
        for i in 0..window_size {
            buffer[i].re = self.queue[chan][i + start_index];
            buffer[i].im = 0.0;
        }
    }
}

enum ProcessEvent {
    End,
    Exit,
}

enum QueueingEvent {
    Setup,
    Enqueue,
}

// debug 用の関数。plot-${chan}.png に fft の結果を plot する
fn plot(buffer: &Vec<Complex32>, title_suffix: String) {
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
pub fn fft_scheduler_thread_func(
    receiver: Receiver<f32>,
    sender: Sender<(usize, usize, Vec<Complex32>)>,
    is_stopped: Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {
    // TODO: WaveFormat を受け取る
    let chan_count = 2;

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(WINDOW_SIZE);
    let queue = Arc::new(RwLock::new(FftQueue::new(chan_count)));

    let total_length = Arc::new(AtomicUsize::new(0));
    let total_length_clone = total_length.clone();
    let queueing_queue_clone = queue.clone();
    let (tx_queueing, rx_queueing) = channel::<QueueingEvent>();
    // TODO: QueueingEvent の channel をわたす
    let queueing_thread = thread::spawn(move || {
        queueing_thread_func(
            queueing_queue_clone,
            total_length_clone,
            receiver,
            tx_queueing,
        )
    });

    // 実際にFFTを実行するスレッドを建てる
    let (tx_process_event, rx_process_event) = channel::<(usize, ProcessEvent)>();
    let mut process_channels = Vec::new();
    let mut process_threads = Vec::new();
    // TODO: CPU のコア数ぶんだけスレッドをたてるようにする
    for id in 0..8 {
        let queue_clone = queue.clone();
        let fft_clone = fft.clone();
        let tx_process_event_clone = tx_process_event.clone();

        // (chan, index)をわたして、そのチャンネル、そのインデックスからの FFT を実行させる
        let (tx_process_target, rx_process_target) = channel::<(usize, usize)>();
        process_channels.push(tx_process_target);

        let sender_clone = sender.clone();

        // TODO: CPU を割り当てる
        process_threads.push(thread::spawn(move || {
            fft_process_thread_func(
                id,
                fft_clone,
                queue_clone,
                tx_process_event_clone,
                sender_clone,
                rx_process_target,
            )
        }));
    }

    let mut next_chan = 0;
    let mut next_index = 0;
    for (id, event) in rx_process_event {
        if let ProcessEvent::End = event {
            // もし len が window_size より大きいなら process を開始させる
            if total_length.load(Relaxed) >= WINDOW_SIZE + next_index {
                process_channels[id].send((next_chan, next_index)).unwrap();

                next_chan += 1;
                if next_chan >= chan_count {
                    next_chan = 0;
                    next_index += HOP_SIZE;
                }
            }
        }
        if is_stopped.load(Relaxed) {
            println!("end. total_length: {}", total_length.load(Relaxed));

            drop(tx_process_event);
            for tx in process_channels {
                drop(tx);
            }
            queueing_thread.join().unwrap();
            for th in process_threads {
                th.join().unwrap();
            }
            break;
        }
    }

    Ok(())
}

fn queueing_thread_func(
    queue: Arc<RwLock<FftQueue>>,
    total_length: Arc<AtomicUsize>,
    rx: Receiver<f32>,
    tx: Sender<QueueingEvent>,
) {
    let mut temp_queue = VecDeque::<f32>::new();
    let mut is_initiallized = false;
    // TODO: ちゃんとする
    let chan_size = 2;

    for sample in rx {
        temp_queue.push_back(sample);

        // 初期化していないなら WINDOW_SIZE 分の長さで初期化する
        if !is_initiallized {
            if temp_queue.len() >= WINDOW_SIZE * chan_size {
                println!("want to get queue lock");
                let mut q = queue.write().unwrap();
                let enqueue_size = temp_queue.len() / chan_size;
                while let Some(sample) = temp_queue.pop_front() {
                    q.push(sample);
                }
                is_initiallized = true;

                total_length.fetch_add(enqueue_size, Relaxed);
                tx.send(QueueingEvent::Setup).unwrap();
            }
        }

        // 初期化済みなら HOP_SIZE を超えると毎回 push できないかチェックする
        if is_initiallized {
            if temp_queue.len() > HOP_SIZE * chan_size {
                if let Ok(mut q) = queue.try_write() {
                    let enqueue_size = temp_queue.len() / chan_size;
                    while let Some(sample) = temp_queue.pop_front() {
                        q.push(sample);
                    }

                    total_length.fetch_add(enqueue_size, Relaxed);
                    tx.send(QueueingEvent::Enqueue).unwrap();
                }
            }
        }
    }
    // capture_thread の tx が drop されると終了する
}

fn fft_process_thread_func(
    id: usize,
    planner: Arc<dyn Fft<f32>>,
    queue: Arc<RwLock<FftQueue>>,
    tx: Sender<(usize, ProcessEvent)>,
    result_sender: Sender<(usize, usize, Vec<Complex32>)>,
    rx: Receiver<(usize, usize)>,
) {
    let mut buffer = vec![Complex32::new(0.0, 0.0); WINDOW_SIZE];

    let mut lock_time = Vec::new();
    let mut fft_time = Vec::new();
    let mut plot_time = Vec::new();

    loop {
        match rx.recv_timeout(std::time::Duration::from_millis(1)) {
            Ok((chan, index)) => {
                let start = get_now_unix_time();

                let q = queue.read().unwrap();

                lock_time.push(get_now_unix_time() - start);

                let start = get_now_unix_time();
                q.set_buffer(&mut buffer, chan, index, WINDOW_SIZE);
                // 明示的に read lock を外す
                drop(q);

                planner.process(&mut buffer);

                fft_time.push(get_now_unix_time() - start);

                // let start = get_now_unix_time();

                // // TODO: ここで FFT の結果に対する処理をする
                let mut send_vec = vec![];
                for i in MIN_TARGET_FREQ_INDEX..MAX_TARGET_FREQ_INDEX + 1 {
                    send_vec.push(buffer[i].clone());
                }
                result_sender.send((chan, index, send_vec));
                // plot(&buffer, format!("{}-{}", chan, index));

                // plot_time.push(get_now_unix_time() - start);

                tx.send((id, ProcessEvent::End)).unwrap();
            }
            Err(RecvTimeoutError::Timeout) => {
                tx.send((id, ProcessEvent::End)).unwrap();
            }
            Err(RecvTimeoutError::Disconnected) => {
                break;
            }
        }
    }
    println!(
        "thread id: {}, lock_time avg: {}, fft_time avg: {}, plot_time avg: {}",
        id,
        lock_time.iter().sum::<u128>()
            / if lock_time.len() == 0 {
                1
            } else {
                lock_time.len() as u128
            },
        fft_time.iter().sum::<u128>()
            / if fft_time.len() == 0 {
                1
            } else {
                fft_time.len() as u128
            },
        plot_time.iter().sum::<u128>()
            / if plot_time.len() == 0 {
                1
            } else {
                plot_time.len() as u128
            },
    )
    // fft_thread の tx が drop されると終了する
}
