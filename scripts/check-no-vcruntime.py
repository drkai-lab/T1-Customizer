#!/usr/bin/env python3
"""Fail if a Windows exe imports the VC++ or Universal CRT dynamically.

A static CRT (`target-feature=+crt-static`) must not list VCRUNTIME140.dll
or api-ms-win-crt-* in the PE import table.
"""

from __future__ import annotations

import struct
import sys
from pathlib import Path


def pe_imports(path: Path) -> list[str]:
    data = path.read_bytes()
    if data[:2] != b"MZ":
        raise SystemExit(f"{path}: not a PE file")
    e_lfanew = struct.unpack_from("<I", data, 0x3C)[0]
    if data[e_lfanew : e_lfanew + 4] != b"PE\0\0":
        raise SystemExit(f"{path}: bad PE signature")
    coff = e_lfanew + 4
    _machine, nsections, _ts, _sym, _nsym, size_opt, _chars = struct.unpack_from(
        "<HHIIIHH", data, coff
    )
    opt = coff + 20
    magic = struct.unpack_from("<H", data, opt)[0]
    if magic == 0x20B:
        dd = opt + 112
    elif magic == 0x10B:
        dd = opt + 96
    else:
        raise SystemExit(f"{path}: unknown optional header {magic:#x}")
    import_rva, _import_size = struct.unpack_from("<II", data, dd + 8)
    sections: list[tuple[int, int, int]] = []
    sec_off = opt + size_opt
    for index in range(nsections):
        _name, _vsize, va, rawsize, rawptr = struct.unpack_from(
            "<8sIIII", data, sec_off + index * 40
        )
        sections.append((va, rawptr, rawsize))

    def rva_to_off(rva: int) -> int:
        for va, rawptr, rawsize in sections:
            if va <= rva < va + max(rawsize, 1):
                return rawptr + (rva - va)
        raise SystemExit(f"{path}: RVA {rva:#x} is outside every section")

    if import_rva == 0:
        return []
    names: list[str] = []
    cursor = rva_to_off(import_rva)
    while True:
        _ilt, _time, _forward, name_rva, iat = struct.unpack_from("<IIIII", data, cursor)
        if name_rva == 0 and iat == 0:
            break
        start = rva_to_off(name_rva)
        end = data.index(b"\0", start)
        names.append(data[start:end].decode("ascii", "replace"))
        cursor += 20
    return names


def forbidden(name: str) -> bool:
    lowered = name.lower()
    return lowered.startswith("vcruntime") or lowered.startswith("api-ms-win-crt-")


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        print("usage: check-no-vcruntime.py EXE [EXE...]", file=sys.stderr)
        return 2
    failed = False
    for arg in argv[1:]:
        path = Path(arg)
        libs = pe_imports(path)
        bad = [name for name in libs if forbidden(name)]
        print(f"{path}:")
        for name in libs:
            print(f"  {name}")
        if bad:
            failed = True
            print(f"dynamic CRT: {', '.join(bad)}", file=sys.stderr)
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
