//! T1 input, normalised to the app event types (`EV_KEY` = 2, `EV_REL` = 3).
//! Nothing grabs the keyboard.

#[cfg(any(windows, target_os = "macos"))]
use std::sync::mpsc::{self, Receiver, Sender};
#[cfg(any(windows, target_os = "macos"))]
use std::sync::{Arc, Mutex};
#[cfg(any(windows, target_os = "macos"))]
use std::time::{Duration, Instant};

#[cfg(any(windows, target_os = "macos", target_os = "linux"))]
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

// ---------------------------------------------------------------------------
// Pair-chord interception (see `pair_chord.rs`). While pair mode is on,
// the resident needs to *swallow* the T1's sentinel keys, which the
// listen-only sources above cannot do: Linux grabs the T1 and re-emits
// everything through a uinput virtual device; Windows uses a low-level
// keyboard hook plus SendInput. macOS has no pair-chord path yet.

/// One raw kernel input event (Linux types: SYN=0, KEY=1, REL=2, MSC=4),
/// before the normalisation the monitor loop applies.
#[cfg(target_os = "linux")]
#[derive(Clone, Copy, Debug)]
pub struct RawKernelEvent {
    pub etype: u16,
    pub code: u16,
    pub value: i32,
}

#[cfg(target_os = "linux")]
mod uinput_sys {
    // ioctl numbers for /dev/uinput (asm-generic, same on x86_64/aarch64).
    pub const UI_SET_EVBIT: libc::c_ulong = 0x4004_5564;
    pub const UI_SET_KEYBIT: libc::c_ulong = 0x4004_5565;
    pub const UI_SET_RELBIT: libc::c_ulong = 0x4004_5566;
    pub const UI_SET_MSCBIT: libc::c_ulong = 0x4004_5569;
    pub const UI_DEV_SETUP: libc::c_ulong = 0x405C_5503;
    pub const UI_DEV_CREATE: libc::c_ulong = 0x5501;
    pub const UI_DEV_DESTROY: libc::c_ulong = 0x5502;
    pub const EV_KEY: i32 = 1;
    pub const EV_REL: i32 = 2;
    pub const EV_MSC: i32 = 4;
    pub const MSC_SCAN: i32 = 4;
    pub const KEY_MAX: i32 = 0x2FF;
    pub const REL_MAX: i32 = 0x0F;
}

/// The T1's input devices, grabbed exclusively, with a uinput virtual
/// device that everything is re-emitted through. Dropping this releases
/// the grab and destroys the virtual device.
#[cfg(target_os = "linux")]
pub struct GrabbedT1 {
    devices: Vec<evdev::Device>,
    name: String,
    uinput_fd: i32,
}

#[cfg(target_os = "linux")]
impl GrabbedT1 {
    pub fn open() -> Option<Self> {
        let nodes = crate::transport::linux::t1_input_nodes();
        let mut devices = Vec::new();
        let mut names = Vec::new();
        for (path, _name) in nodes {
            let Ok(mut device) = evdev::Device::open(&path) else {
                continue;
            };
            if device.set_nonblocking(true).is_err() || device.grab().is_err() {
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
        let uinput_fd = unsafe { create_uinput() };
        if uinput_fd < 0 {
            state::log_line("pair chords: /dev/uinput unavailable, cannot grab the T1");
            return None; // devices drop here, releasing the grab
        }
        Some(Self {
            devices,
            name: names.join(", "),
            uinput_fd,
        })
    }

    pub fn describe(&self) -> String {
        format!("{} (grabbed)", self.name)
    }

    /// Raw events from every grabbed device, in arrival order per device.
    pub fn poll(&mut self, timeout: f64) -> Vec<RawKernelEvent> {
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
                out.push(RawKernelEvent {
                    etype: event.event_type().0,
                    code: event.code(),
                    value: event.value(),
                });
            }
        }
        out
    }

