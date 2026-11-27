//! Japanese, English and Korean UI strings. The Japanese copy stays close
//! to the existing README tone: short, concrete, polite. Korean lives in
//! `i18n_ko.rs`; every key here has an entry there (checked by a test).

pub fn tr(lang: &str, key: &str) -> &'static str {
    if lang == "ko"
        && let Some(text) = super::i18n_ko::tr(key)
    {
        return text;
    }
    let ja = lang == "ja";
    match key {
        "app" => "T1-Customizer",
        "app_presets" => {
            if ja {
                "アプリプリセット"
            } else {
                "App presets"
            }
        }
        "app_preset_applied" => {
            if ja {
                "プリセットを書き込みました"
            } else {
                "Preset written"
            }
        }
        "app_preset_partial" => {
            if ja {
                "プリセットの一部だけ書き込みました（{ok}件）"
            } else {
                "Preset partially written ({ok} items)"
            }
        }
        "app_preset_knob" => {
            if ja {
                "ノブ回転（左回転／右回転）も書き込みました"
            } else {
                "Knob rotation written too"
            }
        }
        "app_preset_knob_skip" => {
            if ja {
                "ノブ回転は「キー割り当て」モードのときだけ書き込めます"
            } else {
                "Knob rotation needs key assignment mode"
            }
        }
        "chords_mode_label" => {
            if ja {
                "キーペア押しモード"
            } else {
                "Key pair mode"
            }
        }
        "chords_on" => {
            if ja {
                "オン"
            } else {
                "On"
            }
        }
        "chords_off" => {
            if ja {
                "オフ"
            } else {
                "Off"
            }
        }
        "chords_note" => {
            if ja {
                "オンの間、ペア対象のキーには目印（Ctrl+Shift+F1〜F6）だけを焼き、同時押しの判定は常駐アプリが行います。単押しは最大で判定間隔（既定100ms）だけ遅れます。目印は現在のプリセットにだけ焼かれます。"
            } else {
                "While on, paired keys carry only a sentinel (Ctrl+Shift+F1-F6) and the resident app decides pairs. A single press can be delayed by up to the decision interval (default 100 ms). Sentinels are burned on the current preset only."
            }
        }
        "chords_assign_hint" => {
            if ja {
                "ペアを選んだ状態で、キー一覧や修飾ページで組み合わせを作って書き込むと、そのペアへの割り当てになります。"
            } else {
                "With a pair selected, building a combo on the Keys or Modifiers page and writing it assigns that pair."
            }
        }
        "chords_single_first" => {
            if ja {
                "先に単押しを割り当ててください: {keys}"
            } else {
                "Assign these single keys first: {keys}"
            }
        }
        "chords_burn_pending" => {
            if ja {
                "本体が未接続のため、目印の書き込みは保留中です。接続してからペアを割り当て直してください。"
            } else {
                "Device not connected: sentinel burning is pending. Reconnect and assign the pair again."
            }
        }
        "chords_restore_pending" => {
            if ja {
                "本体が未接続のため、単押しの書き戻しができていません。キーは目印のままです。接続してからもう一度オフにしてください。"
            } else {
                "Device not connected: singles could not be written back. The keys still carry sentinels. Reconnect and switch off again."
            }
        }
        "chords_pair_saved" => {
            if ja {
                "ペア {pair} に {desc} を割り当てました"
            } else {
                "Assigned {desc} to pair {pair}"
            }
        }
        "config_save" => {
            if ja {
                "設定を保存"
            } else {
                "Save settings"
            }
        }
        "config_load" => {
            if ja {
                "設定を読み込む"
            } else {
                "Load settings"
            }
        }
        "config_saved" => {
            if ja {
                "設定を保存しました：{path}"
            } else {
                "Settings saved: {path}"
            }
        }
        "config_save_empty" => {
            if ja {
                "保存できるボタン設定がまだありません"
            } else {
                "No button settings to save yet"
            }
        }
        "config_save_failed" => {
            if ja {
                "設定の保存に失敗しました"
            } else {
                "Could not save the settings"
            }
        }
        "config_load_bad" => {
            if ja {
                "設定ファイルを読めませんでした"
            } else {
                "Could not read the settings file"
            }
        }
        "config_loaded_pending" => {
            if ja {
                "設定を読み込みました。本体が未接続のため、書き込みはまだです"
            } else {
                "Settings loaded. No device connected, so nothing was written yet"
            }
        }
        "config_loaded_counts" => {
            if ja {
                "設定を読み込みました（書き込み {ok} 件、失敗 {failed} 件、スキップ {skipped} 件）"
            } else {
                "Settings loaded ({ok} written, {failed} failed, {skipped} skipped)"
            }
        }
        "chords_single_only" => {
            if ja {
                "ペア押しモード中、KEY1〜KEY6 に書けるのは単一の組み合わせだけです"
            } else {
                "In pair mode, KEY1-KEY6 accept a single combo only"
            }
        }
        "chords_keys_only" => {
            if ja {
                "ペア押しモード中、KEY1〜KEY6 にはメディア／マウスを書けません"
            } else {
                "In pair mode, KEY1-KEY6 cannot take media/mouse actions"
            }
        }
        "chords_on_done" => {
            if ja {
                "キーペア押しモードをオンにしました"
            } else {
                "Key pair mode is on"
            }
        }
        "chords_off_done" => {
            if ja {
                "キーペア押しモードをオフにし、単押しを書き戻しました"
            } else {
                "Key pair mode is off; singles written back"
            }
        }
        "connected" => {
            if ja {
                "接続中"
            } else {
                "Connected"
            }
        }
        "disconnected" => {
            if ja {
                "未接続"
            } else {
                "Not connected"
            }
        }
        "download_ok" => {
            if ja {
                "書き込み成功"
            } else {
                "Write OK"
            }
        }
        "download_ng" => {
            if ja {
                "書き込み失敗"
            } else {
                "Write failed"
            }
        }
        "download" => {
            if ja {
                "書き込み"
            } else {
                "Write to keyboard"
            }
        }
        "clear" => {
            if ja {
                "選択を解除"
            } else {
                "Clear selection"
            }
        }
        "disable_key" => {
            if ja {
                "※ このキーを無効化"
            } else {
                "※ Disable this key"
            }
        }
        "page_key" => {
            if ja {
                "キー"
            } else {
                "Keys"
            }
        }
        "page_fun" => {
            if ja {
                "修飾キー"
            } else {
                "Modifiers"
            }
        }
        "page_media" => {
            if ja {
                "マルチメディア"
            } else {
                "Media"
            }
        }
        "page_mouse" => {
            if ja {
                "マウス"
            } else {
                "Mouse"
            }
        }
        "page_led" => "LED",
        "page_dial" => {
            if ja {
                "ダイヤル"
            } else {
                "Dial"
            }
        }
        "page_about" => {
            if ja {
                "ヘルプ"
            } else {
                "Help"
            }
        }
        "layer" => {
            if ja {
                "プリセット"
            } else {
                "Preset"
            }
        }
        "layer_hint" => {
            if ja {
                "T1キーボード本体に保存できるキー割り当てのセット（1～3）。表情はプリセット 1 に書き込む"
            } else {
                "A set of key assignments stored on the T1 Keyboard (1–3). Expressions are written to preset 1."
            }
        }
        "selected" => {
            if ja {
                "選択中のキー"
            } else {
                "Selected key"
            }
        }
        "current" => {
            if ja {
                "現在の設定内容(このツールが書き込んだ記録)"
            } else {
                "Current assignment (last written by this tool)"
            }
        }
        "no_key" => {
            if ja {
                "なし"
            } else {
                "none"
            }
        }
        "pick_first" => {
            if ja {
                "先に左の物理キーを選択してください"
            } else {
                "Select a physical key first"
            }
        }
        "pick_function" => {
            if ja {
                "書き込む機能を選んでください"
            } else {
                "Pick a function to write"
            }
        }
        "write_unavailable" => {
            if ja {
                "このタブには、キーボードへ書き込む項目がありません"
            } else {
                "This tab has nothing to write to the keyboard"
            }
        }
        "write_need_combo" => {
            if ja {
                "修飾かベースキーを選んでから書き込んでください"
            } else {
                "Choose a modifier or base key before writing"
            }
        }
        "need_device" => {
            if ja {
                "未接続です"
            } else {
                "Not connected"
            }
        }
        "learn_key" => {
            if ja {
                "キーを学習"
            } else {
                "Learn key"
            }
        }
        "learn_push" => {
            if ja {
                "押し込みを学習"
            } else {
                "Learn push"
            }
        }
        "learn_rotate" => {
            if ja {
                "回転を学習"
            } else {
                "Learn rotate"
            }
        }
        "learning" => {
            if ja {
                "T1キーボードのキーを押してください(5秒)..."
            } else {
                "Press the key on the T1 Keyboard now (5s)..."
            }
        }
        "learn_done" => {
            if ja {
                "学習しました"
            } else {
                "Learned code"
            }
        }
        "learn_fail" => {
            if ja {
                "取得できませんでした"
            } else {
                "Nothing captured"
            }
        }
        "learn_no_device" => {
            if ja {
                "入力デバイスを読めません（input グループ / udev ルール）。README「自己診断」を参照してください。"
            } else {
                "Cannot read an input device (input group / udev rule). See README “Self test”."
            }
        }
        "learn_timeout" => {
            if ja {
                "5 秒以内にキーが押されませんでした。もう一度［学習］を押してください。"
            } else {
                "No key was pressed within 5 seconds. Press Learn again."
            }
        }
        "assign_empty" => {
            if ja {
                "未設定"
            } else {
                "empty"
            }
        }
        "mod_base" => {
            if ja {
                "ベースキー"
            } else {
                "Base key"
            }
        }
        "mod_only" => {
            if ja {
                "(修飾のみ)"
            } else {
                "(modifier only)"
            }
        }
        "mod_add" => {
            if ja {
                "+ グループ追加"
            } else {
                "+ Add group"
            }
        }
        "mod_reset" => {
            if ja {
                "リセット"
            } else {
                "Reset"
            }
        }
        "mod_name" => {
            if ja {
                "名前"
            } else {
                "Name"
            }
        }
        "mod_save" => {
            if ja {
                "保存"
            } else {
                "Save"
            }
        }
        "mod_delete" => {
            if ja {
                "削除"
            } else {
                "Delete"
            }
        }
        "mod_saved" => {
            if ja {
                "保存した組み合わせ"
            } else {
                "Saved combinations"
            }
        }
        "mod_groups" => {
            if ja {
                "グループ"
            } else {
                "Groups"
            }
        }
        "mod_saved_ok" => {
            if ja {
                "組み合わせを保存しました"
            } else {
                "Combination saved"
            }
        }
        "mod_deleted" => {
            if ja {
                "組み合わせを削除しました"
            } else {
                "Combination deleted"
            }
        }
        "mod_need_name" => {
            if ja {
                "この組み合わせの名前を入力してください"
            } else {
                "Enter a name for this combination"
            }
        }
        "mod_need_combo" => {
            if ja {
                "修飾キーかベースキーを選んでください"
            } else {
                "Choose modifiers or a base key"
            }
        }
        "mod_need_base" => {
            if ja {
                "グループを追加するにはベースキーを選んでください"
            } else {
                "Choose a base key to add a group"
            }
        }
        "mod_lone_ready" => {
            if ja {
                "修飾のみです。保存か書き込みを押してください。"
            } else {
                "Modifier-only is ready. Press Save or Write."
            }
        }
        "mod_group_full" => {
            if ja {
                "1つのキーに入れられるグループは最大5つです"
            } else {
                "A key holds at most 5 groups"
            }
        }
        "mod_pick_saved" => {
            if ja {
                "先に保存した組み合わせを選択してください"
            } else {
                "Select a saved combination first"
            }
        }
        "monitor_running" => {
            if ja {
                "状態: 動作中"
            } else {
                "Status: running"
            }
        }
        "monitor_stopped" => {
            if ja {
                "状態: 停止中"
            } else {
                "Status: stopped"
            }
        }
        "start_monitor" => {
            if ja {
                "待受を開始"
            } else {
                "Start listening"
            }
        }
        "stop_monitor" => {
            if ja {
                "待受を止める"
            } else {
                "Stop listening"
            }
        }
        "monitor_error" => {
            if ja {
                "待受を開始できませんでした"
            } else {
                "Listening failed to start"
            }
        }
        "tray_open" => {
            if ja {
                "画面を開く"
            } else {
                "Open window"
            }
        }
        "tray_start" => {
            if ja {
                "待受の開始"
            } else {
                "Start resident"
            }
        }
        "tray_stop" => {
            if ja {
                "待受の停止"
            } else {
                "Stop resident"
            }
        }
        "tray_quit" => {
            if ja {
                "終了"
            } else {
                "Quit"
            }
        }
        "tray_title" => {
            if ja {
                "トレイメニュー"
            } else {
                "Tray menu"
            }
        }
        "port_invalid" => {
            if ja {
                "1〜65535 の数字を入れてください"
            } else {
                "Enter a number from 1 to 65535"
            }
        }
        "host_invalid" => {
            if ja {
                "ホスト名を入れてください（空白は使えません）"
            } else {
                "Enter a host name with no spaces"
            }
        }
        "addr_invalid" => {
            if ja {
                "アドレスは / で始めてください"
            } else {
                "The address must start with /"
            }
        }
        "dial_mode_mic" => {
            if ja {
                "マイク音量"
            } else {
                "Mic volume"
            }
        }
        "dial_mode_headphone" => {
            if ja {
                "イヤーマフ音量"
            } else {
                "Headphone volume"
            }
        }
        "dial_press_label" => {
            if ja {
                "ノブの押し込み"
            } else {
                "Knob press"
            }
        }
        "dial_press_cycle" => {
            if ja {
                "音量モード切り替え"
            } else {
                "Volume mode switch"
            }
        }
        "dial_press_key" => {
            if ja {
                "キー割り当て"
            } else {
                "Key assignment"
            }
        }
        "dial_press_note" => {
            if ja {
                "キー割り当てでは、押し込み／左回し／右回しの3つをキー画面で割り当てられます。この間ノブはモード切替も音量変更もしません"
            } else {
                "In key assignment mode, press / rotate left / rotate right can all be assigned on the Keys page. The knob does not switch modes or change the volume."
            }
        }
        "dial_preset_note" => {
            if ja {
                "音量モード切り替えのときは、ノブを押したまま左右に回すとプリセット（レイヤー）が切り替わります（常駐モニタが必要です）。LEDはプリセットごとに効果が変わり、そのプリセットの間ずっとループします（1=点滅ループ、2=カラーサイクル、3=呼吸。本体に色を指定する機能はないため色そのものは変えられません）。キー割り当てモードでは使えません"
            } else {
                "In volume mode switch mode, hold the knob pressed and twist left/right to switch the preset (layer); the resident monitor must be running. The LED pattern changes per preset and keeps looping while that preset is active (1 = blinking loop, 2 = colour cycle, 3 = breathing — the device has no way to set a colour, so colours themselves cannot change). Not available in key assignment mode."
            }
        }
        "reconnect" => {
            if ja {
                "再接続"
            } else {
                "Reconnect"
            }
        }
        "err_not_found" => {
            if ja {
                "T1キーボードが見つかりません。USB を挿し直し、別のポートも試してください。"
            } else {
                "The T1 Keyboard was not found. Unplug it, try another port, and plug it in again."
            }
        }
        "err_permission" => {
            if ja {
                "T1キーボードにアクセスする権限がありません。端末で sudo ./install.sh を実行してから挿し直してください（.deb で入れた場合は再ログイン）。"
            } else {
                "This PC cannot open the T1 Keyboard. Run sudo ./install.sh, then replug it (log out and back in after a .deb install)."
            }
        }
        "err_busy" => {
            if ja {
                "他のアプリ（MINI KeyBoard.exe など）が T1キーボードを使っています。閉じてから［再接続］を押してください。"
            } else {
                "Another app (such as MINI KeyBoard.exe) is using the T1 Keyboard. Close it, then press Reconnect."
            }
        }
        "err_enum" => {
            if ja {
                "T1キーボードを列挙できませんでした。USB を挿し直してから［再接続］を押してください。"
            } else {
                "The T1 Keyboard could not be listed. Replug it, then press Reconnect."
            }
        }
        "err_open" => {
            if ja {
                "T1キーボードを開けませんでした。USB を挿し直してから［再接続］を押してください。"
            } else {
                "The T1 Keyboard could not be opened. Replug it, then press Reconnect."
            }
        }
        "err_claim" => {
            if ja {
                "T1キーボードの設定口を受け取れませんでした。他の T1-Customizer を閉じてから［再接続］を押してください。"
            } else {
                "Could not claim the T1 Keyboard config interface. Close other T1-Customizer windows, then press Reconnect."
            }
        }
        "err_probe" => {
            if ja {
                "T1キーボードには接続しましたが、設定の通信に失敗しました。挿し直してから［再接続］を押してください。"
            } else {
                "The T1 Keyboard opened, but the config channel failed. Replug it, then press Reconnect."
            }
        }
        "err_write" => {
            if ja {
                "書き込みに失敗しました。接続を確認してからもう一度試してください。"
            } else {
                "The write failed. Check the connection and try again."
            }
        }
        "err_connect" => {
            if ja {
                "T1キーボードに接続できませんでした。USB を確認してから［再接続］を押してください。"
            } else {
                "Could not connect to the T1 Keyboard. Check the USB cable, then press Reconnect."
            }
        }
        "warn_prefix" => {
            if ja {
                "警告"
            } else {
                "Warning"
            }
        }
        "none_paren" => {
            if ja {
                "（なし）"
            } else {
                "(none)"
            }
        }
        "lang_label" => {
            if ja {
                "言語"
            } else {
                "Language"
            }
        }
        "double_click_write" => {
            if ja {
                "リストのダブルクリックで書き込む"
            } else {
                "Write when a row is double-clicked"
            }
        }
        "write_undo" => {
            if ja {
                "元に戻す"
            } else {
                "Undo write"
            }
        }
        "write_undo_tip" => {
            if ja {
                "このセッションで書く直前の内容を、もう一度書き込みます"
            } else {
                "Write back the value from earlier in this session"
            }
        }
        "confirm_cancel" => {
            if ja {
                "キャンセル"
            } else {
                "Cancel"
            }
        }
        "write_all_body" => {
            if ja {
                "プリセット 1 の KEY1〜KEY6 を本体に上書きします。フラッシュは読み戻せません。"
            } else {
                "This overwrites KEY1–KEY6 of preset 1 on the keyboard. The flash cannot be read back."
            }
        }
        "disable_title" => {
            if ja {
                "キーを無効化"
            } else {
                "Disable key"
            }
        }
        "key_layout" => {
            if ja {
                "実機は上段 KEY1–3、下段 KEY4–6、その下にノブが 2 つです。"
            } else {
                "On the device: KEY1–3 on top, KEY4–6 below, then two knobs."
            }
        }
        "knobs" => {
            if ja {
                "ノブ"
            } else {
                "Knobs"
            }
        }
        "zoom" => {
            if ja {
                "表示倍率"
            } else {
                "Zoom"
            }
        }
        "help_heading" => {
            if ja {
                "キーの設定方法"
            } else {
                "How to program a key"
            }
        }
        "help_button" => {
            if ja {
                "ヘルプ"
            } else {
                "Help"
            }
        }
        "current_record" => {
            if ja {
                "現在の記録"
            } else {
                "Current record"
            }
        }
        "font_credit" => {
            if ja {
                "日本語は Windows の Yu Gothic UI / メイリオ、macOS のヒラギノ角ゴシック、Linux の Noto Sans CJK などを先に使います。見つからない字は、同梱の Noto Sans JP サブセットで描きます。韓国語（ハングル）は同梱の Noto Sans KR サブセットです。どちらも SIL Open Font License 1.1、© 2014-2021 Adobe、Reserved Font Name は Source です。"
            } else {
                "Japanese text prefers Yu Gothic UI, then Meiryo, on Windows, Hiragino Sans on macOS, and Noto Sans CJK on Linux. Missing glyphs use the bundled Noto Sans JP subset, and Korean (Hangul) uses the bundled Noto Sans KR subset. Both are under the SIL Open Font License 1.1, © 2014-2021 Adobe, Reserved Font Name: Source."
            }
        }
        "gray_keys" => {
            if ja {
                "このキーは選べません"
            } else {
                "This key cannot be selected"
            }
        }
        "font_in_use" => {
            if ja {
                "使用中のフォント"
            } else {
                "Font in use"
            }
        }
        "font_license" => {
            if ja {
                "Noto Sans JP / KR のライセンス (OFL 1.1)"
            } else {
                "Noto Sans JP / KR license (OFL 1.1)"
            }
        }
        _ => "",
    }
}

