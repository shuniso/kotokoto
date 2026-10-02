# コトコト (kotokoto)

キーを打つとコトコト鳴るだけの Windows 11 常駐アプリ。
[rustyvibes](https://github.com/kunalbagaria/rustyvibes) を最小構成で作り直したもの。

- 音は 1 種類。サウンドパックは読まず、音は起動時に計算で作る（`kotokoto.exe` 単体で動く）
- 鳴るのは文字キーと Space / Enter / Backspace だけ。修飾キー、Tab、Esc、矢印などは鳴らない
- 鳴るのは押したときだけ。押しっぱなしのリピートでは鳴らない
- キーごとに少しだけ高さが違い、Space / Enter / Backspace は低い
- ネットワークは使わず、打鍵内容も保存しない

## 使い方

`kotokoto.exe` を起動するとタスクトレイに常駐する。終了はトレイアイコンをクリックして「終了」。

音量は第 1 引数で指定する（%、0〜200、既定 100）。

```
kotokoto.exe 60
```

ログイン時に起動したいときは、`Win + R` → `shell:startup` で開くフォルダーにショートカットを置く。
音量を変える場合はショートカットの「リンク先」の末尾に数字を足す。

管理者権限で動いているウィンドウでの打鍵は、kotokoto も管理者権限で起動しないと鳴らない。

## ビルド

Windows 上で [Rust](https://rustup.rs/) を入れて:

```
cargo build --release
```

`target\release\kotokoto.exe` ができる。GitHub に push すれば Actions（`.github/workflows/build.yml`）でも同じ exe が artifact として取れる。

## 音色を変える

`src/sound.rs` 冒頭の定数を触る。Windows 以外で `cargo run` すると打鍵デモが鳴るので、音だけ先に確認できる。

## 構成

| ファイル | 役割 |
| --- | --- |
| `src/sound.rs` | コトコト音の合成 |
| `src/audio.rs` | 再生（打鍵を重ねて鳴らす。無音が続くと出力を閉じる） |
| `src/win.rs` | グローバルキーフックとトレイアイコン |

## ライセンス

[MIT](LICENSE)