    /// Re-emit one event through the virtual device (forwarding or an
    /// injected key). Callers add [`Self::sync`] to close a frame.
    pub fn emit(&mut self, etype: u16, code: u16, value: i32) {
        let mut buf = [0u8; 24];
        buf[16..18].copy_from_slice(&etype.to_le_bytes());
        buf[18..20].copy_from_slice(&code.to_le_bytes());
        buf[20..24].copy_from_slice(&value.to_le_bytes());
        unsafe {
            libc::write(
                self.uinput_fd,
                buf.as_ptr() as *const libc::c_void,
                buf.len(),
            );
        }
    }

    pub fn emit_key(&mut self, code: u16, down: bool) {
        self.emit(1, code, if down { 1 } else { 0 });
    }

    pub fn sync(&mut self) {
        self.emit(0, 0, 0);
    }
}

#[cfg(target_os = "linux")]
impl Drop for GrabbedT1 {
    fn drop(&mut self) {
        unsafe {
            libc::ioctl(self.uinput_fd, uinput_sys::UI_DEV_DESTROY);
            libc::close(self.uinput_fd);
        }
    }
}

/// Hand the newest pair-chord config to the Windows hook thread,
/// starting it on first use. While the config is disabled (or no config
/// was ever set) the hook passes every key through untouched.
#[cfg(windows)]
pub fn pair_chord_set_config(cfg: crate::pair_chord::PairChordConfig) {
    let generation = PAIR_CFG_GEN.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
    if let Ok(mut slot) = PAIR_CFG.lock() {
        *slot = Some((generation, cfg));
    }
    if !PAIR_HOOK_STARTED.swap(true, std::sync::atomic::Ordering::SeqCst) {
        std::thread::spawn(pair_hook_loop);
    }
}

#[cfg(windows)]
static PAIR_CFG: Mutex<Option<(u64, crate::pair_chord::PairChordConfig)>> = Mutex::new(None);
#[cfg(windows)]
static PAIR_CFG_GEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
#[cfg(windows)]
static PAIR_HOOK_STARTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
#[cfg(windows)]
static PAIR_HOOK_HANDLE: std::sync::atomic::AtomicPtr<std::ffi::c_void> =
    std::sync::atomic::AtomicPtr::new(std::ptr::null_mut());

/// State owned by the pair hook thread (the low-level hook callback and
/// the timer both run there).
#[cfg(windows)]
struct PairHookState {
    engine: Option<crate::pair_chord::PairChordEngine>,
    generation: u64,
    lctrl: bool,
    lshift: bool,
    /// Sentinel F-keys currently down, by key number 1..=6. An F-key up
    /// counts as a sentinel release while its mark is set, even if the
    /// modifier ups were delivered first.
    sentinel_down: [bool; 7],
    /// The physically held sentinel Ctrl/Shift were already cancelled
    /// with injected ups ahead of the first injection.
    mods_released: bool,
    clock_base: Option<(Instant, u64)>,
}

#[cfg(windows)]
impl PairHookState {
    fn new() -> Self {
        Self {
            engine: None,
            generation: 0,
            lctrl: false,
            lshift: false,
            sentinel_down: [false; 7],
            mods_released: false,
            clock_base: None,
        }
    }

    fn now(&mut self, tick_ms: u64) -> u64 {
        match self.clock_base {
            Some((base, base_ms)) => base_ms + base.elapsed().as_millis() as u64,
            None => {
                // The first real key event anchors the clock; a timer
                // tick before that (tick_ms == 0) has nothing to expire.
                if tick_ms != 0 {
                    self.clock_base = Some((Instant::now(), tick_ms));
                }
                tick_ms
            }
        }
    }

    /// Rebuild the engine when the resident published a newer config.
    fn refresh_config(&mut self) {
        let Some((generation, cfg)) = PAIR_CFG.lock().ok().and_then(|slot| slot.clone()) else {
            return;
        };
        if generation == self.generation {
            return;
        }
        self.generation = generation;
        if let Some(mut old) = self.engine.take() {
            let actions = old.shutdown();
            self.execute(actions);
        }
        self.sentinel_down = [false; 7];
        self.engine = if cfg.enabled {
            Some(crate::pair_chord::PairChordEngine::new(cfg))
        } else {
            None
        };
    }