/// Languages of the window, in the order of the switch at the top.
pub const LANGS: [&str; 3] = ["ja", "en", "ko"];

/// The `lang` value of settings.json as a window language. One of
/// [`LANGS`]; anything else (missing included) is the default, Japanese.
pub fn ui_lang(raw: Option<&str>) -> &'static str {
    match raw {
        Some("en") => "en",
        Some("ko") => "ko",
        _ => "ja",
    }
}

/// Caption of the language button.
pub fn lang_button(lang: &str) -> &'static str {
    match lang {
        "ja" => "JA",
        "ko" => "KO",
        _ => "EN",
    }
}

pub fn pages(lang: &str) -> [&'static str; 7] {
    [
        tr(lang, "page_key"),
        tr(lang, "page_fun"),
        tr(lang, "page_media"),
        tr(lang, "page_mouse"),
        tr(lang, "page_led"),
        tr(lang, "page_dial"),
        tr(lang, "page_about"),
    ]
}

pub fn help_text(lang: &str) -> &'static str {
    if lang == "ko" {
        return super::i18n_ko::help_text();
    }
    if lang == "ja" {
        "1. 上部でプリセット(1～3)を選択。T1キーボード本体に保存できるキー割り当てのセット（1～3）\n\
         2. 左の物理キーを選択\n\
         3. 右のタブで割り当てる機能を選択\n\
         4. ボタンの文言を確認して「書き込み」を押す\n\
         リストのダブルクリックは、左の「リストのダブルクリックで書き込む」を入れたときだけ書き込みます。\n\n\
         修飾キー: 修飾を1つ以上と、任意のベースキーを選びます。\n\
         グループは最大5つまで追加できます。保存すると設定ファイルに残ります。\n\n\
         設定はキーボード本体のフラッシュに保存され、このツールでは読み戻せません。\n\
         「現在の設定内容」はこのツールが書き込んだ記録だけを表示します。\n\n\
         ダイヤルの音量操作は待受（モニター）が必要です。ウィンドウを閉じてもトレイに残ります。"
    } else {
        "1. Choose the preset (1-3) at the top. A set of key assignments stored on the T1 keyboard (1-3).\n\
         2. Pick a physical key on the left\n\
         3. Pick the function on the right tab\n\
         4. Check the button label and press Write\n\
         Double-clicking a list row writes only when the double-click option on the left is on.\n\n\
         Modifiers: pick one or more modifiers and any base key.\n\
         Up to 5 groups can be added. Saved combos stay in the settings file.\n\n\
         Settings are stored in the keyboard flash and cannot be read back.\n\
         The current assignment line shows only what this tool wrote.\n\n\
         Dial volume control needs the resident monitor. Closing the window leaves the app in the tray."
    }
}

