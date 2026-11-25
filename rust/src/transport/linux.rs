//! usbfs writer. Same ioctls as the Python tool: claim interface 1, SUBMITURB
//! on endpoint 0x02 (interrupt then bulk), then control SET_REPORT.
//! hidraw is not used for writes.

use std::fs;
use std::io::ErrorKind;
use std::os::fd::RawFd;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

use crate::device::DeviceError;
use crate::hidparse::{choose_output, hid_descriptor_lengths, parse_report_descriptor};
use crate::protocol::{self, UsbdevfsCtrl, UsbdevfsUrb};
use crate::{CONFIG_ENDPOINT, CONFIG_INTERFACE, PID, VID};

use super::{DynTransport, OpenInfo};

const URB_INTERRUPT: u8 = 1;
const URB_BULK: u8 = 3;

pub struct LinuxUsbfsTransport {
    fd: Option<RawFd>,
    urb_type: Option<u8>,
    last_error: Option<String>,
    node: Option<String>,
}

impl LinuxUsbfsTransport {
    pub fn new() -> Self {
        Self {
            fd: None,
            urb_type: None,
            last_error: None,
            node: None,
        }
    }

    fn control(
        &self,
        request_type: u8,
        request: u8,
        wvalue: u16,
        windex: u16,
        data: &mut [u8],
        timeout: u32,
    ) -> std::io::Result<usize> {
        let Some(fd) = self.fd else {
            return Err(std::io::Error::other("closed"));
        };
        let mut ctrl = UsbdevfsCtrl {
            b_request_type: request_type,
            b_request: request,
            w_value: wvalue,
            w_index: windex,
            w_length: data.len() as u16,
            timeout,
            data: data.as_mut_ptr() as u64,
        };
        let rc = unsafe { libc::ioctl(fd, protocol::usbfs_control() as libc::Ioctl, &mut ctrl) };
        if rc < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(rc as usize)
        }
    }

    fn descriptor_lengths(&self) -> std::collections::BTreeMap<u8, u16> {
        let mut head = [0u8; 9];
        if self.control(0x81, 0x06, 0x0200, 0, &mut head, 500).is_err() || head[0] < 4 {
            return Default::default();
        }
        let mut total = u16::from_le_bytes([head[2], head[3]]) as usize;
        if total < 9 {
            return Default::default();
        }
        total = total.min(1024);
        if total == 9 {
            return hid_descriptor_lengths(&head);
        }
        let mut data = vec![0u8; total];
        match self.control(0x81, 0x06, 0x0200, 0, &mut data, 500) {
            Ok(n) => hid_descriptor_lengths(&data[..n.min(data.len())]),
            Err(_) => Default::default(),
        }
    }

    fn read_report_descriptor(&self) -> OpenInfo {
        let lengths = self.descriptor_lengths();
        let length = lengths
            .get(&CONFIG_INTERFACE)
            .copied()
            .or_else(|| lengths.values().next().copied());
        let Some(length) = length else {
            return OpenInfo {
                report_id: None,
                data_len: None,
                had_report_id: None,
            };
        };
        let mut data = vec![0u8; length as usize];
        let Ok(n) = self.control(0x81, 0x06, 0x2200, CONFIG_INTERFACE as u16, &mut data, 500)
        else {
            return OpenInfo {
                report_id: None,
                data_len: None,
                had_report_id: None,
            };
        };
        let Some(outputs) = parse_report_descriptor(&data[..n.min(data.len())]) else {
            return OpenInfo {
                report_id: None,
                data_len: None,
                had_report_id: None,
            };
        };
        match choose_output(&outputs) {
            Some((rid, bytes, had)) => OpenInfo {
                report_id: Some(rid),
                data_len: Some(bytes),
                had_report_id: Some(had),
            },
            None => OpenInfo {
                report_id: None,
                data_len: None,
                had_report_id: None,
            },
        }
    }

    fn urb_order(&self) -> Vec<u8> {
        if let Some(kind) = self.urb_type
            && (kind == URB_INTERRUPT || kind == URB_BULK)
        {
            return vec![kind];
        }
        vec![URB_INTERRUPT, URB_BULK]
    }

    fn send_urb(&self, frame: &[u8], urb_type: u8) -> bool {
        let Some(fd) = self.fd else {
            return false;
        };
        let mut urb = UsbdevfsUrb {
            typ: urb_type,
            endpoint: CONFIG_ENDPOINT,
            _pad0: 0,
            status: 0,
            flags: 0,
            _pad1: 0,
            buffer: frame.as_ptr() as u64,
            buffer_length: frame.len() as i32,
            actual_length: 0,
            start_frame: 0,
            stream_id: 0,
            error_count: 0,
            signr: 0,
            usercontext: 0,
        };
        let submitted =
            unsafe { libc::ioctl(fd, protocol::usbfs_submiturb() as libc::Ioctl, &mut urb) };
        if submitted < 0 {
            return false;
        }
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            let mut slot: *mut libc::c_void = std::ptr::null_mut();
            let rc = unsafe {
                libc::ioctl(
                    fd,
                    protocol::usbfs_reapurbndelay() as libc::Ioctl,
                    &mut slot,
                )
            };
            if rc >= 0 {
                return urb.status == 0;
            }
            let err = std::io::Error::last_os_error();
            if err.raw_os_error() == Some(libc::EAGAIN) || err.kind() == ErrorKind::WouldBlock {
                if Instant::now() >= deadline {
                    break;
                }
                thread::sleep(Duration::from_millis(10));
                continue;
            }
            break;
        }
        unsafe {
            libc::ioctl(fd, protocol::usb_ioc(0, b'U', 11, 0) as libc::Ioctl, 0);
        }
        false
    }

    fn send_control(&self, frame: &[u8], report_id: u8, has_report_id: bool) -> bool {
        let wvalue = (0x02 << 8)
            | if has_report_id {
                u16::from(report_id)
            } else {
                0
            };
        let mut data = frame.to_vec();
        self.control(
            0x21,
            0x09,
            wvalue,
            u16::from(CONFIG_INTERFACE),
            &mut data,
            1000,
        )
        .is_ok()
    }
}

