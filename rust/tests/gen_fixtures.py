#!/usr/bin/env python3
"""Dump golden outputs from the Python implementation for the Rust parity tests."""

import importlib.machinery
import importlib.util
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
SCRIPT = os.path.join(HERE, "..", "..", "linux", "t1-keyboard-config")

loader = importlib.machinery.SourceFileLoader("t1kb", SCRIPT)
spec = importlib.util.spec_from_loader("t1kb", loader)
t1 = importlib.util.module_from_spec(spec)
loader.exec_module(t1)


def hx(blob):
    return bytes(blob).hex()


cases = []


def add(name, **payload):
    payload["name"] = name
    cases.append(payload)


# --- key reports (tests/test_protocol.py + modifier combos) ---
samples = [
    ("plain_key", 1, 1, 3, [(0, 4)], 0),
    ("modifier_only", 5, 2, 3, [], 0x01),
    ("ctrl_combo", 1, 1, 3, [(0x01, 4)], 0),
    ("two_key_group", 3, 1, 3, [(0, 4), (0x02, 5)], 0),
    ("rid_zero", 1, 3, 0, [(0, 4)], 0),
    ("layer_nibble", 1, 3, 3, [(0, 4)], 0),
    ("cap_seven", 1, 1, 3, [(0, n) for n in range(4, 11)], 0),
]
for name, keynum, layer, rid, keys, lone in samples:
    reports = t1.key_reports(keynum, layer, rid, keys, lone)
    add(
        name,
        kind="key_reports",
        keynum=keynum,
        layer=layer,
        report_id=rid,
        keys=[[m, u] for m, u in keys],
        lone_mod=lone,
        reports=[hx(r) for r in reports],
    )

for label, mods, keynum, layer in (
    ("ctrl_c", 0x01, 1, 1),
    ("ctrl_shift_esc", 0x01 | 0x02, 8, 1),
):
    usage = t1.usage_for_label("C" if label == "ctrl_c" else "Esc")
    keys, lone = t1.editor_key_combo([], mods, usage)
    reports = t1.key_reports(keynum, layer, 3, keys, lone)
    add(
        label,
        kind="key_reports",
        keynum=keynum,
        layer=layer,
        report_id=3,
        keys=[[m, u] for m, u in keys],
        lone_mod=lone,
        reports=[hx(r) for r in reports],
        usage=usage,
        desc=t1.describe_key_combo(keys, lone),
    )

keys, lone = t1.editor_key_combo([], 0x01 | 0x04, 0)
add(
    "modifier_only_combo",
    kind="key_reports",
    keynum=3,
    layer=1,
    report_id=3,
    keys=[[m, u] for m, u in keys],
    lone_mod=lone,
    reports=[hx(r) for r in t1.key_reports(3, 1, 3, keys, lone)],
    desc=t1.describe_key_combo(keys, lone),
)

seq = [(0x01, t1.usage_for_label("C")), (0x04, t1.usage_for_label("Tab"))]
keys, lone = t1.editor_key_combo(seq, 0, 0)
add(
    "multi_group",
    kind="key_reports",
    keynum=4,
    layer=2,
    report_id=3,
    keys=[[m, u] for m, u in keys],
    lone_mod=lone,
    reports=[hx(r) for r in t1.key_reports(4, 2, 3, keys, lone)],
    desc=t1.describe_key_combo(keys, lone),
)

keys, lone = t1.editor_key_combo([(0x01, 6), (0x02, 41)], 0x04, 0)
add(
    "committed_groups",
    kind="key_reports",
    keynum=1,
    layer=1,
    report_id=3,
    keys=[[m, u] for m, u in keys],
    lone_mod=lone,
    reports=[hx(r) for r in t1.key_reports(1, 1, 3, keys, lone)],
)

for index in range(1, 13):
    reports = t1.assignment_reports(3, 1, 3, index)
    add(
        "shift_f%d" % index,
        kind="assignment_reports",
        keynum=3,
        layer=1,
        report_id=3,
        expression=index,
        reports=[hx(r) for r in reports],
    )

# write_key sequences (captured the same way the device does)
class Capture:
    def __init__(self, report_id):
        self.report_id = report_id
        self.had_report_id = report_id != 0
        self.data_len = 64
        self.sent = []
        self.is_open = lambda: True

    def write_payload(self, payload):
        self.sent.append(bytes(payload))
        return True

    def switch_layer(self, layer):
        return self.write_payload(t1.layer_report(layer))

    def write_many(self, payloads):
        for payload in payloads:
            if not self.write_payload(payload):
                return False
        return True

    def write_key(self, keynum, layer, keys, lone_mod=0):
        if self.report_id != 0 and not self.switch_layer(layer):
            return False
        if not self.write_many(t1.key_reports(keynum, layer, self.report_id, keys, lone_mod)):
            return False
        return self.write_payload(t1.flash_report())


