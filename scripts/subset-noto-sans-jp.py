#!/usr/bin/env python3
"""Rebuild the bundled Noto Sans JP subset.

The committed file is rust/assets/fonts/NotoSansJP-Regular.subset.otf.gz.
Source face: Noto Sans JP Regular 2.004 (SIL OFL 1.1, Reserved Font Name
"Source"). Coverage is JIS X 0208 plus kana, fullwidth forms, and the
punctuation blocks used by the UI. The name table is kept, so the copyright
notice stays inside the font.
"""

from __future__ import annotations

import gzip
import pathlib
import subprocess
import sys
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parents[1]
DEST = ROOT / "rust" / "assets" / "fonts" / "NotoSansJP-Regular.subset.otf.gz"
SOURCE_URL = (
    "https://github.com/notofonts/noto-cjk/raw/main/Sans/SubsetOTF/JP/NotoSansJP-Regular.otf"
)


def characters() -> str:
    chars: set[str] = set()
    for start, end in (
        (0x20, 0x7E),
        (0xA0, 0xFF),
        (0x2000, 0x206F),
        (0x2190, 0x21FF),
        (0x2460, 0x24FF),
        (0x3000, 0x303F),
        (0x3040, 0x30FF),
        (0x31F0, 0x31FF),
        (0x3220, 0x3247),
        (0xFF00, 0xFFEF),
    ):
        chars.update(chr(cp) for cp in range(start, end + 1))
    for row in range(1, 85):
        for cell in range(1, 95):
            raw = bytes((0xA1 + row - 1, 0xA1 + cell - 1))
            try:
                chars.add(raw.decode("euc_jp"))
            except UnicodeDecodeError:
                pass
    return ",".join(f"U+{ord(ch):04X}" for ch in sorted(chars))


def main() -> int:
    source = pathlib.Path("/tmp/NotoSansJP-Regular.otf")
    if not source.is_file():
        source.write_bytes(urllib.request.urlopen(SOURCE_URL, timeout=60).read())
    raw_out = DEST.with_suffix("")  # drop .gz
    subprocess.check_call(
        [
            "pyftsubset",
            str(source),
            f"--unicodes={characters()}",
            f"--output-file={raw_out}",
            "--layout-features=",
            "--no-hinting",
            "--drop-tables+=DSIG,vhea,vmtx,VORG,BASE,GDEF,GPOS,GSUB",
            "--recommended-glyphs",
            "--name-IDs=*",
        ]
    )
    DEST.write_bytes(gzip.compress(raw_out.read_bytes(), compresslevel=9))
    raw_out.unlink()
    print(f"wrote {DEST} ({DEST.stat().st_size} bytes)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
