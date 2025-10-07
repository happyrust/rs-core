# Kuzu 属性&关系重构阶段计划

## 阶段划分概览

1. **阶段一：模式蓝图与映射资产搭建**
   - 明确定义 `PE` 节点、强类型属性表、属性节点三层结构的角色与字段。
   - 根据 `all_attr_info.json` 生成 `noun -> 属性表` 映射草稿，梳理需要的 `TO_<NOUN>` 外部引用边。
   - 设计写入/查询流程图，明确缓存(`named_attr_json`) 与一对一 `REL_ATTR` 边的职责。

2. **阶段二：Schema & 代码骨架实现**
   - 在 `src/rs_kuzu/schema.rs` 中补充强类型属性表建表逻辑、统一外部引用边的创建入口。
   - 新增/更新 `rs_kuzu::operations`、`rs_kuzu::queries`，让 `save_*`/`get_*` 具备真实读写能力。

3. **阶段三：同步管线改造**
   - 更新 `sync` 管理器与 `DatabaseAdapter`，写入时同步维护 `named_attr_json`、强类型表和外部引用边。
   - 增强错误处理与回滚策略，确保多通路写入原子性。

4. **阶段四：查询封装与工具链**
   - 提供公共查询函数（如 `get_named_attmap`、`get_spre_ptre`），封装多跳 `MATCH`。
   - 构建脚本/宏生成器，自动产出属性表 SQL、Rust 常量、edge 维护逻辑。

5. **阶段五：验证与迁移**
   - 编写单元/集成测试，覆盖属性读写、引用跳转、缓存一致性。
   - 制作迁移脚本、性能基准，并拟定上线回滚方案。

## 阶段一详细任务

- [ ] **结构说明文档**：补充本文件，列出三层结构与字段职责、缓存策略。
- [ ] **属性表映射草稿**：读取 `all_attr_info.json`，罗列至少示例 `noun` 的属性列与类型。
- [ ] **外部引用清单**：梳理 Surreal 中常见引用字段，确认需要生成的 `TO_<NOUN>` 边集合。
- [ ] **流程图/伪代码**：描述 `save_attmap_kuzu`、`get_named_attmap_kuzu` 的多通路写读顺序。

阶段一完成标志：上述四项素材齐备，可直接指导 Schema 与代码实现。

## 结构三层模型

| 层级 | 载体 | 关键字段 | 职责 |
| ---- | ---- | -------- | ---- |
| 实体 | `PE` 节点 | `refno` 主键、`noun`、`attr_ref`、`named_attr_json` | 表示工厂元素本体，保存缓存快照与属性表引用 |
| 强类型属性 | `Attr_<NOUN>` 节点表 | `refno` 主键 + 结构化列 | 对应 Surreal 中稳定字段，便于索引与高速读取 |
| 动态属性 | 通用 `Attribute` 节点 | `name`、`attr_value`、`attr_type` | 兼容 UDA/临时字段，保留 Surreal 风格图遍历 |

- `PE.named_attr_json`：JSON 缓存（可选天然 `NamedAttrMap` 序列化），读流程优先命中；失效时回落到强类型表+属性节点聚合。
- `attr_ref`：指向强类型属性表行的 `refno`（自引用可用，也可拓展到跨库 ID）。
- `REL_ATTR`：`(:PE)-[:REL_ATTR]->(:Attr_<NOUN>)`，保证一跳命中结构化属性；节点删除时同步清理。

## 示例属性表映射（节选）

```text
noun = "ELBO"
Attr_ELBO(refno INT64 PRIMARY KEY,
          STATUS_CODE STRING,
          LOCK BOOLEAN,
          SPRE_REFNO INT64,
          PTRE_REFNO INT64,
          CATA_HASH STRING,
          LEVE LIST<INT32>,
          ...)

to_edges:
  - TO_SPRE (PE 'ELBO' -> PE 'SPRE')
  - TO_PTRE (PE 'ELBO' -> PTRE)
  - TO_CATA (PE 'ELBO' -> PE 'CATA')
```

将来脚本可从 `all_attr_info.json` 自动导出此结构，并生成 Rust 常量：

```rust
pub struct AttrTableSpec {
    pub noun: &'static str,
    pub table: &'static str,
    pub fields: &'static [AttrFieldSpec],
    pub edges: &'static [EdgeSpec],
}
```

### Attr 表规格（示例）

| noun | 表名 | 关键字段 | 说明 | 引用边 |
| ---- | ---- | -------- | ---- | ------ |
| ELBO | `Attr_ELBO` | `status_code STRING`, `lock BOOLEAN`, `spre_refno INT64`, `ptre_refno INT64`, `cata_hash STRING`, `leve LIST<INT32>` | `leve` 保留 LIST 结构，`cata_hash` 需要同步 `TO_CATA` | `TO_SPRE`, `TO_PTRE`, `TO_CATA` |
| PIPE | `Attr_PIPE` | `status_code STRING`, `lock BOOLEAN`, `spre_refno INT64`, `cata_refno INT64`, `diameter DOUBLE`, `length DOUBLE`, `route LIST<INT64>` | `route` 为可变数组，仍保留 JSON 备份 | `TO_SPRE`, `TO_CATA`, `TO_ROUTE_SEG` |
| PLATFORM | `Attr_PLATFORM` | `status_code STRING`, `geom_refno INT64`, `matl STRING`, `weight DOUBLE`, `owner_refno INT64` | `owner_refno` 仅同步 `REL_ATTR`，不创建 `TO_OWNER` | `TO_GEOM` |

规格文件存放在 `external/rs-core/resource/attr_table_specs/`，以 YAML 形式描述：

