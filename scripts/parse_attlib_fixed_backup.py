#!/usr/bin/env python3
        print(f"\n加载 {segment_name} 段（属性定义）")
        print(f"  使用段指针页号: {start_page}")
        print(f"  使用段指针页号: {start_page}")

        page_num = start_page
        word_idx = 0
        record_count = 0

        page_guard = 0
        segment_end_count = 0  # 记录连续的SEGMENT_END_MARK数量

        while True:
            if page_guard > 200000:
                print(f"  终止 {segment_name}: 触发页扫描上限")
                return
            page_guard += 1
            words = self.read_page(page_num)

            while word_idx < WORDS_PER_PAGE:
                word = words[word_idx]
                word_idx += 1

                if word == PAGE_SWITCH_MARK:
                    # 页切换标记：推进到下一页
                    page_num += 1
                    word_idx = 0
                    segment_end_count = 0  # 重置计数
                    continue

                if word == SEGMENT_END_MARK:
                    segment_end_count += 1
                    # 如果遇到3个或更多连续的SEGMENT_END_MARK，认为段结束
                    if segment_end_count >= 3:
                        print(f"  {segment_name} 加载完成，共 {record_count} 条记录")
                        return
                    continue

                # 遇到非结束标记，重置计数
                segment_end_count = 0

                # 检查是否为有效哈希值
                if word < MIN_HASH or word > MAX_HASH:
                    continue

                attr_hash = word

                # 读取 data_type
                if word_idx >= WORDS_PER_PAGE:
                    page_num += 1
                    word_idx = 0
                    words = self.read_page(page_num)

                data_type = words[word_idx]
                word_idx += 1

                # 读取 default_flag
                if word_idx >= WORDS_PER_PAGE:
                    page_num += 1
                    word_idx = 0
                    words = self.read_page(page_num)

                default_flag = words[word_idx]
                word_idx += 1

                # 读取默认值
                default_value = self._read_default_value(words, word_idx, page_num, data_type, default_flag)

                self.attr_definitions[attr_hash] = AttlibAttrDefinition(
                    attr_hash=attr_hash,
                    data_type=data_type,
                    default_flag=default_flag,
                    default_value=default_value
                )

                if record_count < 5:
                    print(f"    [{record_count}] hash=0x{attr_hash:08X}, type={data_type}, flag={default_flag}")

                record_count += 1
def load_atgtix(self, start_page: int, segment_name: str = "ATGTIX"):
        """加载 ATGTIX 段（属性索引表）"""
        print(f"\n加载 {segment_name} 段（属性索引）")
        print(f"  使用段指针页号: {start_page}")
        
        page_num = start_page
        word_idx = 0
        record_count = 0
        page_guard = 0

        while True:
            if page_guard > 200000:
                print(f"  终止 {segment_name}: 触发页扫描上限")
                return
            page_guard += 1
            words = self.read_page(page_num)

            page_has_data = False
            while word_idx < WORDS_PER_PAGE:
                word = words[word_idx]
                word_idx += 1

                if word == PAGE_SWITCH_MARK:
                    page_num += 1
                    word_idx = 0
                    break  # 切换到下一个页面

                if word == SEGMENT_END_MARK:
                    print(f"  {segment_name} 加载完成，共 {record_count} 条记录")
                    return

                if word < MIN_HASH or word > MAX_HASH:
                    continue

                # 找到有效哈希，标记页面包含数据
                page_has_data = True
                attr_hash = word

                if word_idx >= WORDS_PER_PAGE:
                    page_num += 1
                    word_idx = 0
                    words = self.read_page(page_num)

                combined = words[word_idx]
                word_idx += 1

                self.attr_index[attr_hash] = AttlibAttrIndex(
                    attr_hash=attr_hash,
                    combined=combined
                )

                if record_count < 5:
                    print(f"    [{record_count}] hash=0x{attr_hash:08X}, combined=0x{combined:08X}")

                record_count += 1

            # 如果处理完整个页面都没有找到有效数据，说明段结束
            if not page_has_data and word_idx >= WORDS_PER_PAGE:
                print(f"  {segment_name} 加载完成，共 {record_count} 条记录")
                return
    
    def load_all(self):
        """加载所有段 - 基于IDA Pro的正确顺序"""
        # 根据IDA Pro分析，正确的段指针映射：
        # v47[0] = 段指针[0] = 3 → ATGTDF-1
        # v47[2] = 段指针[2] = 1683 → ATGTIX-1
        # v47[4] = 段指针[4] = 1741 → ATGTDF-2  
        # v47[6] = 段指针[6] = 2236 → ATGTIX-2
        
        # 第一次加载 ATGTIX (段指针[2])
        self.load_atgtix(self.segment_pointers[2], "ATGTIX-1")
        
        # 第一次加载 ATGTDF (段指针[0])
        self.load_atgtdf(self.segment_pointers[0], "ATGTDF-1")
        
        # 第二次加载 ATGTIX (段指针[6])
        self.load_atgtix(self.segment_pointers[6], "ATGTIX-2")
        
        # 第二次加载 ATGTDF (段指针[4])
        self.load_atgtdf(self.segment_pointers[4], "ATGTDF-2")
    
    def get_attribute(self, hash_val: int) -> Optional[AttlibAttrDefinition]:
        """获取属性定义"""
        return self.attr_definitions.get(hash_val)
    
    def close(self):
        """关闭文件"""
        self.file.close()

    def list_object_attributes(self, object_name: str, mapping_path: str = "data/ELBO.json") -> List[Tuple[str, int]]:
        """从映射文件中读取指定对象的所有属性名与哈希
        返回列表 [(attr_name, hash), ...]
        """
        with open(mapping_path, "r", encoding="utf-8") as f:
            data = json.load(f)
        if object_name not in data:
            return []
        obj = data[object_name]
        result: List[Tuple[str, int]] = []
        for attr_name, meta in obj.items():
            # 期望结构: { name, hash, offset, default_val, att_type }
            if isinstance(meta, dict) and "hash" in meta:
                result.append((attr_name, int(meta["hash"])))
        # 稳定排序，便于复现
        result.sort(key=lambda x: x[0])
        return result

