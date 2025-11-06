#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
生成属性元数据 SURQL 文件
从属性列表.md 中提取属性信息，生成用于 SurrealDB 的初始化脚本
"""

import re
import os

def db1_hash(hash_str):
    """实现 Rust 中的 db1_hash 函数逻辑"""
    chars = hash_str.encode('utf-8')
    if len(chars) < 1:
        return 0
    
    val = 0
    i = len(chars) - 1
    while i >= 0:
        val = (val * 27) + (chars[i] - 64)
        i -= 1
    
    # 处理溢出，模拟 Rust 的 saturating_add
    result = val + 0x81BF1
    if result > 0xFFFFFFFF:
        result = 0xFFFFFFFF
    return result & 0xFFFFFFFF

def parse_attr_list_md(file_path):
    """解析属性列表.md文件，提取属性信息"""
    attributes = []
    
    with open(file_path, 'r', encoding='utf-8') as f:
        content = f.read()
    
    # 匹配表格行: | 序号 | `属性名` | 数量 | 说明 |
    pattern = r'\|\s*\d+\s*\|\s*`([A-Z0-9]+)`\s*\|\s*\d+\s*\|\s*([^|]+)\s*\|'
    
    matches = re.findall(pattern, content)
    
    for attr_name, desc in matches:
        desc = desc.strip()
        # 清理描述中的反引号等
        desc = desc.replace('`', '').strip()
        if desc == '-':
            desc = ''
        
        hash_value = db1_hash(attr_name)
        attributes.append({
            'name': attr_name,
            'hash': hash_value,
            'desc': desc
        })
    
    return attributes

def generate_surql(attributes, output_path):
    """生成 SURQL 文件"""
    
    # 创建表定义
    surql_content = """-- PDMS/E3D 属性元数据表
-- 此文件包含所有属性的元数据信息，使用属性的 HASH 值作为 ID

DEFINE TABLE attr_metadata SCHEMAFULL
    PERMISSIONS
        FULL;

DEFINE FIELD name ON attr_metadata TYPE string
    ASSERT $value != NONE;

DEFINE FIELD desc ON attr_metadata TYPE option<string>;

DEFINE FIELD hash ON attr_metadata TYPE number;

DEFINE INDEX attr_metadata_hash_idx ON attr_metadata FIELDS hash UNIQUE;
DEFINE INDEX attr_metadata_name_idx ON attr_metadata FIELDS name UNIQUE;

-- 插入属性元数据
"""
    
    # 生成 INSERT 语句
    for attr in attributes:
        hash_id = attr['hash']
        name = attr['name']
        desc = attr['desc']
        
        # 转义单引号
        desc_escaped = desc.replace("'", "\\'") if desc else None
        
        if desc_escaped:
            surql_content += f"CREATE attr_metadata:{hash_id} SET name = '{name}', desc = '{desc_escaped}', hash = {hash_id};\n"
        else:
            surql_content += f"CREATE attr_metadata:{hash_id} SET name = '{name}', hash = {hash_id};\n"
    
    # 写入文件
    with open(output_path, 'w', encoding='utf-8') as f:
        f.write(surql_content)
    
    print(f'SURQL 文件已生成: {output_path}')
    print(f'共包含 {len(attributes)} 个属性')

if __name__ == '__main__':
    # 获取脚本所在目录
    script_dir = os.path.dirname(os.path.abspath(__file__))
    project_root = os.path.dirname(script_dir)
    
    # 输入文件路径
    attr_list_path = os.path.join(project_root, '属性列表.md')
    
    # 输出文件路径
    output_path = os.path.join(project_root, 'resource', 'surreal', 'attr_metadata.surql')
    
    # 解析属性列表
    print(f'正在解析属性列表文件: {attr_list_path}')
    attributes = parse_attr_list_md(attr_list_path)
    
    # 生成 SURQL 文件
    print(f'正在生成 SURQL 文件: {output_path}')
    generate_surql(attributes, output_path)
    
    # 显示前10个作为示例
    print('\n前10个属性示例:')
    for i, attr in enumerate(attributes[:10], 1):
        print(f'{i}. HASH: {attr["hash"]}, 名称: {attr["name"]}, 描述: {attr["desc"]}')

