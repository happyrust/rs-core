#!/usr/bin/env python3
"""
仅基于 attlib.dat 的 ATGTSX 语法表推导 noun -> attributes（“可应用”属性集合）。
注意：真实运行时的属性列表来自 DAB 的 PRDISP 字段，本脚本是基于语法表的近似。

用法示例：
    python atgtsx_noun_attrs.py data/attlib.dat atgtsx_noun_attrs.csv
"""

import csv
import struct
import sys
from pathlib import Path
from typing import Dict, List, Tuple

PAGE_SIZE = 2048
WORDS_PER_PAGE = 512
DATA_REGION_START = 0x1000
SEGMENT_POINTERS_OFFSET = 0x800
PAGE_SWITCH = 0x00000000
SEGMENT_END = 0xFFFFFFFF
MIN_HASH = 0x81BF2
MAX_HASH = 0x171FAD39
BASE27_OFFSET = 0x81BF1


def decode_base27(hash_val: int) -> str:
    if hash_val < MIN_HASH or hash_val > MAX_HASH:
        return ""
    k = hash_val - BASE27_OFFSET
    chars = []
    while k > 0:
        c = k % 27
        chars.append(" " if c == 0 else chr(c + 64))
        k //= 27
    return "".join(chars)


def read_segment_pointers(fh) -> List[int]:
    fh.seek(SEGMENT_POINTERS_OFFSET)
    return list(struct.unpack(">8I", fh.read(32)))


def load_pages(fh) -> List[List[int]]:
    fh.seek(DATA_REGION_START)
    pages: List[List[int]] = []
    while True:
        buf = fh.read(PAGE_SIZE)
        if len(buf) < PAGE_SIZE:
            break
        pages.append(list(struct.unpack(">512I", buf)))
    return pages


def parse_atgtsx(pages: List[List[int]], start_page: int) -> List[Tuple[int, int, int]]:
    """返回 (attr_hash, noun_hash, extra_info) 列表。"""
    page = start_page
    idx = 0
    entries: List[Tuple[int, int, int]] = []
    while True:
        if page >= len(pages):
            break
        if idx >= WORDS_PER_PAGE:
            page += 1
            idx = 0
            continue
        word = pages[page][idx]
        idx += 1
        if word == PAGE_SWITCH:
            page += 1
            idx = 0
            continue
        if word == SEGMENT_END or word == 0:
            break
        if word < MIN_HASH or word > MAX_HASH:
            continue
        attr_hash = word
        noun_hash = pages[page][idx]
        extra = pages[page][idx + 1]
        idx += 2
        entries.append((attr_hash, noun_hash, extra))
    return entries


def aggregate(entries: List[Tuple[int, int, int]]) -> Dict[int, List[Tuple[int, int]]]:
    """noun_hash -> [(attr_hash, extra_info)]"""
    mapping: Dict[int, List[Tuple[int, int]]] = {}
    for attr, noun, extra in entries:
        mapping.setdefault(noun, []).append((attr, extra))
    return mapping


def main():
    if len(sys.argv) not in (2, 3):
        print(f"用法: {Path(sys.argv[0]).name} attlib.dat [out.csv]")
        sys.exit(1)
    attlib_path = Path(sys.argv[1])
    out_path = Path(sys.argv[2]) if len(sys.argv) == 3 else Path("atgtsx_noun_attrs.csv")

    with attlib_path.open("rb") as fh:
        ptrs = read_segment_pointers(fh)
        atgtsx_page = ptrs[3]
        if atgtsx_page == 0:
            print("未找到 ATGTSX 段指针")
            sys.exit(1)
        pages = load_pages(fh)

    entries = parse_atgtsx(pages, atgtsx_page)
    mapping = aggregate(entries)

    with out_path.open("w", newline="") as out:
        writer = csv.writer(out)
        writer.writerow(["noun_hash", "noun_name", "attr_hashes", "attr_names", "count"])
        for noun, attrs in mapping.items():
            attr_hashes = [a for a, _ in attrs]
            writer.writerow(
                [
                    f"0x{noun:08X}",
                    decode_base27(noun),
                    ";".join(f"0x{x:08X}" for x in attr_hashes),
                    ";".join(decode_base27(x) for x in attr_hashes),
                    len(attr_hashes),
                ]
            )
    print(f"完成，ATGTSX 起始页 {atgtsx_page}，记录条目 {len(entries)}，输出 {out_path}")


if __name__ == "__main__":
    main()
