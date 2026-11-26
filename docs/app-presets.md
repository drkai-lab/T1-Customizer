# アプリプリセット / App presets

「キー」ページの **アプリプリセット** から、よく使うアプリの定番ショートカットを
KEY1〜KEY6（現在のプリセット／レイヤー）に一括で書き込めます。書き込むのは通常の
キー組み合わせ（修飾キー＋キー）で、修飾キーページで書くのと同じ内容です。
書き込みには本体の接続が必要です。

From **App presets** on the Keys page, one click writes a well-known app's
default shortcuts to KEY1–KEY6 of the current preset (layer). What is written
is a plain key combination (modifiers + key), the same bytes the Modifiers
page writes. The keyboard must be connected.

## Adobe Premiere 2026

| Key | 動作 (JA) | Action (EN) | Windows | macOS |
| --- | --- | --- | --- | --- |
| KEY1 | 編集点を追加 | Add Edit | Ctrl+K | Cmd+K |
| KEY2 | リップル削除 | Ripple Delete | Shift+Delete | Shift+Delete |
| KEY3 | 前の編集点をリップルトリミング | Ripple Trim Previous | Q | Q |
| KEY4 | 次の編集点をリップルトリミング | Ripple Trim Next | W | W |
| KEY5 | 取り消し | Undo | Ctrl+Z | Cmd+Z |
| KEY6 | 保存 | Save | Ctrl+S | Cmd+S |

ダイヤルの目安 / Dial suggestion: `=` / `-` でタイムラインのズーム
(= / - : timeline zoom)。

## Adobe Illustrator 2026

| Key | 動作 (JA) | Action (EN) | Windows | macOS |
| --- | --- | --- | --- | --- |
| KEY1 | 選択ツール | Selection Tool | V | V |
| KEY2 | ダイレクト選択ツール | Direct Selection Tool | A | A |
| KEY3 | ペンツール | Pen Tool | P | P |
| KEY4 | 文字ツール | Type Tool | T | T |
| KEY5 | 元に戻す | Undo | Ctrl+Z | Cmd+Z |
| KEY6 | 保存 | Save | Ctrl+S | Cmd+S |

ダイヤルの目安 / Dial suggestion: Ctrl+= / Ctrl+- でズーム
(Ctrl+= / Ctrl+- : zoom)。

## Adobe Photoshop 2026

| Key | 動作 (JA) | Action (EN) | Windows | macOS |
| --- | --- | --- | --- | --- |
| KEY1 | 移動ツール | Move Tool | V | V |
| KEY2 | ブラシツール | Brush Tool | B | B |
| KEY3 | 消しゴムツール | Eraser Tool | E | E |
| KEY4 | 自由変形 | Free Transform | Ctrl+T | Cmd+T |
| KEY5 | 取り消し | Undo | Ctrl+Z | Cmd+Z |
| KEY6 | 保存 | Save | Ctrl+S | Cmd+S |

ダイヤルの目安 / Dial suggestion: `[` / `]` でブラシサイズ
([ / ] : brush size)。

## CLIP STUDIO PAINT

| Key | 動作 (JA) | Action (EN) | Windows | macOS |
| --- | --- | --- | --- | --- |
| KEY1 | ペン | Pen Tool | P | P |
| KEY2 | 筆 | Brush Tool | B | B |
| KEY3 | 消しゴム | Eraser Tool | E | E |
| KEY4 | やり直し | Redo | Ctrl+Y | Cmd+Y |
| KEY5 | 取り消し | Undo | Ctrl+Z | Cmd+Z |
| KEY6 | 保存 | Save | Ctrl+S | Cmd+S |

ダイヤルの目安 / Dial suggestion: `[` / `]` でブラシサイズ
([ / ] : brush size)。

ノブ回転モード / Knob rotation mode:

このプリセットにはノブ回転の割り当ても含まれます。ノブが
「キー割り当て」モード（`dial.press_action = "key"`）のときにプリセットを
適用すると、KEY1〜KEY6 に加えて K1-L に左回転（`-`）、K1-R に右回転
（`^`、日本語配列のキー。T1 のキー表では `=` の usage）を書き込むので、
ノブを回すとキャンバスが回転します。「音量モード切り替え」のときは
ノブ回転分は書き込まず、その旨を表示します。

This preset also carries a knob rotation pair. When the knob is in key
assignment mode (`dial.press_action = "key"`), applying the preset writes,
in addition to KEY1–KEY6, Rotate Left (`-`) to K1-L and Rotate Right
(`^` on a Japanese layout; the `=` usage in the T1 key table) to K1-R,
so turning the knob rotates the canvas. In volume mode switch mode the
knob pair is skipped and the app says so.

メモ / Notes:

- やり直し (Redo) のデフォルトは Ctrl+Y と Ctrl+Shift+Z の両方です。
  プリセットでは Ctrl+Y を使います。
  Redo defaults to both Ctrl+Y and Ctrl+Shift+Z; the preset uses Ctrl+Y.
- 左右反転 (Flip Horizontal) にはデフォルトのショートカットが無いため、
  プリセットには入れていません。
  Flip Horizontal has no default shortcut in CLIP STUDIO PAINT, so it is
  not part of the preset.
- macOS では Adobe と同じく Ctrl → Cmd の読み替えが要ります（下の注意）。
  On macOS, read Ctrl as Cmd, as with the Adobe apps (see the note below).

## macOS についての注意 / Note for macOS

プリセットが書き込むのは Windows 式の **Ctrl** です。macOS では Adobe の
コマンドの多くが **Cmd** なので、Mac で使う場合は、書き込んだキーを
修飾キーページで選び直し、Win（Cmd）キーで保存し直してください。

The presets write **Ctrl**, as on Windows. On macOS most Adobe commands use
**Cmd** instead, so Mac users may re-save a written key with Win (Cmd) on
the Modifiers page.

## バージョンについて / About versions

Adobe は「2026」専用のショートカット一覧を公開していません。2026 年版は
Premiere 26.x（Premiere Pro から改名）、Illustrator 30.x、Photoshop 27.x で、
下記の公式一覧（随時更新）がそのまま当てはまります。

Adobe publishes no separate "2026" shortcut list. The 2026 versions are
Premiere 26.x (renamed from Premiere Pro), Illustrator 30.x and
Photoshop 27.x, and the official rolling lists below apply as-is.

## 出典 / Sources

- Premiere: <https://helpx.adobe.com/jp/premiere/desktop/get-started/keyboard-shortcuts/default-keyboard-shortcuts.html>
- Illustrator: <https://helpx.adobe.com/illustrator/desktop/get-started/preferences-and-settings/default-keyboard-shortcuts-for-illustrator.html>
- Photoshop: <https://helpx.adobe.com/content/dam/help/en/photoshop/using/default-keyboard-shortcuts/photoshop-keyboard-shortcuts.pdf>
- CLIP STUDIO PAINT (JA): <https://help.clip-studio.com/ja-jp/manual_jp/780_shortcuts/780_shortcuts.htm>
- CLIP STUDIO PAINT (EN): <https://help.clip-studio.com/en-us/manual_en/780_shortcuts/Menu_Shortcuts.htm>
