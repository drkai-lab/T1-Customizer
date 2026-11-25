//! Minimal HID report-descriptor and configuration-descriptor parsers.

use std::collections::BTreeMap;

/// Output report bit lengths keyed by report id. `None` when there is no output item.
pub fn parse_report_descriptor(data: &[u8]) -> Option<BTreeMap<u8, u32>> {
    let mut outputs: BTreeMap<u8, u32> = BTreeMap::new();
    let mut rid: u32 = 0;
    let mut rsize: u32 = 0;
    let mut rcount: u32 = 0;
    let mut pos = 0;
    let size = data.len();
    while pos < size {
        let prefix = data[pos];
        pos += 1;
        if prefix == 0xFE {
            if pos + 2 > size {
                break;
            }
            let long_size = data[pos] as usize;
            pos += 3 + long_size;
            continue;
        }
        let mut item_size = (prefix & 0x03) as usize;
        if item_size == 3 {
            item_size = 4;
        }
        if pos + item_size > size {
            break;
        }
        let mut value = 0u32;
        for (i, byte) in data[pos..pos + item_size].iter().enumerate() {
            value |= u32::from(*byte) << (8 * i);
        }
        pos += item_size;
        let btype = (prefix >> 2) & 0x03;
        let tag = prefix >> 4;
        if btype == 1 {
            if tag == 8 {
                rid = value;
            } else if tag == 7 {
                rsize = value;
            } else if tag == 9 {
                rcount = value;
            }
        } else if btype == 0 && tag == 9 {
            let bits = outputs.get(&(rid as u8)).copied().unwrap_or(0);
            outputs.insert(rid as u8, bits + rsize * rcount);
        }
    }
    if outputs.is_empty() {
        None
    } else {
        Some(outputs)
    }
}

/// `{interface_number: wDescriptorLength}` for HID class descriptors (0x21).
pub fn hid_descriptor_lengths(config_data: &[u8]) -> BTreeMap<u8, u16> {
    let mut lengths = BTreeMap::new();
    let mut interface: i32 = -1;
    let mut pos = 0;
    let size = config_data.len();
    while pos + 2 <= size {
        let length = config_data[pos] as usize;
        let dtype = config_data[pos + 1];
        if length < 2 || pos + length > size {
            break;
        }
        if dtype == 0x04 && length >= 4 {
            interface = i32::from(config_data[pos + 2]);
        } else if dtype == 0x21 && length >= 9 && interface >= 0 {
            let iface = interface as u8;
            if let std::collections::btree_map::Entry::Vacant(e) = lengths.entry(iface) {
                let report_len =
                    u16::from(config_data[pos + 7]) | (u16::from(config_data[pos + 8]) << 8);
                if (16..=1024).contains(&report_len) {
                    e.insert(report_len);
                }
            }
        }
        pos += length;
    }
    lengths
}

/// Pick the output report the device should use. Mirrors the Python choice.
pub fn choose_output(outputs: &BTreeMap<u8, u32>) -> Option<(u8, usize, bool)> {
    if outputs.is_empty() {
        return None;
    }
    let (rid, bits) = if outputs.len() == 1 {
        outputs.iter().next().map(|(k, v)| (*k, *v))?
    } else {
        let rid = outputs.keys().find(|id| **id != 0).copied().unwrap_or(0);
        (rid, outputs[&rid])
    };
    let data_bytes = bits.div_ceil(8).max(1) as usize;
    Some((rid, data_bytes, rid != 0))
}