impl Default for LinuxUsbfsTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for LinuxUsbfsTransport {
    fn drop(&mut self) {
        self.close();
    }
}

impl DynTransport for LinuxUsbfsTransport {
    fn kind(&self) -> String {
        "usbfs".into()
    }

    fn last_error(&self) -> Option<String> {
        self.last_error.clone()
    }

    fn set_last_error(&mut self, message: Option<String>) {
        self.last_error = message;
    }

    fn node(&self) -> Option<String> {
        self.node.clone()
    }

    fn urb_type(&self) -> Option<u8> {
        self.urb_type
    }

    fn is_open(&self) -> bool {
        self.fd.is_some()
    }

    fn close(&mut self) {
        if let Some(fd) = self.fd.take() {
            let mut iface = u32::from(CONFIG_INTERFACE);
            unsafe {
                libc::ioctl(
                    fd,
                    protocol::usb_ioc(2, b'U', 16, 4) as libc::Ioctl,
                    &mut iface,
                );
                libc::close(fd);
            }
        }
        self.node = None;
    }

    fn open(&mut self) -> Result<Option<OpenInfo>, DeviceError> {
        self.close();
        let Some((bus, dev)) = find_usb_node() else {
            self.last_error = Some("device-not-found".into());
            return Ok(None);
        };
        let path = format!("/dev/bus/usb/{bus:03}/{dev:03}");
        let fd = unsafe {
            libc::open(
                std::ffi::CString::new(path.clone()).unwrap().as_ptr(),
                libc::O_RDWR,
            )
        };
        if fd < 0 {
            let err = std::io::Error::last_os_error();
            if err.kind() == ErrorKind::PermissionDenied {
                let message = format!(
                    "Permission denied on {path}\nInstall the udev rule first:\n  sudo ./install.sh"
                );
                self.last_error = Some("permission".into());
                return Err(DeviceError { message });
            }
            self.last_error = Some(err.to_string());
            return Ok(None);
        }
        self.fd = Some(fd);
        self.node = Some(path);
        let mut iface = u32::from(CONFIG_INTERFACE);
        let claimed = unsafe {
            libc::ioctl(
                fd,
                protocol::usbfs_claiminterface() as libc::Ioctl,
                &mut iface,
            )
        };
        if claimed < 0 {
            let err = std::io::Error::last_os_error();
            self.close();
            if err.raw_os_error() == Some(libc::EBUSY) {
                self.last_error = Some(
                    "claim-failed: device busy - another t1-keyboard-config window/monitor is using it"
                        .into(),
                );
            } else {
                self.last_error = Some(format!("claim-failed: {err}"));
            }
            return Ok(None);
        }
        Ok(Some(self.read_report_descriptor()))
    }

    fn send_frame(&mut self, frame: &[u8], report_id: u8, has_report_id: bool) -> bool {
        for kind in self.urb_order() {
            if self.send_urb(frame, kind) {
                self.urb_type = Some(kind);
                return true;
            }
        }
        self.send_control(frame, report_id, has_report_id)
    }
}

pub fn find_usb_node() -> Option<(u32, u32)> {
    let entries = fs::read_dir("/sys/bus/usb/devices").ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name()?.to_string_lossy();
        if name.contains(':') {
            continue;
        }
        let Some(vid) = read_hex(&path.join("idVendor")) else {
            continue;
        };
        let Some(pid) = read_hex(&path.join("idProduct")) else {
            continue;
        };
        if vid == u32::from(VID) && pid == u32::from(PID) {
            let Some(bus) = read_dec(&path.join("busnum")) else {
                continue;
            };
            let Some(dev) = read_dec(&path.join("devnum")) else {
                continue;
            };
            return Some((bus, dev));
        }
    }
    None
}

fn read_hex(path: &PathBuf) -> Option<u32> {
    let text = fs::read_to_string(path).ok()?;
    u32::from_str_radix(text.trim(), 16).ok()
}

fn read_dec(path: &PathBuf) -> Option<u32> {
    let text = fs::read_to_string(path).ok()?;
    text.trim().parse().ok()
}

pub fn usb_node_path() -> Option<String> {
    find_usb_node().map(|(bus, dev)| format!("/dev/bus/usb/{bus:03}/{dev:03}"))
}

pub fn t1_input_nodes() -> Vec<(String, String)> {
    let mut nodes = Vec::new();
    let Ok(entries) = fs::read_dir("/sys/class/input") else {
        return nodes;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with("event") {
            continue;
        }
        let uevent = entry.path().join("device/uevent");
        let Ok(text) = fs::read_to_string(&uevent) else {
            continue;
        };
        let mut product = "";
        let mut dev_name = String::new();
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("PRODUCT=") {
                product = rest.trim();
            } else if let Some(rest) = line.strip_prefix("NAME=") {
                dev_name = rest.trim().trim_matches('"').to_string();
            }
        }
        let mut parts = product.split('/');
        let _bus = parts.next();
        let Ok(vid) = u32::from_str_radix(parts.next().unwrap_or(""), 16) else {
            continue;
        };
        let Ok(pid) = u32::from_str_radix(parts.next().unwrap_or(""), 16) else {
            continue;
        };
        if vid != u32::from(VID) || pid != u32::from(PID) {
            continue;
        }
        nodes.push((format!("/dev/input/{name}"), dev_name));
    }
    nodes.sort();
    nodes
}