def data_type_to_string(data_type: int) -> str:
    """将数据类型代码转换为字符串 - 基于DB_Attribute完整映射"""
    types = {
        # 基本类型 (attlib.dat支持存储默认值)
        1: "LOG",        # 布尔类型 (BOOL)
        2: "REAL",       # 双精度浮点 (DOUBLE)
        3: "INT",        # 32位整数 (INTEGER)
        4: "TEXT",       # 27进制编码字符串 (STRING)

    # 扩展类型 (仅运行时支持)
    5: "REF",        # 元素引用 (ELEMENT)
    6: "NAME",       # Noun名称 (WORD)
    7: "ATTRIBUTE",  # 属性引用
    8: "POINT",      # 3D点 (POSITION)
    9: "VECTOR",     # 3D向量 (DIRECTION)
    10: "MATRIX",    # 3D矩阵 (ORIENTATION)
    11: "TRANSFORM", # 3D变换
    12: "DATETIME",  # 日期时间 (DATETIME)
    }
    return types.get(data_type, f"UNKNOWN({data_type})")

if __name__ == "__main__":
    cli = argparse.ArgumentParser(description="attlib.dat 解析器 (修复版)")
    cli.add_argument("object", nargs="?", default=None, help="对象名，例如 ELBO")
    cli.add_argument("--map", dest="mapping", default="data/ELBO.json", help="对象到属性映射文件")
    args = cli.parse_args()

    parser = AttlibParserFixed("data/attlib.dat")
    parser.load_all()

    if args.object:
        obj_name = args.object
        print(f"\n=== 对象 {obj_name} 的属性查询 ===")
        pairs = parser.list_object_attributes(obj_name, args.mapping)
        if not pairs:
            print(f"未在映射文件中找到对象: {obj_name}")
        else:
            print(f"段指针: {parser.segment_pointers}")
            print(f"ATGTIX 索引条数: {len(parser.attr_index)}  ATGTDF 定义条数: {len(parser.attr_definitions)}")
            found_cnt = 0
            for attr_name, hash_val in pairs:
                idx = parser.attr_index.get(hash_val)
                defn = parser.attr_definitions.get(hash_val)
                state = []
                if idx:
                    state.append("Index")
                if defn:
                    state.append("Def")
                mark = "✓" if state else "✗"
                state_s = ",".join(state) if state else "-"
                print(f"{mark} {attr_name:>8s}  hash=0x{hash_val:08X} ({hash_val})  [{state_s}]")
                if defn:
                    print(f"    type={data_type_to_string(defn.data_type)} flag={defn.default_flag} value={defn.default_value}")
                found_cnt += 1 if state else 0
            print(f"\n共 {len(pairs)} 项，存在 {found_cnt} 项（Index或Def 任一存在即计数）")
        parser.close()
    else:
        print("\n=== 所有找到的属性 ===")
        print(f"ATGTIX 索引: {len(parser.attr_index)} 条")
        print(f"ATGTDF 定义: {len(parser.attr_definitions)} 条")

        if parser.attr_definitions:
            print("\nATGTDF 中的属性:")
            for i, (hash_val, attr) in enumerate(list(parser.attr_definitions.items())[:10]):
                print(f"  [{i}] hash=0x{hash_val:08X}, type={data_type_to_string(attr.data_type)}, flag={attr.default_flag}")

        if parser.attr_index:
            print("\nATGTIX 中的所有属性:")
            for i, (hash_val, idx) in enumerate(parser.attr_index.items()):
                print(f"  [{i}] hash=0x{hash_val:08X} ({hash_val}), combined=0x{idx.combined:08X}")

        parser.close()
