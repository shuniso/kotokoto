//! コトコト音の合成。音源ファイルは持たず、計算で作る。
//! 音色を変えたいときはこの定数を触る。

use std::f32::consts::TAU;

/// 1 打鍵の長さ（秒）
const LEN: f32 = 0.12;
/// 音量 100% のときのピーク
const PEAK: f32 = 0.5;

/// 胴鳴り（「コ」の低い部分）
const BODY_HZ: f32 = 170.0;
const BODY_DECAY: f32 = 0.022;
/// 打った瞬間だけ音程を持ち上げる量
const BODY_DROP: f32 = 0.6;

/// 木を叩いたような中域（「ト」の芯）
const KNOCK_HZ: f32 = 470.0;
const KNOCK_DECAY: f32 = 0.011;
const KNOCK_GAIN: f32 = 0.4;

/// 接触音。こもらせたノイズ
const CLICK_CUTOFF: f32 = 2200.0;
const CLICK_DECAY: f32 = 0.003;
const CLICK_GAIN: f32 = 0.5;

/// 通常キーの音の種類。キーごとに少しずつ高さが違う
const NORMAL: usize = 8;
/// Space / Enter など大きいキーの音程倍率
const BIG_PITCH: f32 = 0.78;

/// キーから音の番号を決める。同じキーはいつも同じ音になる。
pub fn variant(key: u32, big: bool) -> usize {
    if big {
        0
    } else {
        1 + key as usize * 5 % NORMAL
    }
}

/// 全種類の音を作る。添字は `variant` の戻り値。
pub fn bank(rate: u32) -> Vec<Vec<f32>> {
    let mut bank = vec![thock(rate, BIG_PITCH, 1)];
    for i in 0..NORMAL {
        let pitch = 0.92 + 0.16 * i as f32 / (NORMAL - 1) as f32;
        bank.push(thock(rate, pitch, i as u32 + 2));
    }
    bank
}

fn thock(rate: u32, pitch: f32, seed: u32) -> Vec<f32> {
    let sr = rate as f32;
    let n = (LEN * sr) as usize;
    let lp_a = 1.0 - (-TAU * CLICK_CUTOFF / sr).exp();
    let mut rng = seed.wrapping_mul(0x9E37_79B9) | 1;
    let (mut body_ph, mut knock_ph, mut lp) = (0.0f32, 0.0f32, 0.0f32);

    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / sr;

        body_ph += TAU * BODY_HZ * pitch * (1.0 + BODY_DROP * (-t / 0.006).exp()) / sr;
        knock_ph += TAU * KNOCK_HZ * pitch / sr;
        let body = body_ph.sin() * (-t / BODY_DECAY).exp();
        let knock = knock_ph.sin() * (-t / KNOCK_DECAY).exp() * KNOCK_GAIN;

        rng ^= rng << 13;
        rng ^= rng >> 17;
        rng ^= rng << 5;
        let white = rng as f32 / u32::MAX as f32 * 2.0 - 1.0;
        lp += lp_a * (white - lp);
        let click = lp * (-t / CLICK_DECAY).exp() * CLICK_GAIN;

        // 頭と尻のプチノイズ除け
        let edge = (t / 0.0004).min((LEN - t) / 0.01).min(1.0);
        out.push((body + knock + click) * edge);
    }

    let peak = out.iter().fold(f32::MIN_POSITIVE, |m, s| m.max(s.abs()));
    out.iter_mut().for_each(|s| *s *= PEAK / peak);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bank_covers_every_variant() {
        let bank = bank(48_000);
        for key in 0..256 {
            assert!(variant(key, false) < bank.len());
            assert_ne!(variant(key, false), variant(key, true));
        }
    }

    #[test]
    fn thock_is_normalized_and_ends_silent() {
        for rate in [44_100, 48_000, 96_000, 192_000] {
            for s in bank(rate) {
                assert_eq!(s.len(), (LEN * rate as f32) as usize);
                assert!(s.iter().all(|x| x.is_finite()));
                let peak = s.iter().fold(0.0f32, |m, x| m.max(x.abs()));
                assert!((peak - PEAK).abs() < 1e-4);
                assert!(s[0].abs() < 1e-3 && s[s.len() - 1].abs() < 1e-3);
            }
        }
    }
}