pub fn about_text(lang: &str, version: &str, path: &str) -> String {
    if lang == "ko" {
        return super::i18n_ko::about_text(version, path);
    }
    if lang == "ja" {
        format!(
            "T1-Customizer {version}\n\
             T1キーボード (VID 1189 / PID 8890) の設定ツールです。無料で、全機能が使えます。\n\
             配布バイナリは Rust です。\n\n\
             自己診断: t1-keyboard-config --selftest\n\
             待受: t1-keyboard-config --monitor\n\
             設定ファイル: {path}"
        )
    } else {
        format!(
            "T1-Customizer {version}\n\
             Configuration tool for the T1 keyboard (VID 1189 / PID 8890). Free; every feature is available.\n\
             The shipped binary is Rust.\n\n\
             Self test: t1-keyboard-config --selftest\n\
             Listener:  t1-keyboard-config --monitor\n\
             Settings:  {path}"
        )
    }
}

pub fn connect_fault_text(lang: &str, code: &str) -> String {
    let key = fault_key(code);
    let text = tr(lang, key).to_string();
    if !matches!(key, "err_enum" | "err_open" | "err_probe" | "err_claim") {
        return text;
    }
    let Some((_, detail)) = code.split_once(':') else {
        return text;
    };
    let detail = detail.trim();
    if detail.is_empty()
        || detail.eq_ignore_ascii_case(
            "device busy - another t1-keyboard-config window/monitor is using it",
        )
    {
        return text;
    }
    format!("{text}\n{detail}")
}