    fn execute(&mut self, actions: Vec<crate::pair_chord::Action>) {
        use crate::pair_chord::Action;
        for action in actions {
            match action {
                Action::InjectDown(combo) => {
                    if !self.mods_released && (self.lctrl || self.lshift) {
                        // The sentinel's own Ctrl/Shift are physically
                        // down; cancel them so the injected combo is
                        // seen on its own.
                        let mut ups = Vec::new();
                        if self.lctrl {
                            ups.push(0xA2u16);
                        }
                        if self.lshift {
                            ups.push(0xA0u16);
                        }
                        send_vks(&ups, true);
                        self.mods_released = true;
                    }
                    send_vks(&crate::pair_chord::vk_codes(combo), false);
                }
                Action::InjectUp(combo) => {
                    send_vks(&crate::pair_chord::vk_codes(combo), true);
                }
            }
        }
        if self.engine.as_ref().is_some_and(|e| e.is_idle()) {
            self.mods_released = false;
        }
    }
}

#[cfg(windows)]
thread_local! {
    static PAIR_STATE: std::cell::RefCell<PairHookState> =
        std::cell::RefCell::new(PairHookState::new());
}

/// Send a combo's virtual keys down (in order) or up (in reverse order)
/// as one SendInput batch.
#[cfg(windows)]
fn send_vks(vks: &[u16], up: bool) {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP,
        SendInput, VIRTUAL_KEY,
    };
    if vks.is_empty() {
        return;
    }
    let extended = |vk: u16| {
        matches!(
            vk,
            0xA3 | 0xA5 | 0x5C | 0x21..=0x28 | 0x2D | 0x2E | 0x6F | 0x90
        )
    };
    let mut inputs = Vec::new();
    let order: Vec<u16> = if up {
        vks.iter().rev().copied().collect()
    } else {
        vks.to_vec()
    };
    for vk in order {
        let mut flags = 0u32;
        if up {
            flags |= KEYEVENTF_KEYUP.0;
        }
        if extended(vk) {
            flags |= KEYEVENTF_EXTENDEDKEY.0;
        }
        inputs.push(INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(vk),
                    wScan: 0,
                    dwFlags: windows::Win32::UI::Input::KeyboardAndMouse::KEYBD_EVENT_FLAGS(flags),
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        });
    }
    unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

#[cfg(windows)]
unsafe extern "system" fn pair_ll_keyboard(
    code: i32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::UI::WindowsAndMessaging::{CallNextHookEx, HHOOK, KBDLLHOOKSTRUCT};
    let mut swallow = false;
    if code >= 0 && lparam.0 != 0 {
        let info = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        // Our own SendInput events come back through the hook; never
        // treat them as physical sentinel presses.
        if info.flags.0 & 0x10 == 0 {
            let vk = info.vkCode as u16;
            let is_up = info.flags.0 & 0x80 != 0 || wparam.0 == 0x0101 || wparam.0 == 0x0105;
            let tick = u64::from(info.time);
            swallow = PAIR_STATE.with(|slot| {
                let mut state = slot.borrow_mut();
                state.refresh_config();
                match vk {
                    0xA2 => state.lctrl = !is_up,
                    0xA0 => state.lshift = !is_up,
                    _ => {}
                }
                if state.engine.is_none() {
                    return false;
                }
                if !(0x70..=0x75).contains(&vk) {
                    return false;
                }
                let keynum = (vk - 0x70 + 1) as usize;
                if is_up {
                    if !state.sentinel_down[keynum] {
                        return false;
                    }
                    state.sentinel_down[keynum] = false;
                    let now = state.now(tick);
                    let actions = state
                        .engine
                        .as_mut()
                        .map(|e| e.key_up(keynum as u8, now))
                        .unwrap_or_default();
                    state.execute(actions);
                    return true;
                }
                if state.sentinel_down[keynum] {
                    // A repeat of a sentinel already counted as down.
                    let now = state.now(tick);
                    let actions = state
                        .engine
                        .as_mut()
                        .map(|e| e.key_down(keynum as u8, now))
                        .unwrap_or_default();
                    state.execute(actions);
                    return true;
                }
                if !(state.lctrl && state.lshift) {
                    return false;
                }
                state.sentinel_down[keynum] = true;
                let now = state.now(tick);
                let actions = state
                    .engine
                    .as_mut()
                    .map(|e| e.key_down(keynum as u8, now))
                    .unwrap_or_default();
                state.execute(actions);
                true
            });
        }
    }
    if swallow {
        return windows::Win32::Foundation::LRESULT(1);
    }
    let hook = HHOOK(PAIR_HOOK_HANDLE.load(std::sync::atomic::Ordering::Relaxed));
    unsafe { CallNextHookEx(Some(hook), code, wparam, lparam) }
}

