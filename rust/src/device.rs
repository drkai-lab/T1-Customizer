//! Protocol session: framing, probe order, and the write commands.
//! The platform send path lives in `transport`.

use crate::protocol::{
    DEFAULT_DATA_LEN, flash_led_report, flash_report, frame, key_reports, layer_report, led_report,
    media_report, mouse_report, probe_candidates, probe_report,
};
use crate::state;
use crate::transport::{DynTransport, make_transport};

#[derive(Debug)]
pub struct DeviceError {
    pub message: String,
}

impl std::fmt::Display for DeviceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for DeviceError {}

pub struct T1Device {
    pub report_id: u8,
    pub data_len: usize,
    pub had_report_id: bool,
    fail_count: u32,
    logged: Option<String>,
    transport: Box<dyn DynTransport>,
}

impl T1Device {
    pub fn new() -> Self {
        Self {
            report_id: 3,
            data_len: DEFAULT_DATA_LEN,
            had_report_id: true,
            fail_count: 0,
            logged: None,
            transport: make_transport(),
        }
    }

    pub fn last_error(&self) -> Option<String> {
        self.transport.last_error()
    }

    pub fn node(&self) -> Option<String> {
        self.transport.node()
    }

    pub fn transport_kind(&self) -> String {
        self.transport.kind()
    }

    pub fn urb_type(&self) -> Option<u8> {
        self.transport.urb_type()
    }

    pub fn is_open(&self) -> bool {
        self.transport.is_open()
    }

    pub fn close(&mut self) {
        self.transport.close();
    }

    pub fn connect(&mut self) -> Result<bool, DeviceError> {
        self.close();
        let found = self.transport.open()?;
        let Some(found) = found else {
            let message = self
                .transport
                .last_error()
                .unwrap_or_else(|| "connect-failed".into());
            self.log_once(&message);
            return Ok(false);
        };
        if let Some(rid) = found.report_id {
            self.report_id = rid;
            self.data_len = found.data_len.unwrap_or(DEFAULT_DATA_LEN);
            self.had_report_id = found.had_report_id.unwrap_or(true);
        } else {
            self.report_id = 3;
            self.had_report_id = true;
            self.data_len = DEFAULT_DATA_LEN;
        }
        if !self.had_report_id {
            self.report_id = 0;
        }
        if !self.probe() {
            let detail = self.transport.last_error();
            self.close();
            let message = match detail {
                Some(detail) if detail != "probe-failed" => format!("probe-failed: {detail}"),
                _ => "probe-failed".into(),
            };
            self.transport.set_last_error(Some(message.clone()));
            self.log_once(&message);
            return Ok(false);
        }
        self.logged = None;
        self.transport.set_last_error(None);
        Ok(true)
    }

    fn log_once(&mut self, message: &str) {
        if self.logged.as_deref() == Some(message) {
            return;
        }
        self.logged = Some(message.to_string());
        state::log_line(&format!("connect failed: {message}"));
    }

    pub fn frame_bytes(&self, payload: &[u8]) -> Vec<u8> {
        frame(payload, self.report_id, self.data_len, self.had_report_id)
    }

    pub fn write_payload(&mut self, payload: &[u8]) -> bool {
        if !self.is_open() {
            return false;
        }
        let framed = self.frame_bytes(payload);
        let ok = self
            .transport
            .send_frame(&framed, self.report_id, self.had_report_id);
        if ok {
            self.fail_count = 0;
            return true;
        }
        self.fail_count += 1;
        if self.fail_count >= 3 {
            self.close();
            self.transport.set_last_error(Some("write-failed".into()));
        }
        false
    }

    pub fn probe(&mut self) -> bool {
        for rid in probe_candidates(self.report_id, self.had_report_id) {
            self.report_id = rid;
            self.fail_count = 0;
            if self.write_payload(&probe_report()) {
                return true;
            }
            if !self.is_open() {
                return false;
            }
        }
        false
    }

    pub fn write_many(&mut self, payloads: &[Vec<u8>]) -> bool {
        for payload in payloads {
            if !self.write_payload(payload) {
                return false;
            }
        }
        true
    }

    pub fn switch_layer(&mut self, layer: u8) -> bool {
        self.write_payload(&layer_report(layer))
    }

    pub fn write_key(&mut self, keynum: u8, layer: u8, keys: &[(u8, u8)], lone_mod: u8) -> bool {
        if self.report_id != 0 && !self.switch_layer(layer) {
            return false;
        }
        let reports: Vec<Vec<u8>> = key_reports(keynum, layer, self.report_id, keys, lone_mod)
            .into_iter()
            .map(|row| row.to_vec())
            .collect();
        if !self.write_many(&reports) {
            return false;
        }
        self.write_payload(&flash_report())
    }

    pub fn write_media(&mut self, keynum: u8, layer: u8, action: &str) -> bool {
        let Some(report) = media_report(keynum, layer, self.report_id, action) else {
            return false;
        };
        if self.report_id != 0 && !self.switch_layer(layer) {
            return false;
        }
        self.write_payload(&report) && self.write_payload(&flash_report())
    }

    pub fn write_mouse(&mut self, keynum: u8, layer: u8, action: &str) -> bool {
        let Some(report) = mouse_report(keynum, layer, self.report_id, action) else {
            return false;
        };
        if self.report_id != 0 && !self.switch_layer(layer) {
            return false;
        }
        self.write_payload(&report) && self.write_payload(&flash_report())
    }

    pub fn write_led(&mut self, layer: u8, mode: u8) -> bool {
        if self.report_id != 0 && !self.switch_layer(layer) {
            return false;
        }
        self.write_payload(&led_report(layer, self.report_id, mode))
            && self.write_payload(&flash_led_report())
    }
}

impl Default for T1Device {
    fn default() -> Self {
        Self::new()
    }
}