fn fault_key(code: &str) -> &'static str {
    let lower = code.to_ascii_lowercase();
    if lower.starts_with("device-not-found") {
        "err_not_found"
    } else if lower.contains("busy") || lower.contains("close other") {
        "err_busy"
    } else if lower.starts_with("permission") {
        "err_permission"
    } else if lower.starts_with("enum-failed") {
        "err_enum"
    } else if lower.starts_with("open-failed") {
        "err_open"
    } else if lower.starts_with("claim-failed") {
        "err_claim"
    } else if lower.starts_with("probe-failed") {
        "err_probe"
    } else if lower.starts_with("write-failed") {
        "err_write"
    } else {
        "err_connect"
    }
}

pub fn learn_countdown(lang: &str, seconds: u64) -> String {
    if lang == "ja" {
        format!("{}（残り{seconds}秒）", tr(lang, "learning"))
    } else {
        format!("{} ({seconds}s left)", tr(lang, "learning"))
    }
}

pub fn wrote_line(lang: &str, target: &str, label: &str) -> String {
    if lang == "ja" {
        format!("{target} に「{label}」を書き込みました")
    } else {
        format!("Wrote \"{label}\" to {target}")
    }
}

pub fn write_button(lang: &str, target: &str, label: &str) -> String {
    if lang == "ja" {
        format!("{target} に「{label}」を書き込む")
    } else {
        format!("Write \"{label}\" to {target}")
    }
}