#[cfg(windows)]
fn pair_hook_loop() {
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetMessageW, MSG, SetTimer, SetWindowsHookExW, TranslateMessage,
        UnhookWindowsHookEx, WH_KEYBOARD_LL, WM_TIMER,
    };
    unsafe {
        let module = GetModuleHandleW(None)
            .ok()
            .map(|module| windows::Win32::Foundation::HINSTANCE(module.0));
        let Ok(hook) = SetWindowsHookExW(WH_KEYBOARD_LL, Some(pair_ll_keyboard), module, 0) else {
            state::log_line("pair chords: SetWindowsHookExW failed");
            return;
        };
        PAIR_HOOK_HANDLE.store(hook.0, std::sync::atomic::Ordering::Relaxed);
        // The timer drives the engine's hold deadlines.
        SetTimer(None, 1, 10, None);
        let mut msg = MSG::default();
        loop {
            let result = GetMessageW(&mut msg, None, 0, 0);
            if !result.as_bool() {
                break;
            }
            if msg.message == WM_TIMER {
                PAIR_STATE.with(|slot| {
                    let mut state = slot.borrow_mut();
                    state.refresh_config();
                    if state.engine.is_some() {
                        let now = state.now(0);
                        let actions = state
                            .engine
                            .as_mut()
                            .map(|e| e.tick(now))
                            .unwrap_or_default();
                        state.execute(actions);
                    }
                });
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        let _ = UnhookWindowsHookEx(hook);
        PAIR_HOOK_HANDLE.store(std::ptr::null_mut(), std::sync::atomic::Ordering::Relaxed);
    }
}

/// Create the forwarding virtual device: every key code, the relative
/// axes and MSC_SCAN, so anything the T1 can send can be re-emitted.
/// Returns the fd, or -1 on failure.
#[cfg(target_os = "linux")]
unsafe fn create_uinput() -> i32 {
    use uinput_sys::*;
    unsafe {
        let path = std::ffi::CString::new("/dev/uinput").expect("static");
        let fd = libc::open(path.as_ptr(), libc::O_WRONLY | libc::O_NONBLOCK);
        if fd < 0 {
            return -1;
        }
        let mut ok = true;
        for (request, bits, max) in [
            (UI_SET_EVBIT, [EV_KEY, EV_REL, EV_MSC], 0),
            (UI_SET_KEYBIT, [0, 0, 0], KEY_MAX),
            (UI_SET_RELBIT, [0, 0, 0], REL_MAX),
            (UI_SET_MSCBIT, [MSC_SCAN, 0, 0], 0),
        ] {
            if request == UI_SET_EVBIT {
                for bit in bits {
                    ok &= libc::ioctl(fd, request, bit) == 0;
                }
            } else if request == UI_SET_MSCBIT {
                ok &= libc::ioctl(fd, request, bits[0]) == 0;
            } else {
                for code in 0..=max {
                    ok &= libc::ioctl(fd, request, code) == 0;
                }
            }
        }
        // struct uinput_setup: input_id (4 x u16), name[80], ff_effects_max.
        let mut setup = [0u8; 92];
        setup[0..2].copy_from_slice(&3u16.to_le_bytes()); // BUS_USB
        let name = b"T1 pair-chord (virtual)";
        setup[8..8 + name.len()].copy_from_slice(name);
        ok &= libc::ioctl(fd, UI_DEV_SETUP, setup.as_ptr()) == 0;
        ok &= libc::ioctl(fd, UI_DEV_CREATE) == 0;
        if !ok {
            libc::close(fd);
            return -1;
        }
        fd
    }
}
