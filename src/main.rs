//! kotokoto: キーを打つとコトコト鳴るだけの常駐アプリ。
//! rustyvibes (https://github.com/kunalbagaria/rustyvibes) を Windows 11 向けに最小構成で作り直したもの。

#![cfg_attr(windows, windows_subsystem = "windows")]

mod audio;
mod config;
#[cfg(not(windows))]
mod dev;
mod sound;
#[cfg(windows)]
mod win;

#[cfg(windows)]
fn main() {
    let config = config::Config::load();
    // 第 1 引数に音量（%）があれば、設定ファイルより優先する
    let volume = std::env::args()
        .nth(1)
        .and_then(|a| a.parse::<f32>().ok())
        .filter(|v| v.is_finite())
        .unwrap_or(config.volume)
        .clamp(0.0, 200.0);
    win::run(audio::spawn(volume / 100.0, config.tone));
}

#[cfg(not(windows))]
fn main() {
    dev::run();
}
