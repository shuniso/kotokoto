//! kotokoto: キーを打つとコトコト鳴るだけの常駐アプリ。
//! rustyvibes (https://github.com/kunalbagaria/rustyvibes) を Windows 11 向けに最小構成で作り直したもの。

#![cfg_attr(windows, windows_subsystem = "windows")]

mod audio;
mod sound;
#[cfg(windows)]
mod win;

fn main() {
    // 第 1 引数は音量（%）。省略時 100
    let volume = std::env::args()
        .nth(1)
        .and_then(|a| a.parse::<f32>().ok())
        .unwrap_or(100.0)
        .clamp(0.0, 200.0)
        / 100.0;
    let tx = audio::spawn(volume);

    #[cfg(windows)]
    win::run(tx);
    #[cfg(not(windows))]
    demo(tx);
}

/// Windows 以外では音色の確認用に打鍵デモを鳴らすだけ。
#[cfg(not(windows))]
fn demo(tx: std::sync::mpsc::Sender<usize>) {
    use std::{thread::sleep, time::Duration};

    // 大文字と空白の文字コードは仮想キーコードと同じ
    for (i, c) in "KOTOKOTO KOTOKOTO  KOTOKOTO KOTOKOTO ".bytes().enumerate() {
        if let Some(variant) = sound::variant(c as u32) {
            let _ = tx.send(variant);
        }
        sleep(Duration::from_millis(70 + (i as u64 * 37) % 90));
    }
    sleep(Duration::from_millis(300));
}
