//! 設定ファイル。exe の隣の kotokoto.txt を読む。

use crate::sound::Tone;

/// 既定の設定。音色の一覧を兼ねる
pub const DEFAULT: &str = include_str!("../kotokoto.txt");

pub struct Config {
    /// 音量（%）
    pub volume: f32,
    pub tone: Tone,
}

impl Config {
    /// 設定の文面を読む。読めない行は無視する。
    pub fn parse(text: &str) -> Config {
        let mut config = Config {
            volume: 60.0,
            tone: Tone::default(),
        };
        config.apply(text);
        config
    }

    /// 既定の設定に、exe の隣の設定ファイルを重ねて読む。
    /// ファイルが無ければ既定の内容で作っておく（一覧から選べるように）。
    #[cfg(windows)]
    pub fn load() -> Config {
        let mut config = Config::parse(DEFAULT);
        let path = std::env::current_exe()
            .ok()
            .and_then(|exe| Some(exe.parent()?.join("kotokoto.txt")));
        if let Some(path) = path {
            match std::fs::read_to_string(&path) {
                Ok(text) => config.apply(&text),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    let _ = std::fs::write(&path, DEFAULT);
                }
                Err(_) => {}
            }
        }
        config
    }

    fn apply(&mut self, text: &str) {
        for line in text.lines() {
            let line = line.trim_start_matches('\u{feff}').trim();
            if line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match key.trim() {
                "volume" => {
                    if let Some(v) = value.trim().parse::<f32>().ok().filter(|v| v.is_finite()) {
                        self.volume = v.clamp(0.0, 200.0);
                    }
                }
                "tone" => self.tone.apply(value),
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_file_selects_exactly_one_tone() {
        let active = DEFAULT
            .lines()
            .filter(|l| l.trim_start().starts_with("tone"))
            .count();
        assert_eq!(active, 1);
        assert_eq!(Config::parse(DEFAULT).volume, 60.0);
    }

    #[test]
    fn last_uncommented_line_wins() {
        let c = Config::parse(
            "\u{feff}# コメント\r\nvolume = 35\r\n#tone = f0=100\r\ntone = f0=150 release=0\r\ntone = f0=180\r\nvolume = x\r\n",
        );
        assert_eq!(c.volume, 35.0);
        assert_eq!(c.tone.f0, 180.0);
        assert_eq!(c.tone.release, 0.0);
    }

    #[test]
    fn volume_is_clamped() {
        assert_eq!(Config::parse("volume = 900").volume, 200.0);
        assert_eq!(Config::parse("volume=-5").volume, 0.0);
    }
}