cap = Capture(3)
keys, lone = t1.editor_key_combo([], 0x01, t1.usage_for_label("C"))
cap.write_key(1, 2, keys, lone)
add("write_key_ctrl_c_layer2", kind="payloads", payloads=[hx(p) for p in cap.sent])

cap0 = Capture(0)
cap0.write_key(5, 3, [], 0x01)
add("write_key_rid0", kind="payloads", payloads=[hx(p) for p in cap0.sent])

for action in ("play", "prev", "next", "mute", "vol_up", "vol_down"):
    for rid in (3, 0, 2):
        add(
            "media_%s_rid%d" % (action, rid),
            kind="media",
            keynum=1,
            layer=1,
            report_id=rid,
            action=action,
            report=hx(t1.media_report(1, 1, rid, action)),
        )
add("media_mute_layer", kind="media", keynum=2, layer=2, report_id=3, action="mute",
    report=hx(t1.media_report(2, 2, 3, "mute")))

for action in t1.MOUSE_PAYLOAD:
    add(
        "mouse_%s" % action,
        kind="mouse",
        keynum=1,
        layer=1,
        report_id=3,
        action=action,
        report=hx(t1.mouse_report(1, 1, 3, action)),
    )

add("led", kind="led", layer=1, report_id=3, mode=2, report=hx(t1.led_report(1, 3, 2)))
add("layer1", kind="raw", report=hx(t1.layer_report(1)))
add("layer9", kind="raw", report=hx(t1.layer_report(9)))
add("flash", kind="raw", report=hx(t1.flash_report()))
add("flash_led", kind="raw", report=hx(t1.flash_led_report()))
add("probe", kind="raw", report=hx(t1.probe_report()))

dev = t1.T1Device()
dev.report_id = 3
dev.data_len = 64
dev.had_report_id = True
add("frame_with_id", kind="frame", frame=hx(dev.frame(bytes([0xAA, 0xAA]))))
dev.report_id = 0
dev.data_len = 8
dev.had_report_id = False
add("frame_without_id", kind="frame", frame=hx(dev.frame(bytes([1, 2, 3]))))
dev.report_id = 3
dev.data_len = 4
dev.had_report_id = True
add("frame_truncate", kind="frame", frame=hx(dev.frame(bytes(range(20)))))

descriptors = {
    "with_id": bytes([0x85, 3, 0x95, 0x40, 0x75, 0x08, 0x91, 0x02]),
    "without_id": bytes([0x95, 8, 0x75, 8, 0x91, 0x02]),
    "input_ignored": bytes([0x85, 5, 0x95, 8, 0x75, 8, 0x81, 0x02]),
    "multi_count": bytes([0x96, 0x00, 0x01, 0x75, 0x08, 0x91, 0x02]),
}
parsed = {}
for name, data in descriptors.items():
    result = t1.parse_report_descriptor(data)
    parsed[name] = None if result is None else {str(k): v for k, v in result["outputs"].items()}


def config_descriptor():
    def interface(number, hid_length):
        return (
            bytes([9, 0x04, number, 0, 1, 3, 1, 1, 0])
            + bytes([9, 0x21, 0x11, 0x01, 0, 1, 0x22, hid_length & 0xFF, hid_length >> 8])
            + bytes([7, 0x05, 0x81, 3, 8, 0, 10])
        )

    body = interface(0, 64) + interface(1, 60) + interface(2, 70)
    head = bytes([9, 2]) + (9 + len(body)).to_bytes(2, "little") + bytes([3, 1, 0, 0x80, 50])
    return head + body


cfg = config_descriptor()
lengths = {str(k): v for k, v in t1.hid_descriptor_lengths(cfg).items()}
short = {str(k): v for k, v in t1.hid_descriptor_lengths(cfg[:12]).items()}
bogus = bytes([9, 0x04, 1, 0, 1, 3, 1, 1, 0]) + bytes([9, 0x21, 0x11, 0x01, 0, 1, 0x22, 4, 0])
bogus_len = {str(k): v for k, v in t1.hid_descriptor_lengths(bogus).items()}

out = {
    "cases": cases,
    "descriptors": parsed,
    "config_lengths": lengths,
    "config_truncated": short,
    "config_bogus": bogus_len,
    "ioctl": {
        "USBDEVFS_SUBMITURB": t1.USBDEVFS_SUBMITURB,
        "USBDEVFS_REAPURBNDELAY": t1.USBDEVFS_REAPURBNDELAY,
        "USBDEVFS_CONTROL": t1.USBDEVFS_CONTROL,
        "USBDEVFS_CLAIMINTERFACE": t1.USBDEVFS_CLAIMINTERFACE,
        "urb_size": 56,
        "ctrl_size": 24,
    },
    "usages": {name: code for name, code in t1.USAGE_KEYS},
}

dest = os.path.join(HERE, "fixtures", "protocol.json")
os.makedirs(os.path.dirname(dest), exist_ok=True)
with open(dest, "w", encoding="utf-8") as fh:
    json.dump(out, fh, indent=2)
    fh.write("\n")
print(dest, "cases", len(cases))
