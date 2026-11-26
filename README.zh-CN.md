# T1-Customizer

**T1 迷你键盘**（[6 Keys 1 Knob RGB Programming Macro Gaming Keypad](https://ja.aliexpress.com/item/1005009812219099.html)，AliExpress，USB VID `1189` / PID `8890`）的免费配置工具。

> Forked from T1-VREmoKey (えもかえ) by AppWizz Kai, version 2.2.4.
> 本分支删除了原应用的全部 VRChat 相关功能和激活（许可证）限制。免费，所有功能均可使用。

[English](README.md) | [日本語](README.ja.md) | 简体中文 | [한국어](README.ko.md)

## 功能

- 按键分配（每个按键最多 5 组修饰键组合）
- 可命名的修饰键组合、多媒体键、鼠标动作、LED 模式
- 预设 1/2/3 切换并写入键盘本体
- 旋钮旋转调节系统音量（麦克风／耳机），按下旋钮切换模式；监听程序驻留托盘
- `settings.json` 在 Linux / Windows / macOS 间通用；原应用为本分支保留功能写入的设置可直接加载，已删除功能的设置会被忽略但保留在文件中
- 界面语言：日语、英语、韩语，保存在 `settings.json` 的 `lang` 中

## 不包含的内容

- 不包含任何 VRChat 功能（无 OSC、无虚拟形象配置、无表情热键、无 Unity 包）
- 无激活、许可证、试用或订单号限制
- 仅 Rust 实现，不包含旧的 Python 实现

## 状态

| 系统 | 状态 |
| --- | --- |
| Linux | 已实现；本分支尚未在真实 T1 上验证 |
| Windows 11 | 已实现；本分支尚未在真实 T1 上验证 |
| macOS | 已实现，**未验证**（无 macOS 设备） |

## 用法

```bash
t1-keyboard-config            # 配置 GUI
t1-keyboard-config --selftest # 设备、权限与写入路径诊断
t1-keyboard-config --watch    # 实时显示 T1 输入事件
t1-keyboard-config --monitor  # 旋钮音量监听
t1-keyboard-config --version
```

## 从源码构建

```bash
cd rust
cargo build --release
cargo test
```

## 许可证

MIT，见 `LICENSE`。
