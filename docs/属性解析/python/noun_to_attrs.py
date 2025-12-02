#!/usr/bin/env python3
"""
从 attlib.dat + DAB 数据库文件提取 noun -> attributes (PRDISP) 列表。

假设格式与 IDA 反汇编一致：
- 页大小 2048B（512 x u32，大端）
- 数据区起点默认 0x1000，可通过 --data-offset 覆盖
- 页切换标记 0，段结束标记 0xFFFFFFFF

用法示例：
    python noun_to_attrs.py --attlib data/attlib.dat --dab /path/to/db.dab --out noun_attrs.csv
"""

import argparse
import csv
import struct
from pathlib import Path
from typing import Dict, Iterable, List, Optional, Tuple

PAGE_SIZE = 2048
WORDS_PER_PAGE = 512
PAGE_SWITCH = 0x00000000
SEGMENT_END = 0xFFFFFFFF
MIN_HASH = 0x81BF2
MAX_HASH = 0x171FAD39
BASE27_OFFSET = 0x81BF1
PRDISP_HASH = 0x0E5416D9

# 常见锚点，用于扫描 ATGTIX-2 起始页（确保包含真实 noun）
ANCHOR_HASHES = {0x000CA439, 0x0009D65A}  # PIPE, SITE


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


def read_pages(path: Path, data_offset: int) -> List[List[int]]:
    pages: List[List[int]] = []
    with path.open("rb") as fh:
        fh.seek(data_offset)
        while True:
            buf = fh.read(PAGE_SIZE)
            if len(buf) < PAGE_SIZE:
                break
            pages.append(list(struct.unpack(">512I", buf)))
    return pages


def parse_atgtix2(pages: List[List[int]]) -> Tuple[Dict[int, Tuple[int, int]], int]:
    """扫描 ATGTIX-2，返回 noun_hash -> (page, offset) 映射及起始页。"""
    best_page = -1
    best_records: List[Tuple[int, int, int, int]] = []
    for page in range(len(pages)):
        if not any(MIN_HASH <= w <= MAX_HASH for w in pages[page]):
            continue
        recs, ended = parse_index_from_page(pages, page)
        if not ended:
            continue
        if not any(h in ANCHOR_HASHES for h, *_ in recs):
            continue
        if len(recs) > len(best_records):
            best_page = page
            best_records = recs
    mapping: Dict[int, Tuple[int, int]] = {}
    for h, page_num, off, _ in best_records:
        mapping[h] = (page_num, off)
    return mapping, best_page


def parse_index_from_page(
    pages: List[List[int]], start_page: int
) -> Tuple[List[Tuple[int, int, int, int]], bool]:
    records: List[Tuple[int, int, int, int]] = []
    page = start_page
    idx = 0
    ended = False
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
        if word == SEGMENT_END:
            ended = True
            break
        if word < MIN_HASH or word > MAX_HASH:
            continue
        combined = pages[page][idx]
        idx += 1
        page_num = combined // WORDS_PER_PAGE
        offset = combined % WORDS_PER_PAGE
        records.append((word, page_num, offset, combined))
    return records, ended


def parse_atgtdf2(pages: List[List[int]], start_page: int) -> Dict[int, int]:
    """解析 ATGTDF-2，返回 field_hash -> slot 序号。"""
    field_idx: Dict[int, int] = {}
    page = start_page
    idx = 0
    slot = 0
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
        if word == SEGMENT_END:
            break
        if word < MIN_HASH or word > MAX_HASH:
            continue
        field_hash = word
        # type
        if idx >= WORDS_PER_PAGE:
            page += 1
            idx = 0
            continue
        idx += 1  # skip type
        # flag / reserved
        idx += 1
        field_idx[field_hash] = slot
        slot += 1
    return field_idx


def read_prdisp(
    dab_pages: List[List[int]],
    noun_index: Dict[int, Tuple[int, int]],
    field_idx2: Dict[int, int],
    noun_hash: int,
    field_hash: int = PRDISP_HASH,
) -> Optional[List[int]]:
    idx = noun_index.get(noun_hash)
    if idx is None:
        return None
    slot = field_idx2.get(field_hash)
    if slot is None:
        return None
    page_num, base_off = idx
    if page_num >= len(dab_pages):
        return None
    words = dab_pages[page_num]
    offset = base_off + slot
    if offset >= len(words):
        return None
    count = words[offset]
    start = offset + 1
    end = start + count
    if end > len(words):
        return None
    return words[start:end]


def main():
    ap = argparse.ArgumentParser(description="Extract noun -> attributes via ATGTIX-2/ATGTDF-2/PRDISP")
    ap.add_argument("--attlib", required=True, type=Path, help="path to attlib.dat")
    ap.add_argument("--dab", required=True, type=Path, help="path to DAB database file")
    ap.add_argument("--out", required=True, type=Path, help="output CSV")
    ap.add_argument("--data-offset", default="0x1000", help="data region offset (hex or int), default 0x1000")
    ap.add_argument("--field-hash", default=f"0x{PRDISP_HASH:08X}", help="field hash (default PRDISP)")
    args = ap.parse_args()

    data_offset = int(args.data_offset, 0)
    field_hash = int(args.field_hash, 0)

    attlib_pages = read_pages(args.attlib, data_offset)
    noun_index, start_page = parse_atgtix2(attlib_pages)
    if start_page < 0:
        raise SystemExit("未找到 ATGTIX-2 段，请检查 attlib.dat 或调整锚点")

    # ATGTDF-2 起点通常在段指针[4]，可根据需要调整，此处采用同一数据区直接起点扫描
    # 选择最早出现的哈希页作为起点
    candidate_pages = [
        i for i, pg in enumerate(attlib_pages) if any(MIN_HASH <= w <= MAX_HASH for w in pg)
    ]
    if not candidate_pages:
        raise SystemExit("未找到任何哈希页，无法解析 ATGTDF-2")
    atgtdf2_start = min(candidate_pages)
    field_idx2 = parse_atgtdf2(attlib_pages, atgtdf2_start)
    if field_hash not in field_idx2:
        print("警告: 指定字段哈希不在 ATGTDF-2 中，结果可能为空")

    dab_pages = read_pages(args.dab, data_offset)

    with args.out.open("w", newline="") as out:
        writer = csv.writer(out)
        writer.writerow(["noun_hash", "noun_name", "attr_count", "attr_hashes", "attr_names"])
        for noun_hash, (page_num, off) in noun_index.items():
            attrs = read_prdisp(dab_pages, noun_index, field_idx2, noun_hash, field_hash)
            if attrs is None:
                continue
            names = [decode_base27(h) for h in attrs]
            writer.writerow(
                [
                    f"0x{noun_hash:08X}",
                    decode_base27(noun_hash),
                    len(attrs),
                    ";".join(f"0x{x:08X}" for x in attrs),
                    ";".join(names),
                ]
            )
    print(f"完成，ATGTIX-2 起始页 {start_page}，输出 {args.out}")


if __name__ == "__main__":
    main()
