<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-CAM-DEBUGGER banner" width="100%">
</p>

# <img src="images/brand/icon-64.png" alt="" width="40" height="40" align="absmiddle"> EH3D-CAM-DEBUGGER
<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | 🇯🇵 <b>日本語</b> | <a href="README_por.md">🇵🇹 Português</a></p>

### 機械に届く前の G コード プログラムの最終確認：時間と警告

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.5-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**正直さの確認 - 現在実際に動くもの：** Rust ライブラリとコマンドライン プログラム（`eh3d-camdebug`）で、コンピューター上で実行される 123 件のテストがあります：G コード プログラムの時間を計算し、材料への早送り、テーブル外の移動、主軸停止での切削、クランプ上を通るヘッドを見つけます。ウィンドウ（`--features gui` でビルド）はG コード プログラムを開き、経路を 2D で描き、行の表とともに移動を 1 つずつ順に確認でき、アラートと警告を表示するが、画面ではまだ確認しておらず、機械では試しておらず、実際のファイルの集まりでも確認していません。

---

> **安全とセキュリティの状況：** このプログラムは機械を制御せず、安全システムでもありません。出力は、機械で何かを実行する前に人が確認する必要があります。ここにあるものは実機で動かしたことがありません。


## 📋 ひと目で

| | |
|---|---|
| バージョン | `0.0.5` |
| 成熟度 | 足場 |
| テスト | 123 |
| 技術スタック | Rust |
| 実機での動作 | 未試験：ここにあるものは機械で動かしたことがありません |

