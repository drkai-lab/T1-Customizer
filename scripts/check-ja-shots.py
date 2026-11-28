#!/usr/bin/env python3
"""Fail if the Japanese UI capture is missing or blank.

The glyph check itself is `T1_FONT_CHECK` (`font-check.txt`). This script
checks that the software-rendered proof PNG was written. The default mode
also requires every tab and the tray note.

`--font-only` checks the proof and `font-check.txt` without window shots.
`--bundled` also requires `source=fallback:`, so a system CJK font cannot
hide a missing glyph in the bundled subset.
"""

from __future__ import annotations

import struct
import sys
from pathlib import Path

UI_SHOTS = ["tray.png"] + [f"tab-{index}.png" for index in range(6)]


def png_size(path: Path) -> tuple[int, int]:
    data = path.read_bytes()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise SystemExit(f"{path} is not a PNG")
    kind = data[12:16]
    if kind != b"IHDR":
        raise SystemExit(f"{path} has no IHDR")
    width, height = struct.unpack(">II", data[16:24])
    return width, height


def check_font(root: Path, bundled: bool) -> str:
    note = root / "font-check.txt"
    if not note.is_file():
        raise SystemExit(f"missing {note}")
    text = note.read_text(encoding="utf-8")
    if "japanese-font: ok" not in text:
        raise SystemExit(text)
    if bundled and "source=fallback:" not in text:
        raise SystemExit(f"expected the bundled subset, got:\n{text}")
    proof = root / "font-proof.png"
    if not proof.is_file():
        raise SystemExit(f"missing {proof}")
    proof_w, proof_h = png_size(proof)
    if proof.stat().st_size < 2_000 or proof_w < 400 or proof_h < 80:
        raise SystemExit(f"font proof looks empty: {proof_w}x{proof_h}")
    print(f"ok font-proof.png {proof_w}x{proof_h}")
    return text


def main() -> int:
    args = sys.argv[1:]
    font_only = "--font-only" in args
    bundled = "--bundled" in args
    args = [arg for arg in args if arg not in ("--font-only", "--bundled")]
    if len(args) != 1:
        raise SystemExit(
            f"usage: {sys.argv[0]} [--font-only] [--bundled] SHOT_DIR"
        )
    root = Path(args[0])
    text = check_font(root, bundled)
    if not font_only:
        for name in UI_SHOTS:
            path = root / name
            if not path.is_file():
                raise SystemExit(f"missing {path}")
            width, height = png_size(path)
            if path.stat().st_size < 12_000 or width < 700 or height < 400:
                raise SystemExit(
                    f"{name} looks blank: {width}x{height} {path.stat().st_size} bytes"
                )
            print(f"ok {name} {width}x{height}")
    print(text.strip())
    return 0


if __name__ == "__main__":
    sys.exit(main())
