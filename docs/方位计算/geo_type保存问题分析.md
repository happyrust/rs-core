# geo_type 保存问题分析

## 问题描述

保存到数据库的 `geo_relate` 记录中，`geo_type` 字段始终是 `'Pos'`，而不是实际的几何类型名称（如 `'SweepSolid'`、`'PrimLoft'` 等）。

## 问题根源

### 1. `geo_type` 字段的双重含义

在代码中存在两种不同的 `geo_type` 概念：

1. **布尔运算类型** (`GeoBasicType` 枚举):
   - `Pos`: 正实体
   - `Neg`: 负实体
   - `Compound`: 复合实体（包含负实体）
   - `CataNeg`: 目录负实体
   - `CataCrossNeg`: 目录交叉负实体

2. **几何类型名称** (字符串):
   - `"SweepSolid"`: 扫描实体
   - `"PrimLoft"`: 基本放样
   - `"Extrusion"`: 拉伸
   - `"Box"`: 盒子
   - 等等

### 2. 当前代码中的问题

#### 在 `cata_model.rs` 中（第 670-691 行）

```rust
let geo_type = if is_ngmr {
    GeoBasicType::CataCrossNeg
} else if is_neg {
    GeoBasicType::CataNeg
} else if !cata_neg_refnos.is_empty() {
    GeoBasicType::Compound
} else {
    GeoBasicType::Pos  // ❌ 这里总是返回 Pos
};

let geom_inst = EleInstGeo {
    // ...
    geo_param: csg_shape
        .convert_to_geo_param()
        .unwrap_or(PdmsGeoParam::Unknown),  // ✅ 这里获取了实际的几何类型
    geo_type,  // ❌ 但这里使用的是布尔运算类型
    // ...
};
```

**问题**: `geo_type` 字段使用的是 `GeoBasicType`（布尔运算类型），而不是实际的几何类型名称。

#### 在 `pdms_inst.rs` 中（第 94 行）

```rust
let relate_json = format!(
    r#"in: inst_info:⟨{0}⟩, out: inst_geo:⟨{1}⟩, trans: trans:⟨{2}⟩, geom_refno: pe:{3}, pts: [{4}], geo_type: '{5}', visible: {6} {7}"#,
    // ...
    inst.geo_type,  // ❌ 这里使用的是 GeoBasicType，而不是实际的几何类型名称
    // ...
);
```

**问题**: 保存到数据库时，使用的是 `inst.geo_type`（`GeoBasicType`），而不是 `geo_param.type_name()`。

#### 在 `manifold_bool.rs` 中（第 227 行）

```rust
let relate_sql = format!(
    "relate {}->geo_relate->inst_geo:⟨{}⟩ set geom_refno=pe:⟨{}⟩, geo_type='Pos', trans=trans:⟨0⟩, visible = true;",
    // ...
);
```

**问题**: 硬编码为 `'Pos'`，没有使用实际的几何类型名称。

## 解决方案

### 方案 1: 使用 `geo_param.type_name()` 作为 `geo_type`

**修改位置**: `cata_model.rs` 第 670-691 行

```rust
// 获取实际的几何类型名称
let actual_geo_type_name = csg_shape
    .convert_to_geo_param()
    .map(|p| p.type_name())
    .unwrap_or_else(|| "Unknown".to_string());

// 保留布尔运算类型用于其他逻辑
let geo_basic_type = if is_ngmr {
    GeoBasicType::CataCrossNeg
} else if is_neg {
    GeoBasicType::CataNeg
} else if !cata_neg_refnos.is_empty() {
    GeoBasicType::Compound
} else {
    GeoBasicType::Pos
};

let geom_inst = EleInstGeo {
    // ...
    geo_param: csg_shape
        .convert_to_geo_param()
        .unwrap_or(PdmsGeoParam::Unknown),
    geo_type: actual_geo_type_name,  // ✅ 使用实际的几何类型名称
    geo_basic_type,  // ✅ 新增字段用于布尔运算类型
    // ...
};
```

**问题**: 需要修改 `EleInstGeo` 结构体，添加 `geo_basic_type` 字段。

### 方案 2: 在保存时使用 `geo_param.type_name()`

**修改位置**: `pdms_inst.rs` 第 94 行

```rust
let geo_type_name = inst.geo_param.type_name();  // ✅ 使用实际的几何类型名称

let relate_json = format!(
    r#"in: inst_info:⟨{0}⟩, out: inst_geo:⟨{1}⟩, trans: trans:⟨{2}⟩, geom_refno: pe:{3}, pts: [{4}], geo_type: '{5}', visible: {6} {7}"#,
    // ...
    geo_type_name,  // ✅ 使用实际的几何类型名称
    // ...
);
```

**优点**: 不需要修改 `EleInstGeo` 结构体，只需要在保存时使用正确的值。

**问题**: `inst.geo_type` 字段仍然存储的是 `GeoBasicType`，可能在其他地方被使用。

### 方案 3: 分离两个字段

**修改位置**: 数据库 schema 和所有相关代码

1. **`geo_type`**: 存储实际的几何类型名称（如 `"SweepSolid"`）
2. **`geo_basic_type`**: 存储布尔运算类型（如 `"Pos"`、`"Neg"`）

**优点**: 语义清晰，两个字段各司其职。

**缺点**: 需要修改数据库 schema 和所有相关查询代码。

## 推荐方案

**推荐使用方案 2**，原因：
1. 修改范围小，只需要修改保存逻辑
2. 不需要修改数据库 schema
3. 向后兼容性好

**实施步骤**:
1. 修改 `pdms_inst.rs`，使用 `inst.geo_param.type_name()` 作为 `geo_type`
2. 修改 `manifold_bool.rs`，从原始数据中获取几何类型名称
3. 检查所有使用 `geo_type` 字段的查询，确认是否需要区分布尔运算类型和几何类型名称

## 相关代码位置

- `gen-model-fork/src/fast_model/cata_model.rs` (第 670-691 行): 设置 `geo_type`
- `gen-model-fork/src/fast_model/pdms_inst.rs` (第 94 行): 保存 `geo_type` 到数据库
- `gen-model-fork/src/fast_model/manifold_bool.rs` (第 227 行): 硬编码 `geo_type='Pos'`
- `rs-core/src/rs_surreal/operation/geometry_op.rs` (第 37 行): `persist_geo_relates_for_shapes` 函数接收 `geo_type` 参数

---

**创建时间**: 2025-01-XX  
**问题**: `geo_type` 字段保存的是布尔运算类型（`'Pos'`），而不是实际的几何类型名称（如 `'SweepSolid'`）





