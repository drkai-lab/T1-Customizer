//! T1 input, normalised to the app event types (`EV_KEY` = 2, `EV_REL` = 3).
//! Nothing grabs the keyboard.

#[cfg(any(windows, target_os = "macos"))]
use std::sync::mpsc::{self, Receiver, Sender};
#[cfg(any(windows, target_os = "macos"))]
use std::sync::{Arc, Mutex};
#[cfg(any(windows, target_os = "macos"))]
use std::time::{Duration, Instant};

#[cfg(any(windows, target_os = "macos"))]
use crate::state;
#[cfg(not(any(windows, target_os = "macos")))]
use crate::tables::normalize_kernel_type;
#[cfg(any(windows, target_os = "macos"))]
use crate::tables::{self, EV_KEY, EV_REL};

#[derive(Clone, Debug)]
pub struct InputEvent {
    pub etype: u16,
    pub code: u16,
    pub value: i32,
}

pub trait InputSource: Send {
    fn wait_events(&mut self, timeout: f64) -> Vec<InputEvent>;
    fn describe(&self) -> String;
}

pub fn open_input_source() -> Option<Box<dyn InputSource>> {
    #[cfg(windows)]
    {
        Some(Box::new(WindowsInput::start()))
    }
    #[cfg(target_os = "macos")]
    {
        return Some(Box::new(MacInput::start()));
    }
    #[cfg(target_os = "linux")]
    {
        LinuxInput::open().map(|src| Box::new(src) as Box<dyn InputSource>)
    }
    #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
    {
        None
    }
}

#[cfg(target_os = "linux")]
struct LinuxInput {
    devices: Vec<evdev::Device>,
    name: String,
}

#[cfg(target_os = "linux")]
impl LinuxInput {
    fn open() -> Option<Self> {
        let nodes = crate::transport::linux::t1_input_nodes();
        let mut devices = Vec::new();
        let mut names = Vec::new();
        for (path, _name) in nodes {
            let Ok(device) = evdev::Device::open(&path) else {
                continue;
            };
            if device.set_nonblocking(true).is_err() {
                continue;
            }
            names.push(
                std::path::Path::new(&path)
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or(path),
            );
            devices.push(device);
        }
        if devices.is_empty() {
            return None;
        }
        Some(Self {
            devices,
            name: names.join(", "),
        })
    }
}

#[cfg(target_os = "linux")]
impl InputSource for LinuxInput {
    fn wait_events(&mut self, timeout: f64) -> Vec<InputEvent> {
        use std::os::fd::AsRawFd;
        let mut pollfds: Vec<libc::pollfd> = self
            .devices
            .iter()
            .map(|dev| libc::pollfd {
                fd: dev.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            })
            .collect();
        if !pollfds.is_empty() {
            unsafe {
                libc::poll(
                    pollfds.as_mut_ptr(),
                    pollfds.len() as libc::nfds_t,
                    (timeout.max(0.0) * 1000.0) as i32,
                );
            }
        }
        let mut out = Vec::new();
        for device in &mut self.devices {
            let Ok(events) = device.fetch_events() else {
                continue;
            };
            for event in events {
                let Some(etype) = normalize_kernel_type(event.event_type().0) else {
                    continue;
                };
                out.push(InputEvent {
                    etype,
                    code: event.code(),
                    value: event.value(),
                });
            }
        }
        out
    }

    fn describe(&self) -> String {
        self.name.clone()
    }
}

#[cfg(any(windows, target_os = "macos"))]
struct QueueInput {
    rx: Receiver<InputEvent>,
    name: String,
    stop: Arc<std::sync::atomic::AtomicBool>,
}

#[cfg(any(windows, target_os = "macos"))]
impl InputSource for QueueInput {
    fn wait_events(&mut self, timeout: f64) -> Vec<InputEvent> {
        let mut out = Vec::new();
        let deadline = Instant::now() + Duration::from_secs_f64(timeout.max(0.0));
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() && !out.is_empty() {
                break;
            }
            match self
                .rx
                .recv_timeout(remaining.max(Duration::from_millis(1)))
            {
                Ok(event) => out.push(event),
                Err(_) => break,
            }
            if Instant::now() >= deadline {
                break;
            }
        }
        out
    }

    fn describe(&self) -> String {
        self.name.clone()
    }
}

#[cfg(windows)]
struct WindowsInput {
    inner: QueueInput,
}

#[cfg(windows)]
impl WindowsInput {
    fn start() -> Self {
        let (tx, rx) = mpsc::channel();
        let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
        if let Ok(mut slot) = RAW_TX.lock() {
            *slot = Some(tx);
        }
        ensure_raw_hub();
        Self {
            inner: QueueInput {
                rx,
                name: "raw-input".into(),
                stop,
            },
        }
    }
}

