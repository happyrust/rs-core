#!/usr/bin/env python3
"""
从 attlib.dat 解析 ATGTIX-2（noun → page/slot 映射）并导出 CSV。
仅依赖标准库，可直接运行：
    python parse_atgtix2.py /path/to/attlib.dat [output.csv]
"""

import csv
import struct
import sys
from pathlib import Path
from typing import Dict, Iterable, List, Tuple

PAGE_SIZE = 2048
WORDS_PER_PAGE = 512
DATA_REGION_START = 0x1000  # 数据区域起始偏移（与 IDA 反汇编一致）
PAGE_SWITCH = 0x00000000    # 页切换标记
SEGMENT_END = 0xFFFFFFFF    # 段结束标记
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


def load_pages(fh) -> List[List[int]]:
    """一次性把数据区域的所有页面读入内存，便于扫描。"""
    fh.seek(0, 2)
    size = fh.tell()
    data_size = size - DATA_REGION_START
    total_pages = data_size // PAGE_SIZE
    pages: List[List[int]] = []
    fh.seek(DATA_REGION_START)
    for _ in range(total_pages):
        buf = fh.read(PAGE_SIZE)
        if len(buf) < PAGE_SIZE:
            break
        pages.append(list(struct.unpack(">512I", buf)))
    return pages


def parse_index_from_page(
    pages: List[List[int]], start_page: int
) -> Tuple[List[Tuple[int, int, int, int]], bool]:
    """按照 IDA 的 ATTLIB_Load_Index_ATGTIX 逻辑，从给定页号开始解析，直到遇到 SEGMENT_END。

    返回: (记录列表, 是否遇到 SEGMENT_END)
    """
    page = start_page
    idx = 0
    records: List[Tuple[int, int, int, int]] = []
    ended_with_mark = False
    while True:
        if page >= len(pages):
            return records, ended_with_mark
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
            ended_with_mark = True
            break
        if word < MIN_HASH or word > MAX_HASH:
            continue

        combined = pages[page][idx]
        idx += 1

        page_num = combined // WORDS_PER_PAGE
        offset = combined % WORDS_PER_PAGE
        records.append((word, page_num, offset, combined))
    return records, ended_with_mark


ANCHOR_HASHES = {
    0x000CA439,  # PIPE
    0x0009D65A,  # SITE
}


def find_best_start_page(pages: List[List[int]]) -> Tuple[int, int]:
    """遍历页面，寻找含有典型 noun（PIPE/SITE 等）的起始页。"""
    best_page = -1
    best_count = -1
    for page in range(len(pages)):
        # 快速预筛：页内至少有一个落在哈希范围的词才尝试
        if not any(MIN_HASH <= w <= MAX_HASH for w in pages[page]):
            continue
        records, ended = parse_index_from_page(pages, page)
        if not ended:
            continue
        if not any(hash_val in ANCHOR_HASHES for hash_val, *_ in records):
            continue
        if len(records) > best_count:
            best_count = len(records)
            best_page = page
    return best_page, best_count


def main() -> int:
    if len(sys.argv) not in (2, 3):
        print(f"用法: {Path(sys.argv[0]).name} /path/to/attlib.dat [output.csv]")
        return 1

    attlib_path = Path(sys.argv[1])
    if not attlib_path.exists():
        print(f"文件不存在: {attlib_path}")
        return 1
    output_path = Path(sys.argv[2]) if len(sys.argv) == 3 else Path("atgtix2.csv")

    with attlib_path.open("rb") as fh:
        pages = load_pages(fh)

    start_page, count_est = find_best_start_page(pages)
    if start_page < 0:
        print("未找到符合 ATGTIX-2 格式的段")
        return 1

    with output_path.open("w", newline="") as out:
        writer = csv.writer(out)
        writer.writerow(
            ["noun_hash", "noun_name", "page", "offset", "combined", "start_page"]
        )

        records, ended = parse_index_from_page(pages, start_page)
        for noun_hash, page, offset, combined in records:
            writer.writerow(
                [
                    f"0x{noun_hash:08X}",
                    decode_base27(noun_hash),
                    page,
                    offset,
                    combined,
                    start_page,
                ]
            )
        count = len(records)

    print(
        f"写出 {output_path} 完成，起始页: {start_page}, 记录数: {count}（预估 {count_est}）"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