pub fn disable_body(lang: &str, key: u8, layer: u8) -> String {
    if lang == "ja" {
        format!("KEY{key}（プリセット{layer}）を無効にします。本体のフラッシュは読み戻せません。")
    } else {
        format!("Disable KEY{key} (preset {layer}). The keyboard flash cannot be read back.")
    }
}

pub fn dial_hint(lang: &str) -> &'static str {
    if lang == "ko" {
        return super::i18n_ko::dial_hint();
    }
    if lang == "ja" {
        "ダイヤル回転で音量を操作します。短く押すとモード（マイク／ヘッドホン）が切り替わります。"
    } else {
        "Dial rotation adjusts the volume. A short press switches the mode (microphone / headphones)."
    }
}

#[cfg(test)]
mod ui_text_tests {
    use super::*;

    /// Every `"key" =>` arm in a string table source.
    fn table_keys(source: &str) -> Vec<&str> {
        source
            .lines()
            .filter_map(|line| line.trim_start().strip_prefix('"'))
            .filter_map(|rest| rest.split_once("\" =>").map(|(key, _)| key))
            .filter(|key| !key.contains(' '))
            .collect()
    }

    fn all_table_keys() -> Vec<&'static str> {
        let mut keys: Vec<&'static str> = table_keys(include_str!("i18n.rs"))
            .into_iter()
            .filter(|key| !matches!(*key, "ja" | "ko"))
            .collect();
        keys.sort_unstable();
        keys.dedup();
        keys
    }

    fn has_hangul(text: &str) -> bool {
        text.chars().any(|c| ('\u{AC00}'..='\u{D7A3}').contains(&c))
    }

    /// Keys that are the same text in every language (codes, not words).
    fn language_neutral(key: &str) -> bool {
        tr("ja", key) == tr("en", key) && tr("ja", key).is_ascii()
    }

    #[test]
    fn korean_has_an_entry_for_every_key() {
        let keys = all_table_keys();
        assert!(keys.len() > 80, "found only {} keys", keys.len());
        let mut missing = Vec::new();
        for key in &keys {
            match super::super::i18n_ko::tr(key) {
                Some(ko) if !ko.is_empty() => {
                    if !language_neutral(key) {
                        assert!(has_hangul(ko), "{key} ko has no Hangul: {ko}");
                    }
                    assert_eq!(tr("ko", key), ko, "{key}");
                }
                _ => missing.push(*key),
            }
        }
        assert!(missing.is_empty(), "no Korean for {missing:?}");
    }

    #[test]
    fn no_removed_feature_wording() {
        for lang in LANGS {
            for key in all_table_keys() {
                let text = tr(lang, key);
                for bad in ["VRChat", "OSC", "activation", "trial", "Trial"] {
                    assert!(!text.contains(bad), "{lang} {key}: {text}");
                }
                // The font credit keys legitimately name the OFL license.
                if key != "font_license" && key != "font_credit" {
                    assert!(!text.contains("license"), "{lang} {key}: {text}");
                }
            }
            assert_eq!(pages(lang).len(), 7);
            for text in [help_text(lang), dial_hint(lang)] {
                assert!(
                    !text.contains("VRChat") && !text.contains("OSC"),
                    "{lang}: {text}"
                );
            }
        }
        assert_eq!(tr("ja", "app"), "T1-Customizer");
        assert_eq!(tr("en", "app"), "T1-Customizer");
        assert_eq!(tr("ja", "layer"), "プリセット");
        assert_eq!(tr("en", "layer"), "Preset");
        assert!(help_text("ja").starts_with("1. 上部でプリセット"));
        assert!(help_text("en").starts_with("1. Choose the preset"));
        assert!(about_text("en", "9.9.9", "/tmp/x").contains("9.9.9"));
    }

    #[test]
    fn window_languages_and_saved_choice() {
        assert_eq!(LANGS, ["ja", "en", "ko"]);
        assert_eq!(ui_lang(None), "ja");
        assert_eq!(ui_lang(Some("en")), "en");
        assert_eq!(ui_lang(Some("ko")), "ko");
        assert_eq!(ui_lang(Some("fr")), "ja");
        assert_eq!(LANGS.map(lang_button), ["JA", "EN", "KO"]);
        assert!(tr("ko", "font_credit").contains("Noto Sans KR"));
    }
}

#[cfg(test)]
mod tests {
    use super::connect_fault_text;

    #[test]
    fn connect_faults_are_translated() {
        let missing = connect_fault_text("ja", "device-not-found");
        assert!(missing.contains("見つかりません"));
        assert!(!missing.contains("device-not-found"));
        let denied = connect_fault_text("ja", "permission");
        assert!(denied.contains("install.sh"));
        let busy = connect_fault_text("en", "permission: close other programs using the T1");
        assert!(busy.contains("MINI KeyBoard"));
        let listed = connect_fault_text("ja", "enum-failed: access denied");
        assert!(listed.contains("列挙"));
        assert!(listed.contains("access denied"));
    }
}
