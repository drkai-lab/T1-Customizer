//! Platform writers for the T1 config interface.
//! Linux uses usbfs SUBMITURB (interrupt, then bulk, then a control SET_REPORT).
//! Windows uses HID WriteFile on mi_01. macOS uses IOHIDDeviceSetReport.

use crate::device::DeviceError;

#[derive(Clone, Debug)]
pub struct OpenInfo {
    pub report_id: Option<u8>,
    pub data_len: Option<usize>,
    pub had_report_id: Option<bool>,
}

pub trait DynTransport: Send {
    fn kind(&self) -> String;
    fn last_error(&self) -> Option<String>;
    fn set_last_error(&mut self, message: Option<String>);
    fn node(&self) -> Option<String>;
    fn urb_type(&self) -> Option<u8> {
        None
    }
    fn is_open(&self) -> bool;
    fn close(&mut self);
    fn open(&mut self) -> Result<Option<OpenInfo>, DeviceError>;
    fn send_frame(&mut self, frame: &[u8], report_id: u8, has_report_id: bool) -> bool;
}

pub fn make_transport() -> Box<dyn DynTransport> {
    #[cfg(windows)]
    {
        Box::new(windows::WindowsHidTransport::new())
    }
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacHidTransport::new())
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        Box::new(linux::LinuxUsbfsTransport::new())
    }
}

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
mod linux {
    use super::{DynTransport, OpenInfo};
    use crate::device::DeviceError;

    pub struct LinuxUsbfsTransport {
        last_error: Option<String>,
    }

    impl LinuxUsbfsTransport {
        pub fn new() -> Self {
            Self { last_error: None }
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
            None
        }
        fn is_open(&self) -> bool {
            false
        }
        fn close(&mut self) {}
        fn open(&mut self) -> Result<Option<OpenInfo>, DeviceError> {
            self.last_error = Some("device-not-found".into());
            Ok(None)
        }
        fn send_frame(&mut self, _frame: &[u8], _report_id: u8, _has_report_id: bool) -> bool {
            false
        }
    }
}

#[cfg(windows)]
pub mod windows;

#[cfg(target_os = "macos")]
pub mod macos;
