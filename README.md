# コトコト (kotokoto)

キーを打つとコトコト鳴るだけの Windows 11 常駐アプリ。
[rustyvibes](https://github.com/kunalbagaria/rustyvibes) を最小構成で作り直したもの。

<img src="assets/icon.png" alt="コトコトのグレーの立体キーキャップ" width="96">

- サウンドパックは読まず、音は計算で作る（`kotokoto.exe` 単体で動く）
- 音色は設定ファイルの一覧から選べる
- 鳴るのは文字キーと Space / Enter / Backspace だけ。修飾キー、Tab、Esc、矢印などは鳴らない
- 押したときと離したときに鳴る（離す音は音色による）。押しっぱなしのリピートでは鳴らない
- キーごとに少しだけ高さが違い、Space / Enter / Backspace は低い。同じキーでも打つたびに少し違う
- ネットワークは使わず、打鍵内容も保存しない

## 使い方

`kotokoto.exe` を起動するとタスクトレイに常駐する。終了はトレイアイコンをクリックして「終了」。

ログイン時に起動したいときは、`Win + R` → `shell:startup` で開くフォルダーにショートカットを置く。

管理者権限で動いているウィンドウでの打鍵は、kotokoto も管理者権限で起動しないと鳴らない。

## ビルド

Windows 上で [Rust](https://rustup.rs/) を入れて:

```
cargo build --release
```

`target\release\kotokoto.exe` ができる。GitHub に push すれば Actions（`.github/workflows/build.yml`）でも同じ exe が artifact として取れる。

## 設定（音量と音色）

`kotokoto.exe` と同じフォルダーの `kotokoto.txt` を編集し、アプリを起動し直す。
ファイルが無ければ、初回起動時に既定の内容（[kotokoto.txt](kotokoto.txt)）で作られる。

- **音量**: `volume = 60`（%、0〜200）。`kotokoto.exe 40` のように第 1 引数で渡すと、そちらが優先される
- **音色**: `tone = ...` の行が一覧になっている。使いたい 1 行だけ行頭の `#` を外す。数値を直接書き換えてもよい

それぞれの音色は [samples](samples) フォルダーの同じ名前の WAV で聴ける。

## 音色を作る

`kotokoto.txt` に `tone = ...` の行を足す（直前に `# 名前 : 説明` のコメント行を置く）。Windows 以外では次で確認できる。

```
cargo run              # 選んでいる音色で打鍵デモを鳴らす
cargo run -- samples   # 一覧の全音色を samples/*.wav に書き出す
```

## 構成

| ファイル | 役割 |
| --- | --- |
| `kotokoto.txt` | 既定の設定と音色の一覧（exe に埋め込まれる） |
| `src/sound.rs` | コトコト音の合成 |
| `src/config.rs` | 設定ファイルの読み込み |
| `src/audio.rs` | 再生（打鍵を重ねて鳴らす。無音が続くと出力を閉じる） |
| `src/win.rs` | グローバルキーフックとトレイアイコン |
| `src/dev.rs` | 開発用の打鍵デモとサンプル書き出し（Windows 版には入らない） |
| `samples/` | 各音色のサンプル音源 |
| `assets/icon.svg` | アイコンの編集用原本（グレーの立体キーキャップ） |
| `assets/icon.png` | 1024 × 1024 の透過 PNG |
| `assets/icon.ico` | Windows 用アイコン（16 / 24 / 32 / 48 / 64 / 128 / 256 px） |
| `build.rs` | Windows ビルド時にアイコンを exe に埋め込む |

アイコンは exe とタスクトレイで共通。配布時にアイコンファイルを添付する必要はない。
SVG を編集した場合は [ImageMagick](https://imagemagick.org/) で再生成できる。

```sh
magick -background none assets/icon.svg -resize 1024x1024 assets/icon.png
magick assets/icon.png -define icon:auto-resize=256,128,64,48,32,24,16 assets/icon.ico
```

## ライセンス

[MIT](LICENSE)
