//! Byte-for-byte checks against fixtures dumped from the Python 1.4.0 encoder.

use std::collections::BTreeMap;

use serde_json::Value;
use t1_keyboard_config::hidparse;
use t1_keyboard_config::keys::{self};
use t1_keyboard_config::protocol::{self, UsbdevfsCtrl, UsbdevfsUrb};
use t1_keyboard_config::tables::{self, EV_KEY, EV_REL, KERNEL_EV_KEY, KERNEL_EV_REL};

fn fixture() -> Value {
    let text = include_str!("fixtures/protocol.json");
    serde_json::from_str(text).expect("protocol.json")
}

fn hex_of(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn pairs(value: &Value) -> Vec<(u8, u8)> {
    value
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .map(|row| {
            let row = row.as_array().expect("pair");
            (
                row[0].as_u64().unwrap() as u8,
                row[1].as_u64().unwrap() as u8,
            )
        })
        .collect()
}

fn u8_field(value: &Value, key: &str) -> u8 {
    value.get(key).and_then(|v| v.as_u64()).unwrap() as u8
}

#[test]
fn hid_reports_match_python() {
    let doc = fixture();
    for case in doc["cases"].as_array().expect("cases") {
        let name = case["name"].as_str().unwrap();
        let kind = case["kind"].as_str().unwrap();
        match kind {
            "key_reports" => {
                let keys = pairs(&case["keys"]);
                let reports = protocol::key_reports(
                    u8_field(case, "keynum"),
                    u8_field(case, "layer"),
                    u8_field(case, "report_id"),
                    &keys,
                    u8_field(case, "lone_mod"),
                );
                let got: Vec<String> = reports.iter().map(|r| hex_of(r)).collect();
                let expect: Vec<String> = case["reports"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_str().unwrap().to_string())
                    .collect();
                assert_eq!(got, expect, "{name}");
            }
            "payloads" => {
                let got = match name {
                    "write_key_ctrl_c_layer2" => {
                        let usage = keys::usage_for_label("C");
                        let (keys, lone) = keys::editor_key_combo(&[], 0x01, usage);
                        protocol::write_key_payloads(3, 1, 2, &keys, lone)
                    }
                    "write_key_rid0" => protocol::write_key_payloads(0, 5, 3, &[], 0x01),
                    other => panic!("unknown payload {other}"),
                };
                let got: Vec<String> = got.iter().map(|p| hex_of(p)).collect();
                let expect: Vec<String> = case["payloads"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_str().unwrap().to_string())
                    .collect();
                assert_eq!(got, expect, "{name}");
            }
            "media" => {
                let report = protocol::media_report(
                    u8_field(case, "keynum"),
                    u8_field(case, "layer"),
                    u8_field(case, "report_id"),
                    case["action"].as_str().unwrap(),
                )
                .unwrap_or_else(|| panic!("{name}"));
                assert_eq!(hex_of(&report), case["report"].as_str().unwrap(), "{name}");
            }
            "mouse" => {
                let report = protocol::mouse_report(
                    u8_field(case, "keynum"),
                    u8_field(case, "layer"),
                    u8_field(case, "report_id"),
                    case["action"].as_str().unwrap(),
                )
                .unwrap_or_else(|| panic!("{name}"));
                assert_eq!(hex_of(&report), case["report"].as_str().unwrap(), "{name}");
            }
            "led" => {
                let report = protocol::led_report(
                    u8_field(case, "layer"),
                    u8_field(case, "report_id"),
                    u8_field(case, "mode"),
                );
                assert_eq!(hex_of(&report), case["report"].as_str().unwrap(), "{name}");
            }
            "raw" => {
                let report: Vec<u8> = match name {
                    "layer1" => protocol::layer_report(1).to_vec(),
                    "layer9" => protocol::layer_report(9).to_vec(),
                    "flash" => protocol::flash_report().to_vec(),
                    "flash_led" => protocol::flash_led_report().to_vec(),
                    "probe" => protocol::probe_report().to_vec(),
                    other => panic!("unknown raw {other}"),
                };
                assert_eq!(hex_of(&report), case["report"].as_str().unwrap(), "{name}");
            }
            "frame" => {
                let (payload, rid, len, had) = match name {
                    "frame_with_id" => (vec![0xAAu8, 0xAA], 3u8, 64usize, true),
                    "frame_without_id" => (vec![1, 2, 3], 0, 8, false),
                    "frame_truncate" => ((0..20).collect(), 3, 4, true),
                    other => panic!("unknown frame {other}"),
                };
                let frame = protocol::frame(&payload, rid, len, had);
                assert_eq!(hex_of(&frame), case["frame"].as_str().unwrap(), "{name}");
            }
            other => panic!("unknown kind {other}"),
        }
    }
}

#[test]
fn descriptors_and_ioctl_match_python() {
    let doc = fixture();
    let samples: [(&str, &[u8]); 4] = [
        ("with_id", &[0x85, 3, 0x95, 0x40, 0x75, 0x08, 0x91, 0x02]),
        ("without_id", &[0x95, 8, 0x75, 8, 0x91, 0x02]),
        ("input_ignored", &[0x85, 5, 0x95, 8, 0x75, 8, 0x81, 0x02]),
        ("multi_count", &[0x96, 0x00, 0x01, 0x75, 0x08, 0x91, 0x02]),
    ];
    for (name, bytes) in samples {
        let parsed = hidparse::parse_report_descriptor(bytes);
        let expect = &doc["descriptors"][name];
        if expect.is_null() {
            assert!(parsed.is_none(), "{name}");
            continue;
        }
        let got = parsed.unwrap_or_else(|| panic!("{name}"));
        let mut expect_map = BTreeMap::new();
        for (key, value) in expect.as_object().unwrap() {
            expect_map.insert(key.parse::<u8>().unwrap(), value.as_u64().unwrap() as u32);
        }
        assert_eq!(got, expect_map, "{name}");
    }

    let cfg = sample_config();
    assert_eq!(
        length_map(&hidparse::hid_descriptor_lengths(&cfg)),
        json_u16_map(&doc["config_lengths"])
    );
    assert!(hidparse::hid_descriptor_lengths(&cfg[..12]).is_empty());
    let bogus = [
        9u8, 0x04, 1, 0, 1, 3, 1, 1, 0, 9, 0x21, 0x11, 0x01, 0, 1, 0x22, 4, 0,
    ];
    assert!(hidparse::hid_descriptor_lengths(&bogus).is_empty());

    let ioctl = &doc["ioctl"];
    assert_eq!(
        protocol::usbfs_submiturb(),
        ioctl["USBDEVFS_SUBMITURB"].as_u64().unwrap() as u32
    );
    assert_eq!(
        protocol::usbfs_reapurbndelay(),
        ioctl["USBDEVFS_REAPURBNDELAY"].as_u64().unwrap() as u32
    );
    assert_eq!(
        protocol::usbfs_control(),
        ioctl["USBDEVFS_CONTROL"].as_u64().unwrap() as u32
    );
    assert_eq!(
        protocol::usbfs_claiminterface(),
        ioctl["USBDEVFS_CLAIMINTERFACE"].as_u64().unwrap() as u32
    );
    assert_eq!(
        std::mem::size_of::<UsbdevfsUrb>(),
        ioctl["urb_size"].as_u64().unwrap() as usize
    );
    assert_eq!(
        std::mem::size_of::<UsbdevfsCtrl>(),
        ioctl["ctrl_size"].as_u64().unwrap() as usize
    );
}

#[test]
fn linux_kernel_events_normalize_to_app_types() {
    assert_eq!(tables::normalize_kernel_type(KERNEL_EV_KEY), Some(EV_KEY));
    assert_eq!(tables::normalize_kernel_type(KERNEL_EV_REL), Some(EV_REL));
    assert_eq!(EV_KEY, 2);
    assert_eq!(EV_REL, 3);
    assert_eq!(KERNEL_EV_KEY, 1);
    assert_eq!(KERNEL_EV_REL, 2);
}

fn sample_config() -> Vec<u8> {
    fn interface(number: u8, hid_length: u16) -> Vec<u8> {
        let mut out = vec![9, 0x04, number, 0, 1, 3, 1, 1, 0];
        out.extend([
            9,
            0x21,
            0x11,
            0x01,
            0,
            1,
            0x22,
            (hid_length & 0xff) as u8,
            (hid_length >> 8) as u8,
        ]);
        out.extend([7, 0x05, 0x81, 3, 8, 0, 10]);
        out
    }
    let body = [interface(0, 64), interface(1, 60), interface(2, 70)].concat();
    let total = (9 + body.len()) as u16;
    let mut head = vec![
        9,
        2,
        (total & 0xff) as u8,
        (total >> 8) as u8,
        3,
        1,
        0,
        0x80,
        50,
    ];
    head.extend(body);
    head
}

fn length_map(map: &BTreeMap<u8, u16>) -> BTreeMap<u8, u16> {
    map.clone()
}

fn json_u16_map(value: &Value) -> BTreeMap<u8, u16> {
    value
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| (k.parse().unwrap(), v.as_u64().unwrap() as u16))
        .collect()
}
