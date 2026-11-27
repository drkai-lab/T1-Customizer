//! Korean (한국어) window text.
//!
//! Every `tr` key in `i18n.rs` has a Korean entry here, checked by
//! `korean_has_an_entry_for_every_key`. Hangul is drawn with the bundled
//! Noto Sans KR subset (see `fonts.rs`), which is generated from the
//! characters of this file, so `scripts/subset-noto-sans-kr.py` must be
//! re-run when new Hangul syllables outside KS X 1001 are added.

/// Korean for any `tr` key. `None` when the key has no Korean entry.
pub fn tr(key: &str) -> Option<&'static str> {
    window(key)
}

fn window(key: &str) -> Option<&'static str> {
    Some(match key {
        "app" => "T1-Customizer",
        "app_presets" => "앱 프리셋",
        "app_preset_applied" => "프리셋을 썼습니다",
        "app_preset_partial" => "프리셋 일부만 썼습니다 ({ok}개)",
        "app_preset_knob" => "노브 회전도 썼습니다",
        "app_preset_knob_skip" => "노브 회전은 키 할당 모드일 때만 쓸 수 있습니다",
        "chords_mode_label" => "키 페어 누르기 모드",
        "chords_on" => "켬",
        "chords_off" => "끔",
        "chords_note" => {
            "켜져 있는 동안 페어 대상 키에는 표식(Ctrl+Shift+F1~F6)만 기록하고, 동시 누름 판정은 상주 앱이 합니다. 단일 누름은 판정 간격(기본 100ms)만큼 늦어질 수 있습니다. 표식은 현재 프리셋에만 기록됩니다."
        }
        "chords_assign_hint" => {
            "페어를 선택한 상태에서 키 목록이나 수식 페이지에서 조합을 만들어 쓰면 그 페어에 할당됩니다."
        }
        "chords_single_first" => "먼저 단일 키를 할당하세요: {keys}",
        "chords_burn_pending" => {
            "본체가 연결되지 않아 표식 기록이 보류 중입니다. 연결한 뒤 페어를 다시 할당하세요."
        }
        "chords_restore_pending" => {
            "본체가 연결되지 않아 단일 키를 되돌리지 못했습니다. 키는 표식 그대로입니다. 연결한 뒤 다시 끄세요."
        }
        "chords_pair_saved" => "페어 {pair}에 {desc}를 할당했습니다",
        "config_save" => "설정 저장",
        "config_load" => "설정 불러오기",
        "config_saved" => "설정을 저장했습니다: {path}",
        "config_save_empty" => "저장할 버튼 설정이 아직 없습니다",
        "config_save_failed" => "설정을 저장하지 못했습니다",
        "config_load_bad" => "설정 파일을 읽지 못했습니다",
        "config_loaded_pending" => "설정을 불러왔습니다. 본체가 연결되지 않아 아직 쓰지 않았습니다",
        "config_loaded_counts" => {
            "설정을 불러왔습니다 (쓰기 {ok}건, 실패 {failed}건, 건너뜀 {skipped}건)"
        }
        "chords_single_only" => "페어 모드에서는 KEY1~KEY6에 단일 조합만 쓸 수 있습니다",
        "chords_keys_only" => "페어 모드에서는 KEY1~KEY6에 미디어/마우스를 쓸 수 없습니다",
        "chords_on_done" => "키 페어 누르기 모드를 켰습니다",
        "chords_off_done" => "키 페어 누르기 모드를 끄고 단일 키를 되돌렸습니다",
        "connected" => "연결됨",
        "disconnected" => "연결 안 됨",
        "download_ok" => "쓰기 성공",
        "download_ng" => "쓰기 실패",
        "download" => "키보드에 쓰기",
        "clear" => "지우기",
        "disable_key" => "이 키 비활성화",
        "page_key" => "키",
        "page_fun" => "수정자 키",
        "page_media" => "멀티미디어",
        "page_mouse" => "마우스",
        "page_dial" => "다이얼",
        "page_about" => "도움말",
        "layer" => "프리셋",
        "gray_keys" => "이 키는 선택할 수 없습니다",
        "layer_hint" => {
            "T1 키보드 본체에 저장할 수 있는 키 할당 세트(1~3). 표정은 프리셋 1에 씁니다"
        }
        "selected" => "선택된 키",
        "current" => "현재 할당 내용(이 도구가 쓴 기록)",
        "no_key" => "없음",
        "pick_first" => "먼저 왼쪽의 물리 키를 선택해 주세요",
        "pick_function" => "기능을 선택한 뒤 '쓰기'를 눌러 주세요",
        "need_device" => "연결되어 있지 않습니다",
        "learn_key" => "키 학습",
        "learn_push" => "누르기 학습",
        "learn_rotate" => "회전 학습",
        "learning" => "지금 T1 키보드의 키를 눌러 주세요(5초)...",
        "learn_done" => "학습 완료",
        "learn_fail" => "입력을 받지 못했습니다",
        "assign_empty" => "미설정",
        "mod_base" => "기본 키",
        "mod_only" => "(수정자 키만)",
        "mod_add" => "+ 그룹 추가",
        "mod_reset" => "초기화",
        "mod_name" => "이름",
        "mod_save" => "저장",
        "mod_delete" => "삭제",
        "mod_saved" => "저장된 조합",
        "mod_groups" => "그룹",
        "mod_saved_ok" => "조합을 저장했습니다",
        "mod_deleted" => "조합을 삭제했습니다",
        "mod_need_name" => "이 조합의 이름을 입력해 주세요",
        "mod_need_combo" => "수정자 키 또는 기본 키를 선택해 주세요",
        "mod_need_base" => "그룹을 추가하려면 기본 키를 선택해 주세요",
        "mod_lone_ready" => "수정자 키만 설정되어 있습니다. 저장 또는 쓰기를 눌러 주세요",
        "mod_group_full" => "키 1개당 최대 5개까지 그룹을 넣을 수 있습니다",
        "mod_pick_saved" => "먼저 저장된 조합을 선택해 주세요",
        "monitor_running" => "감시 중",
        "monitor_stopped" => "감시 중지됨",
        "start_monitor" => "감시 시작",
        "stop_monitor" => "감시 중지",
        "monitor_error" => "감시를 시작할 수 없습니다",
        "tray_open" => "화면 열기",
        "tray_start" => "대기 시작",
        "tray_stop" => "대기 중지",
        "tray_quit" => "종료",
        "tray_title" => "트레이 메뉴",
        "font_credit" => {
            "일본어는 Windows의 Yu Gothic UI / Meiryo, macOS의 Hiragino Sans, Linux의 Noto Sans CJK 등을 먼저 씁니다. 없는 글자는 동봉한 Noto Sans JP 서브셋으로 그립니다. 한국어는 동봉한 Noto Sans KR 서브셋(KS X 1001 한글 2,350자)으로 그립니다. 두 글꼴 모두 SIL Open Font License 1.1, © 2014-2021 Adobe, Reserved Font Name: Source입니다."
        }
        "font_in_use" => "사용 중인 글꼴",
        "font_license" => "Noto Sans JP / KR 라이선스 (OFL 1.1)",
        "page_led" => "LED",
        "dial_mode_mic" => "마이크 볼륨",
        "dial_mode_headphone" => "이어머프 볼륨",
        "dial_press_label" => "노브 누르기",
        "dial_press_cycle" => "볼륨 모드 전환",
        "dial_press_key" => "키 할당",
        "dial_press_note" => {
            "키 할당에서는 누르기 / 왼쪽 돌리기 / 오른쪽 돌리기를 모두 키 화면에서 할당할 수 있습니다. 노브는 모드 전환도 볼륨 변경도 하지 않습니다"
        }
        "dial_preset_note" => {
            "볼륨 모드 전환에서는 노브를 누른 채로 좌우로 돌리면 프리셋(레이어)이 전환됩니다(상주 모니터가 필요합니다). LED는 프리셋마다 효과가 바뀝니다(1=점등, 2=컬러 사이클, 3=호흡. 본체에는 색을 지정하는 기능이 없어 색 자체는 바꿀 수 없습니다). 키 할당 모드에서는 사용할 수 없습니다"
        }
        "help_heading" => "키 설정 방법",
        "addr_invalid" => "주소는 /로 시작해야 합니다",
        "confirm_cancel" => "취소",
        "current_record" => "현재 기록",
        "disable_title" => "키 비활성화",
        "double_click_write" => "리스트를 더블클릭하면 씁니다",
        "err_busy" => {
            "다른 앱(MINI KeyBoard.exe 등)이 T1 키보드를 사용 중입니다. 종료 후［재접속］을 눌러주세요."
        }
        "err_claim" => {
            "T1 키보드 설정 인터페이스를 확보하지 못했습니다. 다른 T1-Customizer 창을 닫고［재접속］을 눌러주세요."
        }
        "err_connect" => "T1 키보드에 연결되지 않았습니다. USB를 확인하고［재접속］을 눌러주세요.",
        "err_enum" => "T1 키보드를 목록화하지 못했습니다. USB를 다시 꽂고［재접속］을 눌러주세요.",
        "err_not_found" => "T1 키보드를 찾을 수 없습니다. USB를 빼고 다른 포트도 시도해주세요.",
        "err_open" => "T1 키보드를 열 수 없습니다. USB를 다시 꽂고［재접속］을 눌러주세요.",
        "err_permission" => {
            "T1 키보드에 접근 권한이 없습니다. 단말에서 sudo ./install.sh를 실행한 후 다시 꽂아주세요(.deb로 설치한 경우 재로그인)."
        }
        "err_probe" => {
            "T1 키보드에 연결되었지만 설정 통신에 실패했습니다. 다시 꽂고［재접속］을 눌러주세요."
        }
        "err_write" => "쓰기 실패했습니다. 연결을 확인하고 다시 시도하세요.",
        "help_button" => "도움말",
        "host_invalid" => "호스트 이름을 입력하세요(공백 불가)",
        "key_layout" => "실물은 상단 KEY1–3, 하단 KEY4–6, 그 아래에 노브가 2개입니다.",
        "knobs" => "노브",
        "lang_label" => "언어",
        "learn_no_device" => {
            "입력 장치를 읽을 수 없습니다(input 그룹 / udev 규칙). README「자기 진단」을 참조하세요."
        }
        "learn_timeout" => "5초 이내에 키가 눌리지 않았습니다. 다시［학습］을 눌러주세요.",
        "none_paren" => "(없음)",
        "port_invalid" => "1~65535 사이의 숫자를 입력하세요",
        "reconnect" => "재접속",
        "warn_prefix" => "경고",
        "write_all_body" => {
            "프리셋 1의 KEY1~KEY6를 본체에 덮어씁니다. 플래시는 다시 읽을 수 없습니다."
        }
        "write_need_combo" => "수정자나 베이스 키를 선택한 후 쓰기",
        "write_unavailable" => "이 탭에는 키보드에 쓸 항목이 없습니다",
        "write_undo" => "원래대로 되돌리기",
        "write_undo_tip" => "이 세션에서 쓰기 전의 내용을 다시 씁니다",
        "zoom" => "표시 배율",
        _ => return None,
    })
}

