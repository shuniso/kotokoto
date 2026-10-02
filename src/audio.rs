//! 再生。専用スレッドが出力ストリームを持ち、打鍵を重ねて鳴らす。

use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{ErrorKind, FromSample, SampleFormat, SizedSample, Stream};

use crate::sound;

/// これだけ打鍵が無ければストリームを閉じる。
/// 開きっぱなしだと Windows の自動スリープを妨げるため。
/// 次の打鍵で開き直すので、既定の出力デバイスの切り替えにもここで追従する。
const IDLE: Duration = Duration::from_secs(10);
const MAX_VOICES: usize = 32;

struct Voice {
    variant: usize,
    pos: usize,
    gain: f32,
}

type Voices = Arc<Mutex<Vec<Voice>>>;

struct Output {
    _stream: Stream,
    voices: Voices,
    broken: Arc<AtomicBool>,
}

/// 再生スレッドを起動する。返り値に `sound::variant` の番号を送ると鳴る。
pub fn spawn(volume: f32) -> Sender<usize> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || run(rx, volume));
    tx
}

fn run(rx: Receiver<usize>, volume: f32) {
    let mut out: Option<Output> = None;
    let mut rng = 0x2545_F491u32;
    loop {
        match rx.recv_timeout(IDLE) {
            Ok(variant) => {
                if out.as_ref().is_none_or(|o| o.broken.load(Relaxed)) {
                    // 古いストリームを先に閉じてから開き直す
                    drop(out.take());
                    out = open();
                    // 開いている間に溜まった打鍵は捨てる（まとめて鳴ってしまうため）
                    while rx.try_recv().is_ok() {}
                }
                let Some(out) = &out else { continue };

                // 打鍵ごとに強さを少し揺らす
                rng ^= rng << 13;
                rng ^= rng >> 17;
                rng ^= rng << 5;
                let gain = volume * (0.8 + 0.2 * (rng >> 8) as f32 / (1 << 24) as f32);

                let mut voices = out.voices.lock().unwrap();
                if voices.len() == MAX_VOICES {
                    voices.remove(0);
                }
                voices.push(Voice { variant, pos: 0, gain });
            }
            Err(RecvTimeoutError::Timeout) => out = None,
            Err(RecvTimeoutError::Disconnected) => return,
        }
    }
}

fn open() -> Option<Output> {
    let device = cpal::default_host().default_output_device()?;
    let config = device.default_output_config().ok()?;
    let voices: Voices = Arc::new(Mutex::new(Vec::with_capacity(MAX_VOICES)));
    let broken = Arc::new(AtomicBool::new(false));

    let (v, b) = (voices.clone(), broken.clone());
    let stream = match config.sample_format() {
        SampleFormat::F32 => build::<f32>(&device, config.into(), v, b),
        SampleFormat::I16 => build::<i16>(&device, config.into(), v, b),
        SampleFormat::U16 => build::<u16>(&device, config.into(), v, b),
        _ => return None,
    }?;
    stream.play().ok()?;
    Some(Output { _stream: stream, voices, broken })
}

fn build<T: SizedSample + FromSample<f32>>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    voices: Voices,
    broken: Arc<AtomicBool>,
) -> Option<Stream> {
    let channels = config.channels as usize;
    let bank = sound::bank(config.sample_rate);
    device
        .build_output_stream(
            config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                let mut voices = voices.lock().unwrap();
                for frame in data.chunks_mut(channels) {
                    let mut mix = 0.0f32;
                    voices.retain_mut(|v| match bank[v.variant].get(v.pos) {
                        Some(s) => {
                            mix += s * v.gain;
                            v.pos += 1;
                            true
                        }
                        None => false,
                    });
                    frame.fill(T::from_sample(mix.clamp(-1.0, 1.0)));
                }
            },
            move |err| {
                // 鳴り続けられる通知は無視し、それ以外は次の打鍵で開き直す
                if !matches!(
                    err.kind(),
                    ErrorKind::DeviceChanged | ErrorKind::RealtimeDenied | ErrorKind::Xrun
                ) {
                    broken.store(true, Relaxed);
                }
            },
            None,
        )
        .ok()
}