**移動先：** [概要](#-概要) · [アーキテクチャ](#-アーキテクチャ) · [クイックスタート](#-クイックスタート) · [テスト](#-テスト) · [構成](#-リポジトリの構成) · [関連](#-関連プロジェクト)

## 🚦 プロジェクトの状況

各行はこのリポジトリの一部分で、信号は今日どこまで進んでいるかを示します。計画ではなく、リポジトリが示せるもの（テスト、ビルド、実行）に基づいて書かれています。

🟢 完了・検証済み · 🟡 動作する（既知の制限あり） · 🟠 一部のみ、または未検証 · 🔴 未実装または動作しない

| 領域 | 状態 | ここでの意味 |
|---|---|---|
| コア | 🟡 | G コード プログラムの時間を計算し、材料への早送り、テーブル外の移動、主軸停止での切削、クランプ上を通るヘッドを見つけます。コンピューター上で通常入力と破損入力でテスト済み。実際のファイルの集まりでは未確認。 |
| コマンドライン | 🟡 | `eh3d-camdebug` はライブラリと同じ関数を実行して結果を表示します。エラーはメッセージと 0 以外の終了コードで返ります。 |
| テスト | 🟡 | 123 件のテストが通ります（`cargo test`）。通常ケース、境界ケース、破損入力を含みます。 |
| セキュリティ | 🟡 | 安全な Rust のみ（`forbid(unsafe_code)`）。受け取ったファイルでパニックせず、不正な値は理由付きのエラーになります。独立したレビューはありません。 |
| ウィンドウ | 🟠 | 作成済み（egui、`--features gui`）：プログラムを開くかドロップ、経路の 2D 表示、移動の 1 つずつの確認と再生、移動の表、アラートと警告、機械の設定と範囲。ロジックはテスト済みですが、まだビルドも画面での確認もしていません。 |
| 実機 | 🔴 | 未試験：ここにあるものは機械で実行したことがありません。 |
| ドキュメント | 🟡 | 8 言語の README、変更履歴、`docs/` から描いた 2 枚の図。 |

## 🎯 概要

* **時間**（`src/timeline.rs`）：各移動は、機械の加速度と早送り速度による停止から停止までの速度台形として計算します。円弧は弧に沿って測り、送りのない切削は既定値を使い、合計は切削と早送りに分けます。実際のコントローラーは移動間で止まらないため、上限値です。 `G4` の待機（`P` は秒、Marlin ではミリ秒で `S` が秒）は移動ではないため、別に加算されます。
* **警報**（`src/checks.rs`）：材料内への早送り（G0）や表面下での横移動、テーブル外への移動、機械の最低 Z より下の Z、高さより低くクランプの上を通るヘッドは警報（終了コード 1）です。主軸停止での切削、送りなしの切削、上限を超える送りは警告です。直線移動は各クランプに対して厳密に判定されるため、どのステップよりも薄いクランプでも見つかります。
* **リーダーが理解できなかったもの**（工具径補正、固定サイクル、壊れた行）は行番号付きの警報として現れるため、誤読されたプログラムが問題なしと報告されることはありません。
* **円弧、`--strict`、原点を動かすコード：** テーブルの範囲とクランプは、G2・G3 の弦ではなく円弧に沿って検査されます。`--strict` はすべての警告をアラートにし（終了コード 1）、終了コードを機械の前の関門にする運用で使えます。G92、G53、G28、G30、G10 を含むプログラムは、問題なしとして通らず報告されます。
* **読み込みとコード別の通知：** プログラムはバイト列から読み込まれます（`--max-bytes N` は上限（既定 256 MiB）を超えるファイルを拒否し、UTF-8 でないテキストは Latin-1 として読み、コード `input-not-utf8` で報告します）。リーダーの指摘はそれぞれ安定したコードを示し、`--ignore CODE[,CODE]` でそのコードの通知を省けます。

## 🧩 EH3D での役割

EH3D-CAM-DEBUGGER は EH3D Fabrication Suite のプログラムの一つです。ライブラリとコマンドラインとして動作し、`--features gui` でビルドすると、スイートの他のプログラムと共通の外観のウィンドウとしても動作します。単純なファイルを読み書きするため、単独でも他のプログラムの間でも使えます。

## 🏛️ アーキテクチャ

<p align="center">
  <img src="images/architecture.svg" alt="EH3D-CAM-DEBUGGER のアーキテクチャ：誰が使い、何を持ち、何に依存するか" width="100%">
</p>


このリポジトリの各部分の関係：誰が使い、何を持ち、何に依存するか。図は `docs/architecture.json` から描かれ、コードと照合されます。

## ✨ 機能

* **ライブラリとコマンドライン：** `eh3d-camdebug` の各コマンドはライブラリの関数なので、スイートの他のプログラムがプロセスを起動せずに呼び出せます。
* **破損入力のテスト：** 空のファイル、ゴミデータ、巨大な数値、途中で切れたファイルも与え、どれもパニックしてはいけません。
* **推測しない：** 意味のない値は、どの値かと理由を示すエラーになり、見かけだけ正しい結果にはなりません。
* **unsafe コードなし：** ライブラリとプログラムに `#![forbid(unsafe_code)]`。
* **送りの欠落は一度だけ知らせ、レーザーモードを読みます：** 送りのない移動ごとに指摘は 1 件で（`--ignore feed-missing` で消せます）、`--laser` は `S0` の `M3` または `M4` をレーザーが消えているものとして読みます。

## 🔒 セキュリティと安全

| 脅威 | 対策 |
|---|---|
| 破損・途中切れ・空のファイル | パニックなし：テストは有効な入力をあらゆる位置で切り、ゴミデータも与えます |
| 意味のない数値（負、無限、極端に大きい） | 値を示すエラーで拒否され、もっともらしい値に丸められることはありません |
| 安全でないメモリ使用 | `forbid(unsafe_code)` の安全な Rust のみ |

**防御していないこと：** エラーなく作られた結果は、検証された結果ではありません。ここには機械の限界を知るものはなく、プログラムは独立したレビューを受けていません。脆弱性は非公開で報告してください（[SECURITY.md](SECURITY.md) 参照）。

## 🔌 インターフェース

ライブラリは `eh3d_cam_debugger`（`use eh3d_cam_debugger::...`）、コマンドラインは `eh3d-camdebug` です。コマンドは次のとおりで、結果は標準出力に表示され、エラーは標準エラー出力に 0 以外の終了コードで報告されます。ネットワークアクセスはありません。

```text
eh3d-camdebug check FILE.nc [options]     the time, the findings; exit code 1 when there is an alert, 2 when there are only warnings, 64 for a wrong option
eh3d-camdebug steps FILE.nc [options]     every move with its line, length and time
eh3d-camdebug [gui] [FILE.nc]             the window (a build with --features gui); with no arguments it opens
options: --accel MM_S2 (500)  --rapid MM_MIN (3000)  --feed MM_MIN (300)  --dialect grbl|marlin|planetcnc  --table WIDTH,DEPTH  --surface Z (0)  --min-z Z  --max-feed MM_MIN
         --laser  --clamp X0,Y0,X1,Y1,HEIGHT  --strict  --ignore CODE[,CODE]  --max-bytes N
```

## ⚙️ 設定

設定ファイルはまだありません。選択は（インターフェースを参照）コマンドラインの引数で渡し、各既定値はコマンドのヘルプ（`eh3d-camdebug help`）に書かれています。固定の上限はコード内の名前付き定数で、テストで確認されています。

## 🚀 クイックスタート

安定版の Rust ツールチェーン（1.88 以降）が必要です。

```bash
cargo build --release
cargo test
cargo run --release -- check examples/sample.nc --table 100,80 --clamp 20,-5,30,5,6
cargo run --release --features gui   # the window
cargo run --release --features gui -- --shot screen.png examples/sample.nc   # a picture of the window
```

手動で開かずにウィンドウの画像を撮るには、`--shot FILE.png` を付けて実行します（`--size WxH`, `--theme forge|dark|light`, `--wait N`, `--lang CODE`, `--open FILE`, `--replace` も使えます）。指定のサイズで開き、PNG を保存し、設定は保持せず、自動で閉じます（終了コードは、ファイルを書き込めたら 0、書けなければ 1、引数が誤りなら 2）。

## 🧪 テスト

テストは 123 件あります（`cargo test`、CI では Windows と Linux で実行）。通常ケース、境界ケース、破損入力を確認し、空のファイル、ゴミデータ、途中で切れたファイルでもパニックしません。CI は `cargo fmt --check`、警告をエラーにした `cargo clippy`、`python -I tools/ci_validate.py` も実行します。

## 🏗️ ビルドとリリース

バージョンは `Cargo.toml` と `eh3d.project.json` にあり、各バージョンは [CHANGELOG.md](CHANGELOG.md) に項目があります。`build.bat` / `build.sh` でビルド、`test.bat` / `test.sh` でテスト、`run.bat` / `run.sh` でサンプル実行します。リリースは GitHub で公開されます。

**ダウンロード:** このリポジトリのリリースごとに、各システム向けのビルド済みプログラムが公開されます。Windows (64 ビットと 32 ビット)、Linux (64 ビット)、macOS (Apple シリコンと Intel) です。各パッケージ (`cam-debugger-<バージョン>-<システム>.zip` または `.tar.gz`) には、プログラム、ライセンス、この README、変更履歴、アイコン、サンプルファイルが入っており、`.sha256` ファイルが付きます。リリースには全ファイルを一覧にした `SHA256SUMS` と `build-manifest.json` も含まれます。まだリリースは公開されていません。

**署名と検証:** リリースのパッケージは EH3D のリリース鍵で署名され、各パッケージの `.sig` ファイルがその隣に公開されます (OpenSSH 署名、名前空間 `eh3d-release`)。実行する前にダウンロードを確認してください。SHA-256 を比較し (`sha256sum -c cam-debugger-<バージョン>-<システム>.tar.gz.sha256`、Windows では `Get-FileHash`)、署名を `ssh-keygen -Y verify -f allowed_signers -I eh3d-release -n eh3d-release -s <パッケージ>.sig < <パッケージ>` で検証します。`allowed_signers` には `eh3d-release <鍵の種類> <EH3D リリースの公開鍵>` の行を入れます。

## 📂 リポジトリの構成

<p align="center">
  <img src="images/repository-structure.svg" alt="EH3D-CAM-DEBUGGER のリポジトリ構成" width="100%">
</p>

<details>
<summary>ツリーのテキスト版</summary>

```text
EH3D-CAM-DEBUGGER/
├── src/
│   ├── checks.rs                                ロジックの一部
│   ├── lib.rs                                   ライブラリ：各コマンドが使うもの
│   ├── main.rs                                  コマンドライン
│   ├── timeline.rs                              ロジックの一部
│   ├── browse.rs                                ファイルを開くウィンドウのファイル一覧
│   ├── gui.rs                                   ウィンドウ（egui、gui 機能）：状態を描き、操作を伝える
│   ├── lang.rs                                  ウィンドウの文言（8言語）
│   ├── shot.rs                                  ウィンドウの画像モード（--shot）：画面なしで読み取り・検証される引数
│   ├── viewport.rs                              画像の拡大縮小と移動（単純な数値）
│   └── window.rs                                画面なしのウィンドウの状態：プログラム、設定、移動、ステップ
├── tests/
│   ├── debugger.rs                              ライブラリとコマンドラインのテスト
│   ├── reading.rs                               ファイル読み込みのテスト：サイズ上限、UTF-8以外のテキスト、コード別の通知
│   ├── readme.rs                                READMEとrunスクリプトが約束する内容を実行するテスト
│   └── window_cli.rs                            gui 機能なしのビルドがコマンドラインのままであることのテスト
├── examples/
│   └── sample.nc                                試すためのサンプル
├── docs/
│   ├── architecture.json                        アーキテクチャ図のデータ
│   └── structure.json                           構成図のデータ
├── tools/                                       CI と README のチェック、図のツール
├── images/                                      バナーとアーキテクチャ・リポジトリ構成の図
├── .github/                                     CI とリリースのワークフロー、Issue と PR のテンプレート
├── eh3d.project.json                            マニフェスト：バージョン、役割、成熟度、ビルドコマンド
├── Cargo.toml Cargo.lock                        ワークスペースと固定された依存関係
├── rust-toolchain.toml                          固定されたRustツールチェーン
├── CHANGELOG.md                                 各バージョンの変更点
├── SECURITY.md SUPPORT.md                       脆弱性の報告方法と問い合わせ先
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   貢献方法、行動規範、ライセンス
```

</details>

## 🗺️ ロードマップと既知の制限

**現時点で分かっている制限**

* ウィンドウはまだ画面で確認しておらず、経路を真上から表示します（3D 表示なし）。検出結果の文章はライブラリのもので、英語です。
* 機械でも実際のファイルの集まりでも試しておらず、独立したレビューも受けていません。
* 3D シミュレーションと材料除去はまだなく、クランプは手で記述する直方体です。

**次のステップ**

* スイートのウィンドウで材料除去を 3D でシミュレートし、結果を部品と比較する。
* ウィンドウを画面で確認して仕上げる。
* 実際のファイルとスイートの他のプログラムで試す。

## 🌍 言語

README とテキストは、英語、スペイン語、フランス語、イタリア語、ドイツ語、中国語、日本語、ヨーロッパポルトガル語で提供されています。

## 🔗 関連プロジェクト

* [EH3D-CORE-LIBS](https://github.com/JuanenRac/EH3D-CORE-LIBS): Gerber、Excellon、G コードのリーダー
* [EH3D-ISOCAM](https://github.com/JuanenRac/EH3D-ISOCAM): 銅箔の絶縁ミリング
* [EH3D-ROUTEPRO](https://github.com/JuanenRac/EH3D-ROUTEPRO): 輪郭ミリング プログラム
* [EH3D-AUTOLEVEL](https://github.com/JuanenRac/EH3D-AUTOLEVEL): 高さマップと G コードの補正
* [EH3D-PHOENIX-CNC](https://github.com/JuanenRac/EH3D-PHOENIX-CNC): 機械制御プログラム

## 📚 ドキュメント

* [変更履歴](CHANGELOG.md)
* [アーキテクチャと構成のデータ](docs/architecture.json)：2 枚の図を描くためのデータ。

## 🤝 コントリビュートとコミュニティ

* プルリクエストの前に `cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test`、`python -I tools/ci_validate.py` を実行し、すべて通してください。[CONTRIBUTING.md](CONTRIBUTING.md) と[行動規範](CODE_OF_CONDUCT.md)を参照。
* README は 8 言語あります。文章の変更は 8 ファイルすべてで行い、2 枚の図は `tools/make_architecture.py` と `tools/make_structure.py` が `docs/architecture.json` と `docs/structure.json` から描きます。
* 脆弱性は非公開で報告してください：[SECURITY.md](SECURITY.md)。質問・提案・報告：electrohobby3d@gmail.com（[SUPPORT.md](SUPPORT.md)）。

## 👤 作者

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 ライセンス

ソフトウェア：GPL-3.0-or-later。[LICENSE](LICENSE) を参照。スイートのハードウェア設計には CERN OHL-S を使います。
