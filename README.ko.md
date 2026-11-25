# T1-Customizer

**T1 미니 키보드**([6 Keys 1 Knob RGB Programming Macro Gaming Keypad](https://ja.aliexpress.com/item/1005009812219099.html), AliExpress, USB VID `1189` / PID `8890`)의 무료 설정 도구입니다.

> Forked from T1-VREmoKey (えもかえる) by AppWizz Kai, version 2.2.4.
> 이 포크는 원본 앱의 VRChat 관련 기능과 활성화(라이선스) 제한을 모두 제거했습니다. 무료이며 모든 기능을 사용할 수 있습니다.

[English](README.md) | [日本語](README.ja.md) | [简体中文](README.zh-CN.md) | 한국어

## 기능

- 키 할당(키마다 수정자 조합 최대 5그룹)
- 이름 붙은 수정자 조합, 멀티미디어, 마우스, LED
- 프리셋 1/2/3 전환과 본체 쓰기
- 다이얼 회전으로 OS 볼륨(마이크/헤드폰) 조작, 누르면 모드 전환. 감시는 트레이에 남습니다
- `settings.json`은 Linux / Windows / macOS 공통. 원본 앱이 쓴 설정 중 이 포크에 남은 기능의 것은 그대로 불러옵니다. 제거된 기능의 설정은 무시되지만 파일에서 지워지지 않습니다
- 화면 언어는 일본어/영어/한국어, `settings.json`의 `lang`에 저장됩니다

## 없는 것

- VRChat 관련 기능 일체(OSC, 아바타 프로필, 표정 핫키, Unity 패키지 없음)
- 활성화, 라이선스, 체험판, 주문 번호 제한 없음
- Rust만 포함합니다. 옛 Python 구현은 포함하지 않습니다

## 상태

| OS | 상태 |
| --- | --- |
| Linux | 구현됨. 이 포크에서는 실기 미확인 |
| Windows 11 | 구현됨. 이 포크에서는 실기 미확인 |
| macOS | 구현됨, **미검증**(macOS 기기가 없음) |

## 사용법

```bash
t1-keyboard-config            # 설정 GUI
t1-keyboard-config --selftest # 기기, 권한, 쓰기 경로 진단
t1-keyboard-config --watch    # T1 입력 이벤트 실시간 표시
t1-keyboard-config --monitor  # 다이얼 볼륨 감시
t1-keyboard-config --version
```

## 소스에서 빌드

```bash
cd rust
cargo build --release
cargo test
```

## 라이선스

MIT. `LICENSE`를 보세요.