#[cfg(windows)]
impl InputSource for WindowsInput {
    fn wait_events(&mut self, timeout: f64) -> Vec<InputEvent> {
        self.inner.wait_events(timeout)
    }
    fn describe(&self) -> String {
        self.inner.describe()
    }
}

#[cfg(windows)]
impl Drop for WindowsInput {
    fn drop(&mut self) {
        self.inner
            .stop
            .store(true, std::sync::atomic::Ordering::Relaxed);
        if let Ok(mut slot) = RAW_TX.lock() {
            *slot = None;
        }
    }
}

/// One raw-input window per process for the T1 monitor (dial/keys).
/// A second
/// RegisterRawInputDevices call would steal the registration, so it is
/// started once and kept for the life of the process.
#[cfg(windows)]
fn ensure_raw_hub() {
    static HUB: std::sync::Once = std::sync::Once::new();
    HUB.call_once(|| {
        std::thread::spawn(windows_raw_loop);
    });
}

#[cfg(windows)]
fn windows_raw_loop() {
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::UI::Input::{RAWINPUTDEVICE, RegisterRawInputDevices};
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, HWND_MESSAGE, MSG,
        RegisterClassW, TranslateMessage, WINDOW_EX_STYLE, WINDOW_STYLE, WM_INPUT, WNDCLASSW,
    };
    use windows::core::w;

    unsafe extern "system" fn wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if msg == WM_INPUT {
            windows_on_raw(lparam);
            return LRESULT(0);
        }
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
    }

    unsafe {
        let class = WNDCLASSW {
            lpfnWndProc: Some(wnd_proc),
            lpszClassName: w!("T1KeyboardRawInput"),
            ..Default::default()
        };
        if RegisterClassW(&class) == 0 {
            state::log_line("windows input backend failed: RegisterClassW");
            return;
        }
        let Ok(hwnd) = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("T1KeyboardRawInput"),
            w!("t1"),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            None,
            None,
        ) else {
            return;
        };
        let devices = [
            RAWINPUTDEVICE {
                usUsagePage: 0x01,
                usUsage: 0x06,
                dwFlags: windows::Win32::UI::Input::RIDEV_INPUTSINK,
                hwndTarget: hwnd,
            },
            RAWINPUTDEVICE {
                usUsagePage: 0x01,
                usUsage: 0x02,
                dwFlags: windows::Win32::UI::Input::RIDEV_INPUTSINK,
                hwndTarget: hwnd,
            },
        ];
        if RegisterRawInputDevices(&devices, std::mem::size_of::<RAWINPUTDEVICE>() as u32).is_err()
        {
            return;
        }
        let mut msg = MSG::default();
        loop {
            let result = GetMessageW(&mut msg, None, 0, 0);
            if !result.as_bool() {
                break;
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

#[cfg(windows)]
fn windows_on_raw(lparam: windows::Win32::Foundation::LPARAM) {
    use windows::Win32::UI::Input::{
        GetRawInputData, GetRawInputDeviceInfoW, HRAWINPUT, RAWINPUTHEADER, RID_INPUT,
        RIDI_DEVICENAME,
    };
    unsafe {
        let header_size = std::mem::size_of::<RAWINPUTHEADER>() as u32;
        let mut size = 0u32;
        let raw = HRAWINPUT(lparam.0 as *mut _);
        GetRawInputData(raw, RID_INPUT, None, &mut size, header_size);
        if size == 0 {
            return;
        }
        let mut buf = vec![0u8; size as usize];
        if GetRawInputData(
            raw,
            RID_INPUT,
            Some(buf.as_mut_ptr() as *mut _),
            &mut size,
            header_size,
        ) == u32::MAX
        {
            return;
        }
        if buf.len() < header_size as usize + 8 {
            return;
        }
        let header = &*(buf.as_ptr() as *const RAWINPUTHEADER);
        if !raw_is_t1(header.hDevice) {
            return;
        }
        let body = header_size as usize;
        let vk = tables::vk_to_evdev();
        if header.dwType == 0 {
            let flags = u16::from_le_bytes([buf[body + 2], buf[body + 3]]);
            let vkey = u16::from_le_bytes([buf[body + 6], buf[body + 7]]);
            let Some(code) = vk.get(&vkey).copied() else {
                return;
            };
            let value = if flags & 0x0001 != 0 { 0 } else { 1 };
            let _ = RAW_TX
                .lock()
                .ok()
                .and_then(|tx| tx.as_ref().cloned())
                .map(|tx| {
                    tx.send(InputEvent {
                        etype: EV_KEY,
                        code,
                        value,
                    })
                });
        } else if header.dwType == 1 {
            let flags = u16::from_le_bytes([buf[body + 4], buf[body + 5]]);
            let data = i16::from_le_bytes([buf[body + 6], buf[body + 7]]);
            push_mouse(flags, data);
        }
        let _ = GetRawInputDeviceInfoW;
        let _ = RIDI_DEVICENAME;
    }
}

#[cfg(windows)]
static RAW_TX: Mutex<Option<Sender<InputEvent>>> = Mutex::new(None);

#[cfg(windows)]
fn raw_is_t1(device: windows::Win32::Foundation::HANDLE) -> bool {
    use windows::Win32::UI::Input::{GetRawInputDeviceInfoW, RIDI_DEVICENAME};
    unsafe {
        let mut length = 0u32;
        GetRawInputDeviceInfoW(Some(device), RIDI_DEVICENAME, None, &mut length);
        if length == 0 {
            return false;
        }
        let mut buf = vec![0u16; length as usize];
        if GetRawInputDeviceInfoW(
            Some(device),
            RIDI_DEVICENAME,
            Some(buf.as_mut_ptr() as *mut _),
            &mut length,
        ) == u32::MAX
        {
            return false;
        }
        let text = String::from_utf16_lossy(&buf).to_ascii_lowercase();
        text.contains("vid_1189") && text.contains("pid_8890")
    }
}

#[cfg(windows)]
fn push_mouse(flags: u16, data: i16) {
    let Ok(slot) = RAW_TX.lock() else {
        return;
    };
    let Some(tx) = slot.as_ref() else {
        return;
    };
    for (bit, code) in [(0x0001u16, 272u16), (0x0004, 273), (0x0010, 274)] {
        if flags & bit != 0 {
            let _ = tx.send(InputEvent {
                etype: EV_KEY,
                code,
                value: 1,
            });
        }
        if flags & (bit << 1) != 0 {
            let _ = tx.send(InputEvent {
                etype: EV_KEY,
                code,
                value: 0,
            });
        }
    }
    if flags & 0x0400 != 0 {
        let _ = tx.send(InputEvent {
            etype: EV_REL,
            code: 8,
            value: if data > 0 { 1 } else { -1 },
        });
    }
    if flags & 0x0800 != 0 {
        let _ = tx.send(InputEvent {
            etype: EV_REL,
            code: 6,
            value: if data > 0 { 1 } else { -1 },
        });
    }
}

#[cfg(target_os = "macos")]
struct MacInput {
    inner: QueueInput,
}

#[cfg(target_os = "macos")]
impl MacInput {
    fn start() -> Self {
        let (tx, rx) = mpsc::channel();
        let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
        std::thread::spawn(move || {
            if let Err(err) = mac_hid_input(tx) {
                state::log_line(&format!("macos input backend failed: {err}"));
            }
        });
        Self {
            inner: QueueInput {
                rx,
                name: "IOHIDDevice".into(),
                stop,
            },
        }
    }
}

#[cfg(target_os = "macos")]
impl InputSource for MacInput {
    fn wait_events(&mut self, timeout: f64) -> Vec<InputEvent> {
        self.inner.wait_events(timeout)
    }
    fn describe(&self) -> String {
        self.inner.describe()
    }
}

#[cfg(target_os = "macos")]
fn mac_hid_input(tx: Sender<InputEvent>) -> Result<(), String> {
    let api = hidapi::HidApi::new().map_err(|err| err.to_string())?;
    let mut devices = Vec::new();
    for info in api.device_list() {
        if info.vendor_id() == crate::VID && info.product_id() == crate::PID {
            if let Ok(dev) = info.open_device(&api) {
                let _ = dev.set_blocking_mode(false);
                devices.push(dev);
            }
        }
    }
    let usage = tables::hid_usage_to_evdev();
    let consumer = tables::consumer_to_evdev();
    while !devices.is_empty() {
        for device in &devices {
            let mut buf = [0u8; 64];
            match device.read_timeout(&mut buf, 50) {
                Ok(n) if n > 2 => {
                    // Best-effort boot-keyboard style report. Owner hardware confirms the layout.
                    let mods = buf[0];
                    if mods != 0 {
                        let _ = tx.send(InputEvent {
                            etype: EV_KEY,
                            code: 42,
                            value: 1,
                        });
                    }
                    for byte in buf.iter().skip(2).take(6).copied().filter(|b| *b != 0) {
                        if let Some(code) = usage.get(&u16::from(byte)).copied() {
                            let _ = tx.send(InputEvent {
                                etype: EV_KEY,
                                code,
                                value: 1,
                            });
                        } else if let Some(code) = consumer.get(&u16::from(byte)).copied() {
                            let _ = tx.send(InputEvent {
                                etype: EV_KEY,
                                code,
                                value: 1,
                            });
                        }
                    }
                }
                _ => {}
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    Ok(())
}
