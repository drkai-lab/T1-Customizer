//! HID WriteFile on the mi_01 interface. SetupAPI detail cbSize follows
//! `protocol::hid_detail_cb_size`; the device path still starts at byte 4.

use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;

use windows::Win32::Devices::DeviceAndDriverInstallation::{
    DIGCF_DEVICEINTERFACE, DIGCF_PRESENT, HDEVINFO, SP_DEVICE_INTERFACE_DATA,
    SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInterfaces, SetupDiGetClassDevsW,
    SetupDiGetDeviceInterfaceDetailW,
};
use windows::Win32::Devices::HumanInterfaceDevice::{
    HIDP_CAPS, HidD_FreePreparsedData, HidD_GetHidGuid, HidD_GetPreparsedData, HidP_GetCaps,
};
use windows::Win32::Foundation::{CloseHandle, ERROR_NO_MORE_ITEMS, HANDLE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING, WriteFile,
};
use windows::Win32::System::IO::DeviceIoControl;
use windows::core::PCWSTR;

use crate::device::DeviceError;
use crate::hidparse::{choose_output, parse_report_descriptor};
use crate::protocol::{hid_detail_cb_size, sp_device_interface_data_cb_size};

use super::{DynTransport, OpenInfo};

const GENERIC_READ: u32 = 0x8000_0000;
const GENERIC_WRITE: u32 = 0x4000_0000;
const IOCTL_HID_GET_REPORT_DESCRIPTOR: u32 = 0x000B_040F;

pub struct WindowsHidTransport {
    handle: Option<HANDLE>,
    out_len: usize,
    last_error: Option<String>,
    node: Option<String>,
}

// A kernel HANDLE is moved with the transport. It is not shared across threads.
unsafe impl Send for WindowsHidTransport {}

impl Default for WindowsHidTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowsHidTransport {
    pub fn new() -> Self {
        Self {
            handle: None,
            out_len: 65,
            last_error: None,
            node: None,
        }
    }

    fn output_length(&self) -> u16 {
        let Some(handle) = self.handle else {
            return 0;
        };
        unsafe {
            let mut preparsed =
                windows::Win32::Devices::HumanInterfaceDevice::PHIDP_PREPARSED_DATA(0);
            if !HidD_GetPreparsedData(handle, &mut preparsed) {
                return 0;
            }
            let mut caps = HIDP_CAPS::default();
            let status = HidP_GetCaps(preparsed, &mut caps);
            let _ = HidD_FreePreparsedData(preparsed);
            if status.0 < 0 {
                0
            } else {
                caps.OutputReportByteLength
            }
        }
    }

    fn read_descriptor(&self) -> OpenInfo {
        let Some(handle) = self.handle else {
            return none_info();
        };
        unsafe {
            let mut buf = vec![0u8; 2048];
            let mut returned = 0u32;
            if DeviceIoControl(
                handle,
                IOCTL_HID_GET_REPORT_DESCRIPTOR,
                None,
                0,
                Some(buf.as_mut_ptr() as *mut _),
                buf.len() as u32,
                Some(&mut returned),
                None,
            )
            .is_err()
            {
                return none_info();
            }
            let n = returned as usize;
            let Some(outputs) = parse_report_descriptor(&buf[..n.min(buf.len())]) else {
                return none_info();
            };
            match choose_output(&outputs) {
                Some((rid, bytes, had)) => OpenInfo {
                    report_id: Some(rid),
                    data_len: Some(bytes),
                    had_report_id: Some(had),
                },
                None => none_info(),
            }
        }
    }
}

fn none_info() -> OpenInfo {
    OpenInfo {
        report_id: None,
        data_len: None,
        had_report_id: None,
    }
}

impl Drop for WindowsHidTransport {
    fn drop(&mut self) {
        self.close();
    }
}

