//! 開発用（Windows 以外）。音色を確かめるための道具で、Windows 版には入らない。
//!
//! - `cargo run`            kotokoto.txt で選んでいる音色で打鍵デモを鳴らす
//! - `cargo run -- samples` kotokoto.txt に載っている全音色を samples/*.wav に書き出す

use std::{fs, thread::sleep, time::Duration};

use crate::audio::{self, Hit};
use crate::config::{Config, DEFAULT};
use crate::sound::{self, Rng, Tone};

const RATE: u32 = 48_000;
/// 打鍵パターン。大文字と空白・改行の文字コードは仮想キーコードと同じ
const TEXT: &str = "KOTOKOTO TO OTO GA SURU\r";

pub fn run() {
    match std::env::args().nth(1).as_deref() {
        Some("samples") => samples(),
        _ => demo(),
    }
}

/// 打鍵 1 つぶんの (高さ, 強さ, 押している秒数, 次の打鍵までの秒数)
fn pattern() -> Vec<(f32, f32, f32, f32)> {
    let mut rng = Rng::new(0x2545_F491);
    TEXT.bytes()
        .filter_map(|c| {
            let pitch = sound::pitch(c as u32)?;
            let gap = 0.13 + 0.045 * rng.next() + if c == b' ' { 0.05 } else { 0.0 };
            Some((
                pitch,
                0.9 + 0.1 * rng.next(),
                0.085 + 0.025 * rng.next(),
                gap,
            ))
        })
        .collect()
}

fn demo() {
    let config = Config::parse(DEFAULT);
    let tx = audio::spawn(config.volume / 100.0, config.tone);
    for (pitch, _, held, gap) in pattern() {
        let release = tx.clone();
        std::thread::spawn(move || {
            sleep(Duration::from_secs_f32(held));
            let _ = release.send(Hit {
                pitch,
                release: true,
            });
        });
        let _ = tx.send(Hit {
            pitch,
            release: false,
        });
        sleep(Duration::from_secs_f32(gap));
    }
    sleep(Duration::from_millis(400));
}

fn samples() {
    fs::create_dir_all("samples").unwrap();
    for (name, tone) in presets() {
        let mut mix = vec![0.0f32; RATE as usize * 20];
        let mut add = |at: f32, buf: &[f32], gain: f32| {
            let start = (at * RATE as f32) as usize;
            for (i, s) in buf.iter().enumerate() {
                mix[start + i] += s * gain;
            }
        };

        let mut at = 0.2f32;
        for (n, (pitch, gain, held, gap)) in pattern().into_iter().enumerate() {
            let mut rng = Rng::new(n as u32 + 100);
            add(at, &sound::press(&tone, RATE, pitch, &mut rng), gain);
            if tone.release > 0.0 {
                add(
                    at + held,
                    &sound::release(&tone, RATE, pitch, &mut rng),
                    gain,
                );
            }
            at += gap;
        }
        mix.truncate(((at + 0.4) * RATE as f32) as usize);

        let path = format!("samples/{name}.wav");
        fs::write(&path, wav(&mix)).unwrap();
        println!("{path}");
    }
}

/// 設定ファイルに載っている音色を、コメントアウトされているものも含めて全部取り出す。
/// 名前は直前のコメント行（`# 名前 : 説明`）から取る。
fn presets() -> Vec<(String, Tone)> {
    let mut out = Vec::new();
    let mut name = String::new();
    for line in DEFAULT.lines() {
        let body = line.trim().trim_start_matches('#').trim();
        let value = body
            .strip_prefix("tone")
            .and_then(|rest| rest.trim_start().strip_prefix('='));
        if let Some(value) = value {
            let mut tone = Tone::default();
            tone.apply(value);
            out.push((name.clone(), tone));
        } else if let Some((head, _)) = body.split_once(" : ") {
            name = head.trim().to_string();
        }
    }
    out
}

/// 16 ビット・モノラルの WAV
fn wav(samples: &[f32]) -> Vec<u8> {
    let data_len = samples.len() as u32 * 2;
    let mut b = Vec::with_capacity(44 + data_len as usize);
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data_len).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&RATE.to_le_bytes());
    b.extend_from_slice(&(RATE * 2).to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        b.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
    }
    b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_preset_has_a_unique_name() {
        let presets = presets();
        assert_eq!(presets.len(), 11);
        let mut names: Vec<_> = presets.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.iter().all(|n| !n.is_empty() && !n.contains('/')));
        names.sort();
        names.dedup();
        assert_eq!(names.len(), presets.len());
    }

    #[test]
    fn pattern_covers_the_whole_text() {
        assert_eq!(pattern().len(), TEXT.len());
    }
}
