#!/usr/bin/env python3
"""显示 ELBO 表的完整结构"""

import kuzu
import sys

# 连接数据库
db = kuzu.Database('./data/kuzu_db')
conn = kuzu.Connection(db)

# 查询表结构
result = conn.execute("CALL table_info('Attr_ELBO') RETURN *;")

print("\n=== Attr_ELBO 表结构 ===\n")
print(f"{'ID':<3} {'字段名':<15} {'类型':<15} {'主键':<5}")
print("=" * 50)

fields = []
while result.has_next():
    row = result.get_next()
    prop_id = row[0]
    name = row[1]
    type_name = row[2]
    default = row[3]
    is_primary = row[4]

    fields.append({
        'id': prop_id,
        'name': name,
        'type': type_name,
        'primary': is_primary
    })

# 排序并显示
fields.sort(key=lambda x: x['id'])
for field in fields:
    print(f"{field['id']:<3} {field['name']:<15} {field['type']:<15} {'✓' if field['primary'] else ''}")

print(f"\n总计: {len(fields)} 个字段")

# 统计类型
type_counts = {}
for field in fields:
    t = field['type']
    type_counts[t] = type_counts.get(t, 0) + 1

print("\n字段类型统计:")
for t, count in sorted(type_counts.items()):
    print(f"  {t:<15}: {count} 个")

# 查找数组类型
array_fields = [f for f in fields if '[' in f['type']]
if array_fields:
    print("\n数组类型字段:")
    for f in array_fields:
        print(f"  - {f['name']}: {f['type']}")

conn.close()