impl DynTransport for WindowsHidTransport {
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
        self.handle.is_some()
    }
    fn close(&mut self) {
        if let Some(handle) = self.handle.take() {
            unsafe {
                let _ = CloseHandle(handle);
            }
        }
        self.node = None;
    }
    fn open(&mut self) -> Result<Option<OpenInfo>, DeviceError> {
        self.close();
        self.last_error = None;
        let path = match find_device_path() {
            Ok(Some(path)) => path,
            Ok(None) => {
                self.last_error = Some("device-not-found".into());
                return Ok(None);
            }
            Err(err) => {
                self.last_error = Some(format!("enum-failed: {err}"));
                return Ok(None);
            }
        };
        let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
        let handle = unsafe {
            CreateFileW(
                PCWSTR(wide.as_ptr()),
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_NORMAL,
                None,
            )
        };
        let handle = match handle {
            Ok(handle) => handle,
            Err(err) => {
                if err.code().0 == 5 {
                    self.last_error =
                        Some("permission: close other programs using the T1 Keyboard".into());
                } else {
                    self.last_error = Some(format!("open-failed: {err}"));
                }
                return Ok(None);
            }
        };
        self.handle = Some(handle);
        self.node = Some(path);
        self.out_len = usize::from(self.output_length()).max(1);
        let info = self.read_descriptor();
        if info.report_id.is_none() {
            let out_len = self.out_len.max(65);
            self.out_len = out_len;
            return Ok(Some(OpenInfo {
                report_id: Some(3),
                data_len: Some(out_len.saturating_sub(1).max(1)),
                had_report_id: Some(true),
            }));
        }
        Ok(Some(info))
    }
    fn send_frame(&mut self, frame: &[u8], _report_id: u8, has_report_id: bool) -> bool {
        let Some(handle) = self.handle else {
            return false;
        };
        if has_report_id && frame.is_empty() {
            return false;
        }
        let mut data = frame.to_vec();
        if data.len() < self.out_len {
            data.resize(self.out_len, 0);
        } else if data.len() > self.out_len {
            data.truncate(self.out_len);
        }
        let mut written = 0u32;
        unsafe { WriteFile(handle, Some(&data), Some(&mut written), None).is_ok() }
    }
}

pub fn find_device_path() -> Result<Option<String>, String> {
    unsafe {
        let guid = HidD_GetHidGuid();
        let hdev: HDEVINFO = SetupDiGetClassDevsW(
            Some(&guid),
            PCWSTR::null(),
            None,
            DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
        )
        .map_err(|err| err.to_string())?;
        let mut iface = SP_DEVICE_INTERFACE_DATA {
            cbSize: sp_device_interface_data_cb_size(),
            ..Default::default()
        };
        let mut found = None;
        let mut enum_error = None;
        for index in 0..512u32 {
            if SetupDiEnumDeviceInterfaces(hdev, None, &guid, index, &mut iface).is_err() {
                let err = windows::Win32::Foundation::GetLastError();
                if err != ERROR_NO_MORE_ITEMS && err.0 != 0 {
                    enum_error = Some(err.0);
                }
                break;
            }
            let mut required = 0u32;
            let _ =
                SetupDiGetDeviceInterfaceDetailW(hdev, &iface, None, 0, Some(&mut required), None);
            if required == 0 {
                continue;
            }
            let mut buf = vec![0u8; required as usize];
            buf[..4].copy_from_slice(&hid_detail_cb_size().to_ne_bytes());
            if SetupDiGetDeviceInterfaceDetailW(
                hdev,
                &iface,
                Some(buf.as_mut_ptr() as *mut _),
                required,
                Some(&mut required),
                None,
            )
            .is_err()
            {
                continue;
            }
            let text = path_from_detail(&buf);
            let low = text.to_ascii_lowercase();
            if low.contains("vid_1189") && low.contains("pid_8890") && low.contains("mi_01") {
                found = Some(text);
                break;
            }
        }
        let _ = SetupDiDestroyDeviceInfoList(hdev);
        if found.is_none()
            && let Some(code) = enum_error
        {
            return Err(format!("SetupDiEnumDeviceInterfaces failed ({code})"));
        }
        Ok(found)
    }
}

fn path_from_detail(buf: &[u8]) -> String {
    if buf.len() < 6 {
        return String::new();
    }
    let wide = &buf[4..];
    let mut units = Vec::new();
    let mut i = 0;
    while i + 1 < wide.len() {
        let unit = u16::from_le_bytes([wide[i], wide[i + 1]]);
        if unit == 0 {
            break;
        }
        units.push(unit);
        i += 2;
    }
    OsString::from_wide(&units).to_string_lossy().into_owned()
}
