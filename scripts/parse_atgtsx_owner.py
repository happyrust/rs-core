#!/usr/bin/env python3
"""
从 attlib.dat 中解析 ATGTSX (属性语法表)，并聚焦 OWNER 属性的语法块。

功能概述：
1. 读取段指针表并定位 ATGTSX 段；
2. 按 FHDBRN 规则遍历记录，获得 (pack_code, index_ptr, third_value) 三元组；
3. 过滤 pack_code 解码后为 "OWNER" 的记录；
4. 根据 index_ptr 从语法字符串段中提取 token 流，解码所有大于 0x81BF1 的值为 DB1 字符串；
5. 将解析结果输出为 JSON 及人类可读的控制台摘要，便于进一步分析 ALLOWED 集合。

目前我们尚未完全掌握控制指令（value <= 0x81BF1）的语义，因此脚本只会原样保留这些数值。
"""

from __future__ import annotations

import argparse
import json
import struct
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Iterator, List, Optional, Tuple

PAGE_SIZE = 2048
WORDS_PER_PAGE = 512
PAGE_SWITCH_MARK = 0x00000000
SEGMENT_END_MARK = 0xFFFFFFFF
DATA_REGION_START = 0x1000
SEGMENT_POINTERS_OFFSET = 0x0800
DB1_BASE = 0x81BF1


def read_segment_pointers(f) -> List[int]:
    f.seek(SEGMENT_POINTERS_OFFSET)
    data = f.read(32)
    return [struct.unpack(">I", data[i : i + 4])[0] for i in range(0, 32, 4)]


def read_page(f, page_num: int) -> List[int]:
    f.seek(DATA_REGION_START + page_num * PAGE_SIZE)
    raw = f.read(PAGE_SIZE)
    return [struct.unpack(">I", raw[i : i + 4])[0] for i in range(0, PAGE_SIZE, 4)]


def decode_pack_code(pack_code: int) -> str:
    """27 进制压缩，读取顺序与 pack_code 保持一致。"""
    if pack_code == 0:
        return ""
    chars: List[str] = []
    value = pack_code
    while value:
        value, rem = divmod(value, 27)
        if rem == 0:
            chars.append(" ")
        else:
            chars.append(chr(rem + 64))
    return "".join(reversed(chars)).strip()


def db1_dehash(value: int) -> Optional[str]:
    """根据 core.dll 的 db1_hash 实现（逆向顺序）解码 token。"""
    if value <= DB1_BASE:
        return None
    k = value - DB1_BASE
    chars: List[str] = []
    while k:
        k, rem = divmod(k, 27)
        chars.append(chr(rem + 64))
    return "".join(chars)


@dataclass
class AtgtsxRecord:
    pack_code: int
    index_ptr: int
    third_value: int

    @property
    def attr_name(self) -> str:
        return decode_pack_code(self.pack_code)


def iter_atgtsx_records(f, start_page: int) -> Iterator[AtgtsxRecord]:
    page_num = start_page
    word_idx = 0
    while True:
        words = read_page(f, page_num)
        while word_idx < WORDS_PER_PAGE:
            word = words[word_idx]
            word_idx += 1

            if word == PAGE_SWITCH_MARK:
                page_num += 1
                word_idx = 0
                break

            if word == SEGMENT_END_MARK:
                return

            pack_code = word
            if word_idx >= WORDS_PER_PAGE:
                page_num += 1
                words = read_page(f, page_num)
                word_idx = 0
            index_ptr = words[word_idx]
            word_idx += 1

            if word_idx >= WORDS_PER_PAGE:
                page_num += 1
                words = read_page(f, page_num)
                word_idx = 0
            third_value = words[word_idx]
            word_idx += 1

            yield AtgtsxRecord(pack_code, index_ptr, third_value)


def read_token_stream(f, string_segment_start: int, offset: int) -> List[int]:
    """index_ptr 指向的 token 流，结束于 0 或 SEGMENT_END_MARK。"""
    tokens: List[int] = []
    current_page = string_segment_start + offset // WORDS_PER_PAGE
    idx = offset % WORDS_PER_PAGE

    while True:
        words = read_page(f, current_page)
        while idx < WORDS_PER_PAGE:
            value = words[idx]
            idx += 1
            if value == PAGE_SWITCH_MARK:
                current_page += 1
                idx = 0
                break
            if value == 0 or value == SEGMENT_END_MARK:
                return tokens
            tokens.append(value)
        else:
            current_page += 1
            idx = 0


def load_owner_records(file_path: Path) -> Tuple[List[int], List[AtgtsxRecord]]:
    with file_path.open("rb") as f:
        pointers = read_segment_pointers(f)
        atgtsx_start = pointers[3]
        string_start = pointers[4]
        records = [
            rec
            for rec in iter_atgtsx_records(f, atgtsx_start)
            if rec.attr_name == "OWNER"
        ]
    return string_start, records


def summarize_owner(
    file_path: Path, owner_record: AtgtsxRecord, string_page_start: int
) -> dict:
    with file_path.open("rb") as f:
        token_values = read_token_stream(f, string_page_start, owner_record.index_ptr)

    decoded_tokens = [
        {"value": value, "text": db1_dehash(value)} for value in token_values
    ]

    return {
        "record": asdict(owner_record),
        "tokens": decoded_tokens,
    }


def main() -> None:
    parser = argparse.ArgumentParser(
        description="解析 attlib.dat 中 OWNER 属性的 ATGTSX 语法块"
    )
    parser.add_argument(
        "--attlib",
        default="data/attlib.dat",
        type=Path,
        help="attlib.dat 路径 (默认: data/attlib.dat)",
    )
    parser.add_argument(
        "--output",
        default="owner_tokens.json",
        type=Path,
        help="解析结果输出 JSON (默认: owner_tokens.json)",
    )
    args = parser.parse_args()

    string_start, owners = load_owner_records(args.attlib)
    if not owners:
        raise SystemExit("未在 ATGTSX 中找到 pack_code = OWNER 的记录")

    if len(owners) > 1:
        print(f"警告：检测到 {len(owners)} 条 OWNER 记录，默认解析第一条。")

    owner_info = summarize_owner(args.attlib, owners[0], string_start)
    args.output.write_text(json.dumps(owner_info, ensure_ascii=False, indent=2))

    print(f"发现 OWNER 记录数量: {len(owners)}")
    print(f"index_ptr = {owners[0].index_ptr}, third_value = {owners[0].third_value}")
    print(f"token 数量: {len(owner_info['tokens'])}")
    print("样例 token（value -> text）:")
    for item in owner_info["tokens"][:20]:
        print(f"  {item['value']:>10} -> {item['text'] or '<CONTROL>'}")
    print(f"结果已写入 {args.output}")


if __name__ == "__main__":
    main()
