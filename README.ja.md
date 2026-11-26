# T1-Customizer

**T1 ミニキーボード**（[6 Keys 1 Knob RGB Programming Macro Gaming Keypad](https://ja.aliexpress.com/item/1005009812219099.html)、AliExpress、USB VID `1189` / PID `8890`）の設定ツールです。無料です。

> Forked from T1-VREmoKey (えもかえ) by AppWizz Kai, version 2.2.4.
> このフォークでは、元アプリの VRChat 関係の機能とアクティベーション（ライセンス）による制限をすべて外しました。無料で、全機能が使えます。

[English](README.md) | 日本語 | [简体中文](README.zh-CN.md) | [한국어](README.ko.md)

## できること

- キー割り当て（キーごとに修飾キーの組み合わせを最大5グループ）
- アプリプリセット: Adobe Premiere / Illustrator / Photoshop 2026 と CLIP STUDIO PAINT のショートカット一式を KEY1〜KEY6 に一括書き込み（`docs/app-presets.md` 参照）
- 名前付きの修飾キーの組み合わせ、マルチメディア、マウス、LED
- プリセット 1/2/3 の切り替えと本体への書き込み
- ダイヤル回転で OS の音量（マイク／ヘッドホン）を操作。押し込むとモード切り替え。待受はトレイに残ります
- ノブの押し込みの選択: ダイヤル画面で、ノブを「音量モード切り替え」（既定）か「キー割り当て」か選べます。キー割り当てでは押し込み（K1-C / K2-C）・左回し・右回しの3パターンをキーパネルで選んで、通常のキーと同じように書き込めます
- CLIP STUDIO PAINT プリセットのノブ回転: キー割り当てモードでプリセットを適用すると、キャンバスの左回転／右回転も K1-L / K1-R に書き込めます
- `settings.json` は Linux / Windows / macOS で共通（コードは Linux evdev コードで保存）。元アプリで書いた設定のうち、このフォークに残る機能の分はそのまま読み込めます。外した機能の設定は無視されますが、ファイルからは消えません
- 画面の言語は日本語／英語／韓国語。`settings.json` の `lang` に保存されます

## 入っていないもの

- VRChat 関係は一切ありません（OSC、アバタープロファイル、表情ホットキー、Unity パッケージ）
- アクティベーション、ライセンス、体験版、注文番号による制限はありません
- Rust のみです。古い Python 実装は含みません

## 状態

| OS | 状態 |
| --- | --- |
| Linux | 実装済み。このフォークでは実機未確認 |
| Windows 11 | 実装済み。このフォークでは実機未確認 |
| macOS | 実装済み、**未検証**（検証できる macOS 機がありません） |

本体プロトコルのコードは元の 2.2.4 から変えていません。2.2.4 自体も実機未確認でした（実機確認済みなのは Python 1.4.0）。

## 使い方

```bash
t1-keyboard-config            # 設定 GUI
t1-keyboard-config --selftest # デバイス・権限・書き込み経路の診断
t1-keyboard-config --watch    # T1 の入力イベントをライブ表示
t1-keyboard-config --monitor  # ダイヤル音量の待受
t1-keyboard-config --version
```

ウィンドウを閉じてもトレイに残ります（ウィンドウを開く／待受の開始・停止／終了）。

## ソースからビルド

```bash
cd rust
cargo build --release
cargo test
```

Linux の GUI ビルドには GTK 3 の開発パッケージ（`libgtk-3-dev`、`libayatana-appindicator3-dev`）が要ります。
Linux では `99-t1-keyboard.rules` を `/etc/udev/rules.d/` に入れると、root なしでデバイスにアクセスできます。

## ライセンス

MIT。`LICENSE` を見てください。
