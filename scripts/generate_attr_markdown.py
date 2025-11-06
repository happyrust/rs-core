#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
生成属性列表 Markdown 文档
整理 all_attr_info.json 中 named_attr_info_map 的所有属性
"""

import json
from collections import defaultdict

# 读取 JSON 文件
with open('all_attr_info.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

# 收集所有属性及它们所在的 NOUN
attr_to_nouns = defaultdict(set)
all_attrs = set()

for noun_name, noun_attrs in data['named_attr_info_map'].items():
    for attr_name in noun_attrs.keys():
        all_attrs.add(attr_name)
        attr_to_nouns[attr_name].add(noun_name)

# 按字母顺序排序
sorted_attrs = sorted(all_attrs)

# 生成 Markdown 文档
md_content = """# PDMS/E3D 属性列表

本文档整理了 `all_attr_info.json` 中 `named_attr_info_map` 的所有属性。

## 统计信息

- **总属性数**: {total_attrs}
- **总 NOUN 类型数**: {total_nouns}

## 属性列表

""".format(
    total_attrs=len(all_attrs),
    total_nouns=len(data['named_attr_info_map'])
)

# 添加属性表格
md_content += "| 序号 | 属性名称 | 使用该属性的 NOUN 数量 | 说明 |\n"
md_content += "|------|----------|----------------------|------|\n"

for idx, attr_name in enumerate(sorted_attrs, 1):
    noun_count = len(attr_to_nouns[attr_name])
    # 如果有太多 NOUN，只显示前几个
    if noun_count <= 10:
        nouns_list = ", ".join(sorted(attr_to_nouns[attr_name]))
    else:
        nouns_list = ", ".join(sorted(list(attr_to_nouns[attr_name]))[:10]) + f" ... (共 {noun_count} 个)"
    
    md_content += f"| {idx} | `{attr_name}` | {noun_count} | - |\n"

# 添加按 NOUN 分组的属性列表
md_content += "\n## 按 NOUN 类型分组的属性\n\n"

sorted_nouns = sorted(data['named_attr_info_map'].keys())
for noun_name in sorted_nouns:
    noun_attrs = data['named_attr_info_map'][noun_name]
    sorted_noun_attrs = sorted(noun_attrs.keys())
    
    md_content += f"### {noun_name} ({len(sorted_noun_attrs)} 个属性)\n\n"
    md_content += "| 属性名称 | 属性类型 | 默认值类型 |\n"
    md_content += "|----------|----------|------------|\n"
    
    for attr_name in sorted_noun_attrs:
        attr_info = noun_attrs[attr_name]
        att_type = attr_info.get('att_type', '')
        default_val_type = list(attr_info.get('default_val', {}).keys())
        default_val_type_str = default_val_type[0] if default_val_type else ''
        
        md_content += f"| `{attr_name}` | {att_type} | {default_val_type_str} |\n"
    
    md_content += "\n"

# 保存 Markdown 文件
output_file = '属性列表.md'
with open(output_file, 'w', encoding='utf-8') as f:
    f.write(md_content)

print(f'Markdown 文档已生成: {output_file}')
print(f'共包含 {len(all_attrs)} 个唯一属性')
print(f'共包含 {len(data["named_attr_info_map"])} 个 NOUN 类型')

# 显示统计信息
print('\n使用频率最高的前10个属性:')
attr_usage = [(attr, len(nouns)) for attr, nouns in attr_to_nouns.items()]
attr_usage.sort(key=lambda x: x[1], reverse=True)
for attr, count in attr_usage[:10]:
    print(f'  {attr}: 被 {count} 个 NOUN 使用')










