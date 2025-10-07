# Stage1 数据流蓝图

本文档描述 SurrealDB 源数据解析到 Kuzu 三层模型的目标流程，指导阶段二后续代码实现。

## 1. 源数据入口
- `external/rs-core/src/parse/page.rs` 解析 PDMS 页，产出 `DbPageBasicInfo` / `SessionPageData`。
- `NamedAttrMap` 仍是解析层的聚合结构，但需进一步拆分为强类型字段、动态属性和外部引用。

## 2. 中间结构
解析完成后应转成如下中间结构，供同步与写入层使用：

```rust
pub struct PeRecord {
    pub refno: RefnoEnum,
    pub noun: String,
    pub dbnum: i32,
    pub name: Option<String>,
    pub sesno: Option<i32>,
    pub cache_json: Option<serde_json::Value>,
}

pub struct TypedAttrRecord {
    pub noun: String,
    pub refno: RefnoEnum,
    pub fields: BTreeMap<String, serde_json::Value>,
}

pub struct EdgeRecord {
    pub from: RefnoEnum,
    pub to: RefnoEnum,
    pub edge_type: EdgeType,
}

pub enum EdgeType {
    RelAttr,
    ToNoun { target_noun: String, field: String },
    Owner,
}
```

- `PeRecord` 保存 PE 基础信息以及缓存的 `named_attr_json`。
- `TypedAttrRecord` 负责投影到 `Attr_<NOUN>` 强类型表。
- `EdgeRecord` 承载 `REL_ATTR` 及所有 `TO_<NOUN>` 外部引用信息。

## 3. 缓冲与写入
- `sync::batch_optimizer` 需要扩展缓冲区，分别缓存三类记录，触发阈值时分流到 Surreal 与 Kuzu。
- Surreal 写入保持现有流程，仅更新 `NamedAttrMap`/UDA。
- Kuzu 写入按以下顺序执行：
  1. `MERGE` PE 基础信息，更新 `attr_ref`、`named_attr_json`。
  2. 写入对应的 `Attr_<NOUN>` 节点表并建立 `REL_ATTR`。
  3. 依 `EdgeRecord` 维护 `TO_*` 关系。
  4. 失败时回滚缓存或重试，避免缓存失效。

## 4. 缓存策略
- `named_attr_json` 作为读缓存，仅当强类型写入成功后才刷新。
- 当解析发现字段缺失或引用目标不存在时，应记录到异步补偿队列，避免主流程阻塞。

## 5. 验证入口
- `tests/kuzu_integration_test.rs` 增加用例比对 `PE` 与 `Attr_<NOUN>` 字段。
- `external/rs-core/src/bin/test_pe_sync.rs` 扩展校验：抽样比对 Surreal `NamedAttrMap` 与 Kuzu 强类型字段，验证 `TO_*` 关系完整性。

## 6. 后续阶段依赖
- 阶段二：根据本蓝图实现 schema 扩展与写入逻辑。
- 阶段三：`DatabaseAdapter::Hybrid` 衔接双写，确保同步批量器调用新的写入 API。
- 阶段四：封装通用查询，优先命中 `named_attr_json`，回退至强类型表聚合。
- 阶段五：补充迁移脚本与性能测试，验证双端一致性。

