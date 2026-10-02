//! コトコト音の合成。音源ファイルは持たず、計算で作る。
//!
//! ノイズで箱を叩き、不規則に並んだ共鳴をいくつも鳴らす。打鍵のたびに共鳴の高さと強さを
//! 少し揺らすので、毎回まったく同じ音にはならない。音色の数値は設定ファイルで変えられる。

use std::f32::consts::{PI, TAU};

/// 音量 100% のときのピーク
const PEAK: f32 = 0.5;
/// 1 回の当たりの長さ（秒）
const LEN: f32 = 0.08;
/// 指が触れてから底に当たるまでの間隔（秒）
const TOUCH_GAP: f32 = 0.008;

/// 通常キーの高さの種類。キーごとに少しずつ高さが違う
const NORMAL: u32 = 8;
/// Space / Enter など大きいキーの音程倍率
const BIG_PITCH: f32 = 0.78;

/// 音色。各項目の意味は kotokoto.txt を参照。
#[derive(Clone, Debug, PartialEq)]
pub struct Tone {
    pub f0: f32,
    pub modes: usize,
    pub spread: f32,
    pub loss: f32,
    pub contact: f32,
    pub click_ms: f32,
    pub click: f32,
    pub click_hz: f32,
    pub touch: f32,
    pub release: f32,
}

impl Default for Tone {
    fn default() -> Self {
        Tone {
            f0: 220.0,
            modes: 10,
            spread: 15.0,
            loss: 0.13,
            contact: 0.16,
            click_ms: 1.5,
            click: 0.6,
            click_hz: 3600.0,
            touch: 0.0,
            release: 0.45,
        }
    }
}

impl Tone {
    /// `f0=220 modes=10 ...` の形式を読んで上書きする。
    /// 書かれていない項目や読めない項目は元のまま。値は鳴らせる範囲に丸める。
    pub fn apply(&mut self, text: &str) {
        for (key, value) in text.split_whitespace().filter_map(|p| p.split_once('=')) {
            let Some(v) = value.parse::<f32>().ok().filter(|v| v.is_finite()) else {
                continue;
            };
            match key {
                "f0" => self.f0 = v.clamp(40.0, 2000.0),
                "modes" => self.modes = (v as usize).clamp(1, 24),
                "spread" => self.spread = v.clamp(1.0, 64.0),
                "loss" => self.loss = v.clamp(0.01, 2.0),
                "contact" => self.contact = v.clamp(0.02, 10.0),
                "click_ms" => self.click_ms = v.clamp(0.1, 20.0),
                "click" => self.click = v.clamp(0.0, 4.0),
                "click_hz" => self.click_hz = v.clamp(200.0, 16_000.0),
                "touch" => self.touch = v.clamp(0.0, 2.0),
                "release" => self.release = v.clamp(0.0, 2.0),
                _ => {}
            }
        }
    }
}

pub struct Rng(u32);

impl Rng {
    pub fn new(seed: u32) -> Self {
        Rng(seed.wrapping_mul(0x9E37_79B9) | 1)
    }

    /// -1.0..1.0
    pub fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0 as f32 / u32::MAX as f32 * 2.0 - 1.0
    }
}

/// 仮想キーコードから音程の倍率を決める。同じキーはいつも同じ高さになる。
/// 鳴らすのは文字キーと Space / Enter / Backspace だけで、それ以外（修飾キー、Tab、Esc、
/// 矢印、ファンクションキーなど）は `None`。
pub fn pitch(vk: u32) -> Option<f32> {
    match vk {
        // Backspace, Enter, Space
        0x08 | 0x0D | 0x20 => Some(BIG_PITCH),
        // 0-9, A-Z, テンキー, 記号キー（日本語配列の ¥ や ろ を含む）
        0x30..=0x39 | 0x41..=0x5A | 0x60..=0x6F | 0xBA..=0xC0 | 0xDB..=0xDF | 0xE2 => {
            Some(0.92 + 0.16 * (vk * 5 % NORMAL) as f32 / (NORMAL - 1) as f32)
        }
        _ => None,
    }
}

