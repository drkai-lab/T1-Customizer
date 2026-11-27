//! Protocol session: framing, probe order, and the write commands.
//! The platform send path lives in `transport`.

use crate::protocol::{
    DEFAULT_DATA_LEN, WriteStep, frame, frame_delay_after_ms, is_knob_slot, key_write_steps,
    layer_report, led_write_steps, media_write_steps, mouse_write_steps, probe_candidates,
    probe_report,
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
    last_failure: Option<String>,
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
            last_failure: None,
            transport: make_transport(),
        }
    }

    pub fn last_error(&self) -> Option<String> {
        self.transport.last_error()
    }

    /// Detail of the most recent failed write: which operation and
    /// stage failed, plus the transport's error when it has one. The
    /// GUI appends this to the failure toast so a failed write on real
    /// hardware produces actionable data instead of a bare "failed".
    pub fn last_failure(&self) -> Option<String> {
        self.last_failure.clone()
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

    /// Send one step of a write sequence. On success the next frame is
    /// paced with `frame_delay_after_ms` (the MCU needs time to commit
    /// saves to flash; an unpaced burst can silently lose frames). On
    /// failure the operation, stage and transport error are recorded
    /// in `last_failure` and written to the app log.
    fn write_step(&mut self, op: &str, stage: &str, payload: &[u8]) -> bool {
        if self.write_payload(payload) {
            let delay = frame_delay_after_ms(payload);
            if delay > 0 {
                std::thread::sleep(std::time::Duration::from_millis(delay));
            }
            return true;
        }
        let detail = self
            .transport
            .last_error()
            .unwrap_or_else(|| "send-failed".into());
        let message = format!("{op} stage={stage}: {detail}");
        self.last_failure = Some(message.clone());
        state::log_line(&format!("write failed: {message}"));
        false
    }

    /// Run a whole write sequence, stopping at the first failed step.
    fn run_steps(&mut self, op: &str, steps: &[WriteStep]) -> bool {
        let total = steps.len();
        for (index, step) in steps.iter().enumerate() {
            let stage = format!("{} {}/{}", step.stage, index + 1, total);
            if !self.write_step(op, &stage, &step.payload) {
                return false;
            }
        }
        self.last_failure = None;
        true
    }

    /// Log the exact frames of a knob-slot write (and its outcome) so a
    /// real-hardware failure leaves actionable data in the app log.
    fn log_knob_write(&self, op: &str, keynum: u8, layer: u8, steps: &[WriteStep], ok: bool) {
        let frames = steps
            .iter()
            .map(|s| {
                s.payload
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect::<Vec<_>>()
            .join(" | ");
        state::log_line(&format!(
            "knob write {op} keynum={keynum} layer={layer} report_id={} ok={ok} frames: {frames}",
            self.report_id
        ));
    }

    /// Knob-slot writes get one full retry: the assignment is
    /// idempotent, and a retry distinguishes a transient frame loss
    /// from a slot the firmware genuinely refuses.
    /// Record the outcome of an assignment write in the app log
    /// (`paths::log_file()`), whatever the target slot. Every GUI
    /// assignment write — key, media, mouse; success or failure —
    /// leaves one of these lines, so a reported failure can always be
    /// matched against the log afterwards.
    fn log_write_outcome(&self, op: &str, keynum: u8, layer: u8, ok: bool) {
        state::log_line(&format!(
            "write {op} keynum={keynum} layer={layer} ok={ok}"
        ));
    }

    fn run_write(&mut self, op: &str, keynum: u8, layer: u8, steps: &[WriteStep]) -> bool {
        let knob = is_knob_slot(keynum);
        let ok = self.run_steps(op, steps);
        self.log_write_outcome(op, keynum, layer, ok);
        if knob {
            self.log_knob_write(op, keynum, layer, steps, ok);
        }
        if !ok && knob && self.is_open() {
            state::log_line(&format!("knob write {op} keynum={keynum}: retrying once"));
            let retry_ok = self.run_steps(op, steps);
            self.log_write_outcome(op, keynum, layer, retry_ok);
            self.log_knob_write(op, keynum, layer, steps, retry_ok);
            return retry_ok;
        }
        ok
    }

    pub fn switch_layer(&mut self, layer: u8) -> bool {
        self.write_step("switch_layer", "layer", &layer_report(layer))
    }

    pub fn write_key(&mut self, keynum: u8, layer: u8, keys: &[(u8, u8)], lone_mod: u8) -> bool {
        let steps = key_write_steps(self.report_id, keynum, layer, keys, lone_mod);
        self.run_write("write_key", keynum, layer, &steps)
    }

    pub fn write_media(&mut self, keynum: u8, layer: u8, action: &str) -> bool {
        let Some(steps) = media_write_steps(self.report_id, keynum, layer, action) else {
            // Log this too: the write dies before any device traffic,
            // and a failure that leaves no trace in the log cannot be
            // diagnosed after the fact.
            self.last_failure = Some(format!("write_media: unknown action {action}"));
            state::log_line(&format!(
                "write write_media keynum={keynum} layer={layer} action={action} ok=false: unknown action"
            ));
            return false;
        };
        self.run_write("write_media", keynum, layer, &steps)
    }

    pub fn write_mouse(&mut self, keynum: u8, layer: u8, action: &str) -> bool {
        let Some(steps) = mouse_write_steps(self.report_id, keynum, layer, action) else {
            self.last_failure = Some(format!("write_mouse: unknown action {action}"));
            state::log_line(&format!(
                "write write_mouse keynum={keynum} layer={layer} action={action} ok=false: unknown action"
            ));
            return false;
        };
        self.run_write("write_mouse", keynum, layer, &steps)
    }

    pub fn write_led(&mut self, layer: u8, mode: u8) -> bool {
        let steps = led_write_steps(layer, self.report_id, mode);
        self.run_steps("write_led", &steps)
    }
}

impl Default for T1Device {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every GUI assignment write must leave a line in the app log —
    /// including a write that dies before any device traffic (unknown
    /// action) and one that fails on a closed transport. Until this
    /// was enforced, a failed knob mouse write produced no log file
    /// at all: only knob writes that reached the step runner were
    /// logged, and the unknown-action return path logged nothing.
    #[test]
    fn assignment_writes_are_logged_even_when_they_fail_early() {
        let dir = std::env::temp_dir().join(format!("t1-device-log-test-{}", std::process::id()));
        // SAFETY: this is the only test that redirects the state dir,
        // and no other test reads the log file.
        unsafe { std::env::set_var("T1_STATE_DIR", &dir) };
        let mut device = T1Device::new();

        // Unknown action (the display-label leak): fails in the app,
        // before run_write — must still be logged with its action.
        assert!(!device.write_mouse(13, 1, "Wheel Up"));
        assert!(
            device
                .last_failure()
                .is_some_and(|f| f.contains("unknown action Wheel Up"))
        );

        // Valid action on a knob slot, transport closed: fails in the
        // step runner — the outcome line and the knob frame line must
        // both be logged.
        assert!(!device.write_mouse(13, 1, "wheel_up"));

        let text = std::fs::read_to_string(dir.join("monitor.log"))
            .expect("a write attempt must create the log file");
        assert!(
            text.contains(
                "write write_mouse keynum=13 layer=1 action=Wheel Up ok=false: unknown action"
            ),
            "log was:\n{text}"
        );
        assert!(
            text.contains("write write_mouse keynum=13 layer=1 ok=false"),
            "log was:\n{text}"
        );
        assert!(text.contains("knob write write_mouse"), "log was:\n{text}");

        unsafe { std::env::remove_var("T1_STATE_DIR") };
        let _ = std::fs::remove_dir_all(&dir);
    }
}
