#!/usr/bin/env python3
# Quick inspection tool for attlib.dat segments (ATGTDF-1/2, ATGTSX, ATGTIX-2)
# 用来快速 dump 原始 word 流，辅助验证 Rust 解析逻辑。

import sys
import struct
from pathlib import Path

PAGE_SIZE = 2048
WORDS_PER_PAGE = 512
DATA_REGION_START = 0x1000
SEGMENT_POINTERS_OFFSET = 0x0800

PAGE_SWITCH_MARK = 0x00000000
SEGMENT_END_MARK = 0xFFFFFFFF


def read_u32_be(buf, offset):
    return int.from_bytes(buf[offset:offset + 4], "big")


def read_segment_pointers(f):
    f.seek(SEGMENT_POINTERS_OFFSET)
    raw = f.read(32)
    if len(raw) != 32:
        raise RuntimeError("failed to read segment pointers (32 bytes)")
    segs = [read_u32_be(raw, i * 4) for i in range(8)]
    return segs


def read_page(f, page_num):
    """Read one 2048-byte page as 512 big-endian u32 words."""
    offset = DATA_REGION_START + page_num * PAGE_SIZE
    f.seek(offset)
    buf = f.read(PAGE_SIZE)
    if len(buf) != PAGE_SIZE:
        raise RuntimeError(f"failed to read page {page_num} at offset 0x{offset:X}")
    return [read_u32_be(buf, i * 4) for i in range(WORDS_PER_PAGE)]


def dump_page_words(words, limit=64):
    for i, w in enumerate(words[:limit]):
        print(f"  {i:03}: 0x{w:08X} ({w})")


def inspect_atgtdf1(f, start_page, max_pages=2):
    print(f"\n=== Inspect ATGTDF-1 from page {start_page} (up to {max_pages} pages) ===")
    for p in range(start_page, start_page + max_pages):
        print(f"\n-- Page {p} --")
        words = read_page(f, p)
        dump_page_words(words, limit=64)


def inspect_atgtdf2(f, start_page, max_pages=2):
    print(f"\n=== Inspect ATGTDF-2 from page {start_page} (up to {max_pages} pages) ===")
    for p in range(start_page, start_page + max_pages):
        print(f"\n-- Page {p} --")
        words = read_page(f, p)
        dump_page_words(words, limit=64)


def decode_base27_raw(v: int) -> str:
    """纯 27 进制解码（不减 offset），主要用来看 pack_code / hash 形态。"""
    if v <= 0:
        return ""
    chars = []
    k = v
    while k > 0:
        c = k % 27
        if c == 0:
            chars.append("@")
        else:
            chars.append(chr(64 + c))  # 1->'A' ...
        k //= 27
    return "".join(reversed(chars))


def decode_db1_hash(hash_val: int) -> str:
    """模仿 Rust 的 decode_hash_to_name: base27 + 0x81BF1 offset。"""
    HASH_BASE_OFFSET = 0x81BF1
    HASH_UDA_THRESHOLD = 0x171FAD39
    if hash_val > HASH_UDA_THRESHOLD:
        # UDA，暂时不关心
        return ""
    if hash_val <= HASH_BASE_OFFSET:
        return ""
    k = hash_val - HASH_BASE_OFFSET
    chars = []
    while k > 0:
        c = k % 27
        chars.append(chr(64 + c))
        k //= 27
    return "".join(chars)


def inspect_atgtsx(f, start_page, max_records=32):
    print(f"\n=== Inspect ATGTSX from page {start_page} ===")
    page = start_page
    words = read_page(f, page)
    i = 0
    rec = 0
    extra_counts = {}
    while i < WORDS_PER_PAGE and rec < max_records:
        w = words[i]
        if w in (PAGE_SWITCH_MARK, SEGMENT_END_MARK, 0):
            print(f"  sentinel reached at word {i}: 0x{w:08X}")
            break
        if i + 2 >= WORDS_PER_PAGE:
            break
        attr = words[i]
        second = words[i + 1]
        third = words[i + 2]
        attr_name_db1 = decode_db1_hash(attr)
        attr_name_raw = decode_base27_raw(attr)
        second_db1 = decode_db1_hash(second)
        second_raw = decode_base27_raw(second)
        print(
            f"  [{rec:02}] attr=0x{attr:08X} db1='{attr_name_db1}' raw='{attr_name_raw}' "
            f" second=0x{second:08X} db1='{second_db1}' raw='{second_raw}' extra={third}"
        )
        extra_counts[third] = extra_counts.get(third, 0) + 1
        i += 3
        rec += 1

    if extra_counts:
        print("\nextra field frequency (value: count):")
        for val in sorted(extra_counts.keys()):
            print(f"  {val}: {extra_counts[val]}")


def inspect_atgtix2(f, start_page, max_records=16):
    print(f"\n=== Inspect ATGTIX-2 from page {start_page} ===")
    words = read_page(f, start_page)
    i = 0
    rec = 0
    while i < WORDS_PER_PAGE and rec < max_records:
        w = words[i]
        if w in (PAGE_SWITCH_MARK, SEGMENT_END_MARK, 0):
            print(f"  sentinel reached at word {i}: 0x{w:08X}")
            break
        noun_hash = w
        combined = words[i + 1] if i + 1 < WORDS_PER_PAGE else 0
        name = decode_db1_hash(noun_hash)
        print(f"  [{rec:02}] noun_hash=0x{noun_hash:08X} name='{name}' combined={combined}")
        i += 2
        rec += 1


def main():
    if len(sys.argv) < 2:
        print("Usage: attlib_debug.py <attlib.dat> [mode]")
        print("  modes: header | atgtdf2 | atgtsx | atgtix2 | all")
        return

    path = Path(sys.argv[1])
    mode = sys.argv[2] if len(sys.argv) > 2 else "all"

    if not path.is_file():
        print(f"file not found: {path}")
        return

    with path.open("rb") as f:
        segs = read_segment_pointers(f)
        print("段指针表:")
        for i, ptr in enumerate(segs):
            print(f"  段 {i}: 0x{ptr:08X} (页号: {ptr})")

        atgtdf1_page = segs[0]
        atgtsx_page = segs[3]
        atgtdf2_page = segs[4]
        atgtix2_page = segs[6]

        if mode == "header":
            # 专门 dump ATGTDF-1 若干页，不做任何 hash 过滤
            inspect_atgtdf1(f, atgtdf1_page, max_pages=2)
        elif mode == "all":
            print("\n=== Quick peek ATGTDF-1 first page ===")
            words = read_page(f, atgtdf1_page)
            dump_page_words(words, limit=32)

        if mode in ("atgtdf2", "all"):
            inspect_atgtdf2(f, atgtdf2_page, max_pages=2)

        if mode in ("atgtsx", "all"):
            inspect_atgtsx(f, atgtsx_page, max_records=32)

        if mode in ("atgtix2", "all"):
            inspect_atgtix2(f, atgtix2_page, max_records=16)


if __name__ == "__main__":
    main()
