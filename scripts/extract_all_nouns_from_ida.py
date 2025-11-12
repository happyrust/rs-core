#!/usr/bin/env python3
"""
从 IDA Pro 提取所有 Noun 类型定义

基于 core.dll 中的全局字符串
作者: AI Analysis Tool
日期: 2025
"""

import json
import re
from pathlib import Path

# 从 IDA Pro list_globals_filter 提取的所有 aNoun 字符串
# 这些是手动复制的 IDA Pro 输出
NOUN_STRINGS = """
aNounAbox3qbvdb
aNounAccpnt3qbv
aNounAccset3qbv
aNounAcdt3qbvdb
aNounAcone3qbvd
aNounAcr3qbvdbN
aNounAcrl3qbvdb
aNounAcrst3qbvd
aNounAcrule3qbv
aNounAcrw3qbvdb
aNounAcstyl3qbv
aNounActi3qbvdb
aNounActn3qbvdb
aNounActo3qbvdb
aNounActor3qbvd
aNounAcyli3qbvd
aNounAdde3qbvdb
aNounAdim3qbvdb
aNounAdir3qbvdb
aNounAdish3qbvd
aNounAextr3qbvd
aNounAhu3qbvdbN
aNounAidarc3qbv
aNounAidcir3qbv
aNounAidgro3qbv
aNounAidlin3qbv
aNounAidpoi3qbv
aNounAidtex3qbv
aNounBend3qbvdb
aNounBran3qbvdb
aNounCable3qbvd
aNounCap3qbvdbN
aNounCirc3qbvdb
aNounCone3qbvdb
aNounCoup3qbvdb
aNounCyli3qbvdb
aNounDamp3qbvdb
"""

def extract_noun_names(noun_strings: str) -> dict:
    """从 IDA Pro 字符串中提取 Noun 名称"""
    
    nouns = {}
    lines = [line.strip() for line in noun_strings.strip().split('\n') if line.strip()]
    
    for line in lines:
        # 提取 Noun 名称：aNoun{NAME}3qbv...
        # 模式：aNoun + 大写字母开头的名称 + 3qbv
        match = re.match(r'aNoun([A-Z][a-z]{2,}).*', line)
        if match:
            noun_name = match.group(1).upper()
            nouns[noun_name] = {
                'string_name': line,
                'identified': True
            }
        else:
            # 如果是特殊格式（如 aNounCap3qbvdbN）
            match2 = re.match(r'aNoun([A-Z][a-z]{1,3}).*', line)
            if match2:
                noun_name = match2.group(1).upper()
                nouns[noun_name] = {
                    'string_name': line,
                    'identified': True
                }
    
    return nouns

def generate_complete_noun_list(output_path: str):
    """生成完整的 Noun 列表"""
    
    print("="*70)
    print(" "*15 + "从 IDA Pro 提取所有 Noun 类型")
    print("="*70)
    
    # 提取 Noun 名称
    nouns = extract_noun_names(NOUN_STRINGS)
    
    print(f"\n✅ 提取到 {len(nouns)} 个 Noun 类型:")
    for name in sorted(nouns.keys()):
        print(f"   - {name}")
    
    # 基于 PDMS 标准添加层级关系
    hierarchy = {
        "WORL": ["SITE"],
        "SITE": ["ZONE"],
        "ZONE": ["EQUI", "STRU"],
        "EQUI": ["PIPE", "BRAN"],
        "PIPE": ["BEND", "CAP", "CONE", "COUP", "CYLI", "DAMP"],
        "BRAN": ["BEND", "CAP", "CONE", "COUP", "CYLI"],
    }
    
    # 生成输出
    output_data = {
        "version": "4.0",
        "source": "IDA Pro core.dll 全局字符串提取",
        "description": "从 core.dll 提取的所有 Noun 类型定义（不依赖 JSON）",
        "extraction_method": "分析 IDA Pro 中的 aNoun* 全局字符串变量",
        "nouns": nouns,
        "hierarchy": hierarchy,
        "statistics": {
            "total_nouns": len(nouns),
            "identified_nouns": len([n for n in nouns.values() if n.get('identified')]),
            "hierarchy_relations": sum(len(v) for v in hierarchy.values())
        }
    }
    
    # 保存文件
    output_file = Path(output_path)
    with open(output_file, 'w', encoding='utf-8') as f:
        json.dump(output_data, f, indent=2, ensure_ascii=False)
    
    print(f"\n✅ 数据已保存到: {output_file}")
    print(f"\n📊 统计:")
    for key, value in output_data['statistics'].items():
        print(f"   - {key}: {value}")
    
    return output_data


def main():
    """主函数"""
    output_path = "/Volumes/DPC/work/plant-code/rs-core/all_nouns_from_ida.json"
    
    result = generate_complete_noun_list(output_path)
    
    print("\n" + "="*70)
    print(" "*25 + "✨ 提取完成！")
    print("="*70)


if __name__ == "__main__":
    main()
