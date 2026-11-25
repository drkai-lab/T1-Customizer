#!/usr/bin/env python3
"""Rebuild the bundled Noto Sans KR subset (Hangul for the Korean window).

The committed file is rust/assets/fonts/NotoSansKR-Regular.subset.otf.gz.
Source face: Noto Sans KR Regular 2.004 (SIL OFL 1.1, © 2014-2021 Adobe,
Reserved Font Name "Source"; same license text as rust/assets/fonts/OFL.txt).
Coverage is the 2,350 Hangul syllables of KS X 1001, the Hangul
compatibility jamo, and every Hangul character quoted in
rust/src/gui/i18n_ko.rs. Kana, kanji, Latin and punctuation come from the
Japanese face and egui's own fonts, so they are left out. The name table is
kept, so the copyright notice stays inside the font.
"""

from __future__ import annotations

import gzip
import pathlib
import re
import subprocess
import sys
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parents[1]
DEST = ROOT / "rust" / "assets" / "fonts" / "NotoSansKR-Regular.subset.otf.gz"
KO_SOURCE = ROOT / "rust" / "src" / "gui" / "i18n_ko.rs"
SOURCE_URL = (
    "https://github.com/notofonts/noto-cjk/raw/main/Sans/SubsetOTF/KR/NotoSansKR-Regular.otf"
)


def is_hangul(ch: str) -> bool:
    cp = ord(ch)
    return 0xAC00 <= cp <= 0xD7A3 or 0x3131 <= cp <= 0x318E


def characters() -> str:
    chars: set[str] = {" "}
    chars.update(chr(cp) for cp in range(0x3131, 0x318F))
    # KS X 1001 Hangul rows (EUC-KR lead bytes 0xB0..0xC8).
    for lead in range(0xB0, 0xC9):
        for trail in range(0xA1, 0xFF):
            try:
                chars.add(bytes((lead, trail)).decode("euc_kr"))
            except UnicodeDecodeError:
                pass
    for text in re.findall(r'"((?:[^"\\]|\\.)*)"', KO_SOURCE.read_text(encoding="utf-8")):
        chars.update(ch for ch in text if is_hangul(ch))
    return ",".join(f"U+{ord(ch):04X}" for ch in sorted(chars))


def main() -> int:
    source = pathlib.Path("/tmp/NotoSansKR-Regular.otf")
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
            "--desubroutinize",
            "--drop-tables+=DSIG,vhea,vmtx,VORG,BASE,GDEF,GPOS,GSUB",
            "--name-IDs=*",
        ]
    )
    DEST.write_bytes(gzip.compress(raw_out.read_bytes(), compresslevel=9, mtime=0))
    raw_out.unlink()
    print(f"wrote {DEST} ({DEST.stat().st_size} bytes)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