pub fn help_text() -> &'static str {
    "1. 위쪽에서 프리셋(1~3)을 고릅니다. T1 키보드 본체에 저장할 수 있는 키 할당 세트(1~3)입니다\n\
     2. 왼쪽에서 물리 키를 고릅니다\n\
     3. 오른쪽 탭에서 할당할 기능을 고릅니다\n\
     4. 버튼 문구를 확인하고 '쓰기'를 누릅니다\n\
     \n\
     수정자 키: 수정자를 하나 이상 고르고, 필요하면 기본 키를 고릅니다.\n\
     그룹은 최대 5개까지 추가할 수 있습니다. 저장하면 설정 파일에 남습니다.\n\
     \n\
     설정은 키보드 본체의 플래시에 저장되며, 이 도구로 다시 읽어 올 수 없습니다.\n\
     '현재 할당 내용'은 이 도구가 쓴 기록만 보여 줍니다.\n\
     \n\
     다이얼의 볼륨 조작은 감시가 필요합니다. 창을 닫아도 트레이에 남습니다."
}

pub fn about_text(version: &str, path: &str) -> String {
    format!(
        "T1-Customizer {version}\n\
     T1 키보드(VID 1189 / PID 8890)의 설정 도구입니다. 무료이며 모든 기능을 사용할 수 있습니다.\n\
     배포 바이너리는 Rust입니다.\n\
     \n\
     자가 진단: t1-keyboard-config --selftest\n\
     감시: t1-keyboard-config --monitor\n\
     설정 파일: {path}"
    )
}

pub fn dial_hint() -> &'static str {
    "다이얼을 돌려 볼륨을 조작합니다. 짧게 누르면 모드(마이크/헤드폰)가 바뀝니다."
}