```yaml
noun: ELBO
table: Attr_ELBO
fields:
  - name: status_code
    type: String
    nullable: true
  - name: lock
    type: Bool
  - name: spre_refno
    type: Refno
    edge: TO_SPRE
  - name: ptre_refno
    type: Refno
    edge: TO_PTRE
  - name: cata_hash
    type: String
    edge: TO_CATA
  - name: leve
    type: List<Int32>
    cache: true
```

代码生成脚本可读取该目录，生成 Rust 常量与 Kuzu `CREATE TABLE` 语句，确保 schema 与实现同步迭代。

## 外部引用清单（首批）

| 来源 NOUN | 属性/语义 | 目标实体 | 需要的 Edge | 备注 |
| --------- | ---------- | -------- | ----------- | ---- |
| ELBO | SPRE_REFNO | SPRE | `TO_SPRE` | 断面引用 |
| ELBO | PTRE_REFNO | PTRE | `TO_PTRE` | 点集引用 |
| ELBO | CATA_HASH / CATA_REF | CATA | `TO_CATA` | 元件库 |
| PIPE | SPRE_REFNO | SPRE | `TO_SPRE` | 与 ELBO 复用 |
| PIPE | CATA_REFNO | CATA | `TO_CATA` | |
| PLATFORM | GEOM_REFNO | GEOM | `TO_GEOM` | 示例 |

后续可根据 Surreal 查询统计补齐表格，通过脚本生成边定义。

## NamedAttrMap → Kuzu 类型映射与解析策略

| `NamedAttrValue` 变体 | Kuzu `LogicalType` | 强类型列示例 | 写入注意点 |
| --------------------- | ------------------ | ------------- | ---------- |
| `IntegerType` / `I32` | `Int64`            | `status_code_i32` | 落表统一升位为 `INT64`，读取时再降位 |
| `F32Type`             | `Double`           | `design_temp` | 保留精度，必要时使用 `DECIMAL` |
| `StringType` / `WordType` | `String`       | `cata_hash` / `body_size` | `WordType` 需要在入库前做 dehash |
| `BoolType`            | `Bool`             | `lock`         | —— |
| `RefU64Type` / `RefnoEnumType` | `Int64`   | `spre_refno` | 同步生成 `TO_<NOUN>` 边和 `attr_ref` |
| `Vec3Type`            | `STRUCT{x DOUBLE, y DOUBLE, z DOUBLE}` | `position` | 结构化存储，保留 JSON 备份兜底 |
| `F32VecType` / `DoubleArrayType` | `List<Double>` | `level` | 列长度可变，直接映射 `LIST` |
| `IntArrayType`        | `List<Int64>`      | `route`        | —— |
| `StringArrayType`     | `List<String>`     | `tags`         | —— |
| 复杂对象 / 未知类型   | `String` (JSON)    | `ext_json`     | 作为兜底，防止信息缺失 |

解析流水线：

1. 使用 `AttrTableSpec` 将 `NamedAttrMap` 拆分为 **强类型字段** 与 **动态字段** 两部分。
2. 强类型字段写入 `Attr_<NOUN>`，并记录需要建立的 `TO_<TARGET>` 引用列表。
3. 动态字段（含 UDA）继续写入 `Attribute` 节点，通过 `REL_ATTR` 挂载在 `PE` 下。
4. 生成并回写 `pe.named_attr_json`，保证 Surreal 风格的快速读取路径。

## 写入/读取流程伪代码

### save_attmap_kuzu(refno, noun, attmap)

1. 解析 `attmap` → 强类型字段结构体。
2. `MERGE (attr:Attr_<NOUN> {refno}) SET attr.<cols> = ...`。
3. `MATCH (pe:PE {refno}) MERGE (pe)-[:REL_ATTR]->(attr)`，更新 `pe.attr_ref`。
4. 遍历引用字段：
   - 若值有效，`MATCH` 目标节点；
   - `MERGE (pe)-[:TO_<TARGET>]->(target)`；
   - 维护引用属性节点（如 `Attribute`）。
5. 构造 `named_attr_json` 并写回 `pe`。
6. 更新 UDA：`MERGE`/`DELETE` 通用 `Attribute` 节点，保持 Surreal 行为。

### get_named_attmap_kuzu(refno)

1. `MATCH (pe:PE {refno}) RETURN pe.named_attr_json`，若存在直接反序列化。
2. 否则：
   - `MATCH (pe)-[:REL_ATTR]->(attr:Attr_<NOUN>)` 拉取强类型字段。
   - `MATCH (pe)-[:REL_ATTR]->(:Attribute)` 拉取 UDA/动态字段。
   - 聚合两份数据，写入缓存。
3. 返回 `NamedAttrMap`。

### get_spre_ptre(refno)

```cypher
MATCH (pe:PE {refno})
OPTIONAL MATCH (pe)-[:TO_SPRE]->(spre:PE {noun:'SPRE'})
OPTIONAL MATCH (pe)-[:TO_PTRE]->(ptre:PTRE)
RETURN spre.refno, ptre.refno;
```

必要时继续沿 `TO_CATA` → `TO_POINTSET` 追加 MATCH。

---

> **阶段一 TODO**：基于以上骨架，补齐更多 noun 示例与引用表，评估字段与边的自动生成策略。

## 阶段一完成清单

- [x] 三层模型定义（PE / Attr_<NOUN> / Attribute）及缓存策略
- [x] 多个 `noun` 的属性表与外部引用示例
- [x] `TO_<NOUN>` 外部引用边命名规则与生成思路
- [x] `NamedAttrMap` → Kuzu 类型映射与解析流水线
- [ ] 自动化脚本 PoC（阶段二执行）

上述素材确保 Surreal 解析结果可映射至 Kuzu，并为后续 Schema/代码实现提供直接输入，满足“解析能够保存数据到 Kuzu”的前置条件。
