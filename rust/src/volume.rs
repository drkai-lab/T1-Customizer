//! Default sink / source volume. Linux uses wpctl. Windows tries the endpoint
//! volume interface and falls back to the media keys. macOS uses osascript.

use std::process::Command;

pub fn which_wpctl() -> bool {
    Command::new("wpctl")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
        || which("wpctl")
}

fn which(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn set_volume(kind: &str, delta: Option<f64>, absolute: Option<f64>) -> bool {
    #[cfg(windows)]
    {
        windows_volume(kind, delta, absolute)
    }
    #[cfg(target_os = "macos")]
    {
        return macos_volume(kind, delta, absolute);
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let target = if kind == "sink" {
            "@DEFAULT_AUDIO_SINK@"
        } else {
            "@DEFAULT_AUDIO_SOURCE@"
        };
        wpctl_volume(target, delta, absolute)
    }
}

pub fn wpctl_volume(target: &str, delta: Option<f64>, absolute: Option<f64>) -> bool {
    if !which("wpctl") {
        return false;
    }
    let value = if let Some(delta) = delta {
        let output = Command::new("wpctl")
            .args(["get-volume", target])
            .output()
            .ok();
        let Some(output) = output else {
            return false;
        };
        let text = String::from_utf8_lossy(&output.stdout);
        let Some(current) = text.split(':').nth(1).and_then(|rest| {
            rest.split_whitespace()
                .next()
                .and_then(|n| n.parse::<f64>().ok())
        }) else {
            return false;
        };
        (current + delta).clamp(0.0, 2.0)
    } else if let Some(absolute) = absolute {
        absolute.clamp(0.0, 2.0)
    } else {
        return false;
    };
    Command::new("wpctl")
        .args(["set-volume", target, &format!("{value:.2}")])
        .output()
        .is_ok()
}

#[cfg(windows)]
fn windows_volume(kind: &str, delta: Option<f64>, absolute: Option<f64>) -> bool {
    if windows_core_volume(kind, delta, absolute) {
        return true;
    }
    if let Some(delta) = delta {
        if delta == 0.0 {
            return false;
        }
        unsafe {
            let key = if delta > 0.0 { 0xAFu8 } else { 0xAEu8 };
            windows::Win32::UI::Input::KeyboardAndMouse::keybd_event(
                key,
                0,
                windows::Win32::UI::Input::KeyboardAndMouse::KEYBD_EVENT_FLAGS(0),
                0,
            );
            windows::Win32::UI::Input::KeyboardAndMouse::keybd_event(
                key,
                0,
                windows::Win32::UI::Input::KeyboardAndMouse::KEYEVENTF_KEYUP,
                0,
            );
        }
        return true;
    }
    false
}

#[cfg(windows)]
fn windows_core_volume(kind: &str, delta: Option<f64>, absolute: Option<f64>) -> bool {
    use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
    use windows::Win32::Media::Audio::{
        IMMDeviceEnumerator, MMDeviceEnumerator, eCapture, eMultimedia, eRender,
    };
    use windows::Win32::System::Com::{
        CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
    };
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let Ok(enumerator) =
            CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL)
        else {
            return false;
        };
        let flow = if kind == "source" { eCapture } else { eRender };
        let Ok(device) = enumerator.GetDefaultAudioEndpoint(flow, eMultimedia) else {
            return false;
        };
        let Ok(volume) = device.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None) else {
            return false;
        };
        if let Some(absolute) = absolute {
            return volume
                .SetMasterVolumeLevelScalar(absolute.clamp(0.0, 1.0) as f32, std::ptr::null())
                .is_ok();
        }
        let Some(delta) = delta else {
            return false;
        };
        let Ok(level) = volume.GetMasterVolumeLevelScalar() else {
            return false;
        };
        volume
            .SetMasterVolumeLevelScalar(
                (f64::from(level) + delta).clamp(0.0, 1.0) as f32,
                std::ptr::null(),
            )
            .is_ok()
    }
}

#[cfg(target_os = "macos")]
fn macos_volume(kind: &str, delta: Option<f64>, absolute: Option<f64>) -> bool {
    let target = if kind == "source" {
        "input volume"
    } else {
        "output volume"
    };
    let value = if let Some(absolute) = absolute {
        (absolute.clamp(0.0, 1.0) * 100.0).round() as i32
    } else if let Some(delta) = delta {
        let current = Command::new("osascript")
            .args(["-e", &format!("get {target} of (get volume settings)")])
            .output()
            .ok()
            .and_then(|out| {
                String::from_utf8_lossy(&out.stdout)
                    .trim()
                    .parse::<f64>()
                    .ok()
            })
            .unwrap_or(0.0);
        (current + delta * 100.0).round() as i32
    } else {
        return false;
    };
    let value = value.clamp(0, 100);
    Command::new("osascript")
        .args(["-e", &format!("set volume {target} {value}")])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