/// 押したときの音
pub fn press(tone: &Tone, rate: u32, pitch: f32, rng: &mut Rng) -> Vec<f32> {
    let mut main = strike(tone, rate, pitch, 1.0, 1.0, rng);
    normalize(&mut main, PEAK);
    if tone.touch == 0.0 {
        return main;
    }
    // 指が触れる音: 柔らかく、共鳴は弱い。本体はその少し後に鳴る
    let mut touch = strike(tone, rate, pitch * 1.3, 0.5, 0.3, rng);
    normalize(&mut touch, PEAK * tone.touch);
    let gap = (TOUCH_GAP * (1.0 + 0.25 * rng.next()) * rate as f32) as usize;
    touch.resize(main.len() + gap, 0.0);
    for (o, s) in touch[gap..].iter_mut().zip(&main) {
        *o += s;
    }
    touch
}

/// 離したときの音: 樹脂どうしが当たるので、高く、硬く、共鳴は少なめ
pub fn release(tone: &Tone, rate: u32, pitch: f32, rng: &mut Rng) -> Vec<f32> {
    let mut out = strike(tone, rate, pitch * 1.25, 1.5, 0.6, rng);
    normalize(&mut out, PEAK * tone.release);
    out
}

/// 1 回の当たり。`hard` は接触の硬さの倍率（大きいほど明るい）、`body` は共鳴の量。
fn strike(tone: &Tone, rate: u32, pitch: f32, hard: f32, body: f32, rng: &mut Rng) -> Vec<f32> {
    let sr = rate as f32;
    let n = (LEN * sr) as usize;
    let contact = tone.contact / 1000.0 / hard;

    // 叩く力。接触時間より上の高さはほとんど含まないノイズ
    let lp_a = 1.0 - (-TAU * (1.0 / contact).min(sr * 0.45) / sr).exp();
    let mut lp = 0.0f32;
    let mut exciter: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f32 / sr;
            lp += lp_a * (rng.next() - lp);
            lp * (-t / (tone.click_ms / 1000.0)).exp() * (t / 0.0001).min(1.0)
        })
        .collect();
    normalize(&mut exciter, 1.0);

    let mut out = vec![0.0f32; n];

    // 共鳴の並びは音色ごとに固定（その「キーボード」の個性）
    let mut layout = Rng::new(tone.f0.to_bits() ^ tone.modes as u32);
    for k in 0..tone.modes {
        let pos = k as f32 / (tone.modes - 1).max(1) as f32;
        let base = tone.f0 * tone.spread.powf(pos) * (1.0 + 0.12 * layout.next());
        let level = 0.75 + 0.25 * layout.next();

        let hz = base * pitch * (1.0 + 0.015 * rng.next());
        if hz > sr * 0.45 {
            continue;
        }
        // 高い共鳴ほど速く消え、接触が長いほど高い共鳴は鳴らない
        let decay = (1.0 / (PI * hz * tone.loss)).min(0.03);
        let weight = level / (1.0 + (hz * contact).powi(2)) * (1.0 + 0.3 * rng.next());
        let mut mode = resonate(&exciter, hz, decay, sr);
        normalize(&mut mode, weight * body);
        for (o, m) in out.iter_mut().zip(&mode) {
            *o += m;
        }
    }

    // 接触音。狭い帯域のノイズ
    let click_hz = (tone.click_hz * (pitch * hard).sqrt()).min(sr * 0.45);
    let mut click = resonate(&exciter, click_hz, 0.0006, sr);
    normalize(&mut click, tone.click);
    for (o, c) in out.iter_mut().zip(&click) {
        *o += c;
    }

    // 尻のプチノイズ除け
    for (i, o) in out.iter_mut().enumerate() {
        *o *= ((n - i) as f32 / (0.008 * sr)).min(1.0);
    }
    out
}

/// 2 極の共鳴器に通す
fn resonate(input: &[f32], hz: f32, decay: f32, sr: f32) -> Vec<f32> {
    let r = (-1.0 / (decay * sr)).exp();
    let (a1, a2) = (2.0 * r * (TAU * hz / sr).cos(), -r * r);
    let (mut y1, mut y2) = (0.0f32, 0.0f32);
    input
        .iter()
        .map(|x| {
            let y = x + a1 * y1 + a2 * y2;
            y2 = y1;
            y1 = y;
            y
        })
        .collect()
}

