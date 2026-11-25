//! One settings window per state directory.
//!
//! A second launch asks the running window to come forward, then exits.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use fs2::FileExt;

type Waker = Arc<dyn Fn() + Send + Sync>;

const PORT_FILE: &str = "gui-port";

pub enum Claim {
    Primary(GuiInstance),
    AlreadyRunning,
}

pub struct GuiInstance {
    _file: Option<File>,
    stop: Arc<AtomicBool>,
    raised: Arc<AtomicBool>,
    waker: Arc<Mutex<Option<Waker>>>,
    thread: Option<JoinHandle<()>>,
    port_file: Option<PathBuf>,
}

impl GuiInstance {
    pub fn set_waker(&self, waker: Waker) {
        if let Ok(mut slot) = self.waker.lock() {
            *slot = Some(waker);
        }
    }

    pub fn take_raise(&self) -> bool {
        self.raised.swap(false, Ordering::SeqCst)
    }
}

impl Drop for GuiInstance {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(path) = self.port_file.take() {
            let _ = fs::remove_file(path);
        }
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
    }
}

pub fn claim(dir: &Path) -> Claim {
    if fs::create_dir_all(dir).is_err() {
        return Claim::Primary(GuiInstance::inert());
    }
    let Ok(file) = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(dir.join("gui.lock"))
    else {
        return Claim::Primary(GuiInstance::inert());
    };
    if file.try_lock_exclusive().is_err() {
        return Claim::AlreadyRunning;
    }
    let Ok(listener) = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)) else {
        return Claim::Primary(GuiInstance::held(file));
    };
    let Ok(addr) = listener.local_addr() else {
        return Claim::Primary(GuiInstance::held(file));
    };
    let port_path = dir.join(PORT_FILE);
    if fs::write(&port_path, format!("{}\n", addr.port())).is_err() {
        return Claim::Primary(GuiInstance::held(file));
    }
    let stop = Arc::new(AtomicBool::new(false));
    let raised = Arc::new(AtomicBool::new(false));
    let waker = Arc::new(Mutex::new(None));
    let thread = thread::Builder::new()
        .name("gui-instance".into())
        .spawn({
            let stop = stop.clone();
            let raised = raised.clone();
            let waker = waker.clone();
            move || serve(listener, stop, raised, waker)
        })
        .ok();
    Claim::Primary(GuiInstance {
        _file: Some(file),
        stop,
        raised,
        waker,
        thread,
        port_file: Some(port_path),
    })
}

/// Tell the running window to come forward. The caller should then exit.
pub fn focus_existing(dir: &Path) -> bool {
    allow_set_foreground();
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(1) {
        let Ok(text) = fs::read_to_string(dir.join(PORT_FILE)) else {
            thread::sleep(Duration::from_millis(20));
            continue;
        };
        let Some(port) = text.trim().parse::<u16>().ok() else {
            thread::sleep(Duration::from_millis(20));
            continue;
        };
        let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
        if let Ok(mut stream) = TcpStream::connect_timeout(&addr, Duration::from_millis(200)) {
            let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
            if stream.write_all(b"raise\n").is_ok() {
                let mut buf = [0u8; 8];
                let _ = stream.read(&mut buf);
                return true;
            }
        }
        thread::sleep(Duration::from_millis(20));
    }
    false
}

impl GuiInstance {
    fn inert() -> Self {
        Self {
            _file: None,
            stop: Arc::new(AtomicBool::new(true)),
            raised: Arc::new(AtomicBool::new(false)),
            waker: Arc::new(Mutex::new(None)),
            thread: None,
            port_file: None,
        }
    }

    fn held(file: File) -> Self {
        Self {
            _file: Some(file),
            stop: Arc::new(AtomicBool::new(true)),
            raised: Arc::new(AtomicBool::new(false)),
            waker: Arc::new(Mutex::new(None)),
            thread: None,
            port_file: None,
        }
    }
}

fn serve(
    listener: TcpListener,
    stop: Arc<AtomicBool>,
    raised: Arc<AtomicBool>,
    waker: Arc<Mutex<Option<Waker>>>,
) {
    let _ = listener.set_nonblocking(true);
    while !stop.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((mut stream, _)) => {
                let _ = stream.set_read_timeout(Some(Duration::from_millis(200)));
                let mut buf = [0u8; 16];
                let _ = stream.read(&mut buf);
                raised.store(true, Ordering::SeqCst);
                if let Ok(slot) = waker.lock()
                    && let Some(wake) = slot.clone()
                {
                    wake();
                }
                let _ = stream.write_all(b"ok\n");
            }
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(20));
            }
            Err(_) => thread::sleep(Duration::from_millis(20)),
        }
    }
}

#[cfg(windows)]
fn allow_set_foreground() {
    use windows::Win32::UI::WindowsAndMessaging::AllowSetForegroundWindow;
    unsafe {
        let _ = AllowSetForegroundWindow(u32::MAX);
    }
}

#[cfg(not(windows))]
fn allow_set_foreground() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_second_launch_asks_the_first_window_to_focus() {
        let dir = std::env::temp_dir().join(format!(
            "t1-gui-instance-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|dur| dur.as_nanos())
                .unwrap_or(0)
        ));
        let _ = fs::remove_dir_all(&dir);
        let primary = match claim(&dir) {
            Claim::Primary(instance) => instance,
            Claim::AlreadyRunning => panic!("first claim lost the lock"),
        };
        assert!(matches!(claim(&dir), Claim::AlreadyRunning));
        let hits = Arc::new(AtomicBool::new(false));
        primary.set_waker({
            let hits = hits.clone();
            Arc::new(move || hits.store(true, Ordering::SeqCst))
        });
        assert!(focus_existing(&dir), "running instance did not answer");
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(2) && !primary.take_raise() {
            thread::sleep(Duration::from_millis(10));
        }
        assert!(hits.load(Ordering::SeqCst), "waker was not called");
        drop(primary);
        match claim(&dir) {
            Claim::Primary(next) => drop(next),
            Claim::AlreadyRunning => panic!("lock was not released"),
        }
        let _ = fs::remove_dir_all(&dir);
    }
}
