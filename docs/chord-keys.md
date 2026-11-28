# キーペア押し / Key-pair chords

「キー」ページの **キーペア押しモード** をオンにすると、2つのキーの同時押しに
単押しとは別のキー組み合わせを割り当てられます。対象ペアは7つです。

With **Key pair mode** on (Keys page), pressing two keys together can fire
its own key combination, separate from either single press. Seven pairs:

| ペア / Pair | キー |
| --- | --- |
| 横・上段 | KEY1+KEY2、KEY2+KEY3 |
| 横・下段 | KEY4+KEY5、KEY5+KEY6 |
| 縦 | KEY4+KEY1、KEY5+KEY2、KEY6+KEY3 |

## 誤入力予防の仕組み / How mis-input prevention works

T1 はキーを押した瞬間に、焼かれている組み合わせをそのまま PC へ送ります。
そのため、単押しを普通に焼いたままでは、同時押しのつもりでも両方の単押しが
先に発火してしまいます。これを防ぐため、ペア押しモードでは次の方式を使います。

The T1 sends its burned combo the instant a key goes down, so with ordinary
singles burned, a "pair" press would fire both singles first. Pair mode
therefore works like this:

1. ペア対象のキーには、実物の組み合わせではなく **目印（センチネル）**
   `Ctrl+Shift+F1`〜`Ctrl+Shift+F6`（キー番号順）だけを焼きます。
2. 常駐アプリ（`--monitor`）が目印を横取りし、単押しを **判定間隔**
   （`chords.interval_ms`、既定 100ms）まで保留します。
3. 間隔内に相方のキーが押されれば、ペアの組み合わせだけを送出し、
   両方の単押しは飲み込みます。間隔を過ぎれば、保留していた単押しを
   押した順に送出します。間隔内に離せば、その場で単押し（タップ）に
   なります。

   Participating keys are burned with a **sentinel** only
   (`Ctrl+Shift+F1`..`F6`, in key order). The resident monitor intercepts
   the sentinels and holds a lone press for up to the decision interval
   (`chords.interval_ms`, default 100 ms). A partner press inside the
   interval injects only the pair combo and swallows both singles; after
   the interval the held single is injected, in press order. Releasing
   inside the interval fires the single at once, as a tap.

**代償 / Trade-off:** ペア対象のキーの単押しは、最大で判定間隔ぶん遅れます。
ペアに参加していないキーには遅れはありません。

A participating single press can be delayed by up to the interval. Keys in
no configured pair are never delayed.

## 使い方 / Use

1. まず各キーの単押しを通常どおり割り当てます（ペア押しモードがオフの
   状態で書くと、その内容が単押しとして記録されます）。
2. キーページでペア押しモードを **オン** にします。ペアに参加するキーの
   うち単押しが記録済みのものには、目印が自動で焼かれます。単押しが
   未記録のキーは、トーストで名前を知らせます。そのキーの単押しを
   先に割り当ててください（オンの状態で単押しを書くと、記録と目印の
   焼き込みが同時に行われます）。
3. ペアの行を選んだ状態で、キー画面で組み合わせを作って
   書き込むと、そのペアへの割り当てになります（ペアの割り当て自体は
   設定にだけ保存され、本体へは焼きません）。

1. Assign each key's single as usual (writing while pair mode is off
   records it as the single).
2. Turn pair mode **on**. Participating keys with a recorded single get
   their sentinel burned automatically. Keys without a recorded single
   are named in a toast; assign their single first (writing a single
   while pair mode is on records it and burns the sentinel at once).
3. Select a pair row, build a combo on the Keys page, and
   write: that assigns the pair (pair assignments live in settings only;
   nothing is burned for the pair itself).

ペア押しモード中の KEY1〜KEY6 への書き込みは、単一の組み合わせだけに
限られます（複数グループの組み合わせ、メディア／マウスは書けません）。

While pair mode is on, KEY1–KEY6 accept a single combo only (no
multi-group combos, no media/mouse actions).

## オン／オフと書き戻し / The toggle and restoring singles

- オンにすると目印が焼かれるのは **現在のプリセット（レイヤー）だけ**
  です。別のプリセットではペアは効かず、単押しがそのまま出ます。
- オフにすると、記録済みの単押しが本体へ書き戻されます。
- **本体が未接続のままオフにすると書き戻しはできません。** その場合、
  キーは目印のまま残り、トーストがその旨を知らせます。接続してから
  もう一度オフにしてください（単押しを割り当て直しても戻ります）。

- Sentinels are burned on the **current preset (layer) only**. On other
  presets pairs do not fire and singles come out as-is.
- Turning off burns the recorded singles back to the device.
- **Turning off while the device is disconnected cannot write back.**
  The keys then keep their sentinels and the toast says so; reconnect
  and switch off again (or re-assign the singles) to restore them.

## プラットフォーム対応 / Platform support

| プラットフォーム | 横取りと送出 | 状態 |
| --- | --- | --- |
| Linux | evdev グラブ＋uinput 仮想デバイス | 実装済み（実機未検証） |
| Windows | 低レベルキーボードフック＋SendInput | 実装済み（実機未検証） |
| macOS | — | **未対応**: トグルは保存されますが常駐アプリは横取りしません |

| Platform | Interception and injection | Status |
| --- | --- | --- |
| Linux | evdev grab + uinput virtual device | Implemented (not verified on hardware) |
| Windows | Low-level keyboard hook + SendInput | Implemented (not verified on hardware) |
| macOS | — | **Not supported yet**: the toggle is saved, but the resident does not intercept |

Linux では、ペア押しモードの間、常駐アプリが T1 の入力デバイスをグラブし、
目印以外のすべてのイベント（ノブの回転、メディアキーなど）を仮想デバイス
経由でそのまま流します。グラブには `/dev/uinput` への書き込み権限が必要です。

On Linux, while pair mode is on, the resident grabs the T1's input devices
and forwards every non-sentinel event (knob rotation, media keys, …)
through the virtual device unchanged. Grabbing needs write access to
`/dev/uinput`.

Windows では、目印の組み合わせ（Ctrl+Shift+F1〜F6）はペア押しモードの間、
どのアプリにも届きません。

On Windows, the sentinel combinations (Ctrl+Shift+F1–F6) do not reach any
app while pair mode is on.

## 設定 / Settings

`settings.json` の `chords` に保存されます。

Stored under `chords` in `settings.json`:

```json
{
  "chords": {
    "enabled": true,
    "interval_ms": 100,
    "pairs": { "1+2": { "mods": 1, "usage": 6 } },
    "singles": { "1": { "mods": 1, "usage": 4 } }
  }
}
```
