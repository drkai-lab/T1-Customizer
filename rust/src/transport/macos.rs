//! IOHIDDeviceSetReport via hidapi on the mi_01 interface.

use hidapi::{HidApi, HidDevice};

use crate::device::DeviceError;
use crate::{PID, VID};

use super::{DynTransport, OpenInfo};

pub struct MacHidTransport {
    device: Option<HidDevice>,
    last_error: Option<String>,
    node: Option<String>,
}

impl MacHidTransport {
    pub fn new() -> Self {
        Self {
            device: None,
            last_error: None,
            node: None,
        }
    }
}

impl DynTransport for MacHidTransport {
    fn kind(&self) -> String {
        "hid".into()
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
    fn is_open(&self) -> bool {
        self.device.is_some()
    }
    fn close(&mut self) {
        self.device = None;
        self.node = None;
    }
    fn open(&mut self) -> Result<Option<OpenInfo>, DeviceError> {
        self.close();
        let api = match HidApi::new() {
            Ok(api) => api,
            Err(err) => {
                self.last_error = Some(format!("enum-failed: {err}"));
                return Ok(None);
            }
        };
        let info = api.device_list().find(|dev| {
            dev.vendor_id() == VID && dev.product_id() == PID && dev.interface_number() == 1
        });
        let Some(info) = info else {
            self.last_error = Some("device-not-found".into());
            return Ok(None);
        };
        match info.open_device(&api) {
            Ok(device) => {
                self.node = Some("IOHIDDevice".into());
                self.device = Some(device);
                Ok(Some(OpenInfo {
                    report_id: Some(3),
                    data_len: Some(64),
                    had_report_id: Some(true),
                }))
            }
            Err(err) => {
                self.last_error = Some(format!("open-failed: {err}"));
                Ok(None)
            }
        }
    }
    fn send_frame(&mut self, frame: &[u8], report_id: u8, has_report_id: bool) -> bool {
        let Some(device) = &self.device else {
            return false;
        };
        let data = if has_report_id {
            if frame.is_empty() {
                return false;
            }
            // hidapi on macOS expects the report id in the first byte.
            frame.to_vec()
        } else {
            let mut data = Vec::with_capacity(1 + frame.len());
            data.push(report_id);
            data.extend_from_slice(frame);
            data
        };
        device.write(&data).is_ok()
    }
}