fn normalize(buf: &mut [f32], peak: f32) {
    let max = buf.iter().fold(f32::MIN_POSITIVE, |m, s| m.max(s.abs()));
    buf.iter_mut().for_each(|s| *s *= peak / max);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peak(buf: &[f32]) -> f32 {
        buf.iter().fold(0.0f32, |m, x| m.max(x.abs()))
    }

    #[test]
    fn only_typing_keys_sound() {
        // Backspace, Enter, Space は低い
        for vk in [0x08, 0x0D, 0x20] {
            assert_eq!(pitch(vk), Some(BIG_PITCH));
        }
        // A, Z, 0, 9, テンキー 5, ; , / , ¥ , ろ
        for vk in [0x41, 0x5A, 0x30, 0x39, 0x65, 0xBA, 0xBF, 0xDC, 0xE2] {
            assert!(
                pitch(vk).is_some_and(|p| (0.9..1.1).contains(&p)),
                "{vk:#x}"
            );
        }
        // Tab, Shift, Ctrl, Alt, CapsLock, Esc, 変換, 矢印, Delete, Win, F1, 左Shift, 左Ctrl, 半角/全角
        for vk in [
            0x09, 0x10, 0x11, 0x12, 0x14, 0x1B, 0x1C, 0x25, 0x26, 0x2E, 0x5B, 0x70, 0xA0, 0xA2,
            0xF3, 0xF4,
        ] {
            assert_eq!(pitch(vk), None, "{vk:#x}");
        }
    }

    #[test]
    fn press_is_normalized_and_ends_silent() {
        let tone = Tone::default();
        for rate in [44_100, 48_000, 96_000, 192_000] {
            for pitch in [BIG_PITCH, 0.92, 1.08] {
                let s = press(&tone, rate, pitch, &mut Rng::new(1));
                assert_eq!(s.len(), (LEN * rate as f32) as usize);
                assert!(s.iter().all(|x| x.is_finite()));
                assert!((peak(&s) - PEAK).abs() < 1e-4);
                assert!(s[0].abs() < 0.02 && s[s.len() - 1].abs() < 1e-3);
            }
        }
    }

    #[test]
    fn release_and_touch_follow_their_levels() {
        let mut tone = Tone::default();
        tone.apply("touch=0.2 release=0.5");
        let r = release(&tone, 48_000, 1.0, &mut Rng::new(1));
        assert!((peak(&r) - PEAK * 0.5).abs() < 1e-4);

        // 触れる音があると、そのぶん長くなり、本体は後ろにずれる
        let p = press(&tone, 48_000, 1.0, &mut Rng::new(1));
        assert!(p.len() > (LEN * 48_000.0) as usize);
        assert!(p.iter().all(|x| x.is_finite()));
        assert!(peak(&p[..200]) <= PEAK * 0.2 + 1e-4);
    }

    #[test]
    fn every_hit_differs_slightly() {
        let tone = Tone::default();
        let mut rng = Rng::new(1);
        let a = press(&tone, 48_000, 1.0, &mut rng);
        let b = press(&tone, 48_000, 1.0, &mut rng);
        assert_ne!(a, b);
    }

    #[test]
    fn apply_overrides_and_clamps() {
        let mut tone = Tone::default();
        tone.apply("f0=300 modes=1 loss=0 release=abc unknown=1 click=NaN spread");
        assert_eq!(tone.f0, 300.0);
        assert_eq!(tone.modes, 1);
        assert_eq!(tone.loss, 0.01);
        assert_eq!(tone.release, Tone::default().release);
        assert_eq!(tone.click, Tone::default().click);

        // 極端な値でも壊れた音にならない
        tone.apply("f0=2000 modes=24 spread=64 contact=0.02 click_hz=16000 click=0");
        let s = press(&tone, 44_100, 1.08, &mut Rng::new(3));
        assert!(s.iter().all(|x| x.is_finite()));
        assert!(peak(&s) <= PEAK + 1e-4);
    }
}
