use crate::pdms_types::PdmsGenericType;
use crate::rs_surreal::geometry_query::PlantTransform;
use crate::shape::pdms_shape::RsVec3;
use crate::types::PlantAabb;
use crate::{RefU64, RefnoEnum, SUL_DB, SurlValue, SurrealQueryExt, get_inst_relate_keys};
use anyhow::Context;
use bevy_transform::components::Transform;
use chrono::{DateTime, Local, NaiveDateTime};
use glam::{DVec3, Vec3};
use parry3d::bounding_volume::Aabb;
use serde::de::DeserializeOwned;
use serde_derive::{Deserialize, Serialize};
use serde_json;
use serde_with::serde_as;
use surrealdb::types as surrealdb_types;
use surrealdb::types::{Kind, SurrealValue, Value};

/// 完整的 Ptset 点数据，包含位置和方向信息
#[derive(Serialize, Deserialize, Debug, Clone, Default, SurrealValue)]
pub struct FullPtsetPoint {
    /// 点位置
    pub pt: RsVec3,
    /// 主方向（连接方向）
    #[serde(default)]
    pub dir: Option<RsVec3>,
    /// 参考方向
    #[serde(default)]
    pub ref_dir: Option<RsVec3>,
    /// 点编号
    #[serde(default)]
    pub number: i32,
    /// 方向标志
    #[serde(default)]
    pub dir_flag: f32,
    /// 口径
    #[serde(default)]
    pub pbore: f32,
    /// 连接类型
    #[serde(default)]
    pub pconnect: String,
}

/// 初始化数据库的所有模型相关表结构和索引
pub async fn init_model_tables() -> anyhow::Result<()> {
    return Ok(());
    // 1. 定义关系表 (RELATION)
    // 这些表必须显式定义为 TYPE RELATION，否则如果第一条插入不是 relate 语句可能会创建为普通表
    let relation_tables = [
        "inst_relate",
        "inst_relate_aabb",
        "inst_relate_bool",
        "inst_relate_cata_bool",
        "geo_relate",
        "ngmr_relate",
        "neg_relate",
        "tubi_relate",
    ];

    for table in relation_tables {
        let sql = format!("DEFINE TABLE IF NOT EXISTS {} TYPE RELATION;", table);
        let _ = SUL_DB.query(sql).await;
    }

    // 2. 定义普通表 (NORMAL/SCHEMALESS)
    // 虽然 SurrealDB 默认是 Schemaless，但显式定义是个好习惯
    let normal_tables = ["inst_geo", "inst_info", "tubi_info"];
    for table in normal_tables {
        let sql = format!("DEFINE TABLE IF NOT EXISTS {} TYPE NORMAL;", table);
        let _ = SUL_DB.query(sql).await;
    }

    // 3. 创建 inst_relate 的核心索引
    let create_index_sql = "
        DEFINE INDEX IF NOT EXISTS idx_inst_relate_zone_refno ON TABLE inst_relate COLUMNS zone_refno;
        DEFINE INDEX IF NOT EXISTS idx_inst_relate_in ON TABLE inst_relate COLUMNS in;
        DEFINE INDEX IF NOT EXISTS idx_inst_relate_out ON TABLE inst_relate COLUMNS out;
    ";
    let _ = SUL_DB.query(create_index_sql).await?;

    // 4. 清理旧的计算字段定义（已弃用，改为在查询中直接使用 graph traversal）
    // world_trans 和 world_aabb 不再使用 <future> 计算字段，避免性能问题和定义不一致
    let remove_old_fields_sql = r#"
        REMOVE FIELD IF EXISTS world_trans ON TABLE pe;
        REMOVE FIELD IF EXISTS world_aabb ON TABLE pe;
    "#;
    let _ = SUL_DB.query(remove_old_fields_sql).await;

    Ok(())
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, SurrealValue)]
pub struct TubiInstQuery {
    pub refno: RefnoEnum,
    pub leave: RefnoEnum,
    pub generic: Option<String>,
    #[serde(default)]
    pub world_aabb: Option<PlantAabb>,
    pub world_trans: PlantTransform,
    pub geo_hash: String,
    pub date: Option<surrealdb::types::Datetime>,
    /// 规格值（来自 ZONE 的 owner.spec_value）
    pub spec_value: Option<i64>,
}

/// 将 SurrealDB 的原始值向量解码为目标类型列表
///
/// # 参数
///
/// * `values` - 从查询结果中获取的 `SurlValue` 向量
///
/// # 返回值
///
/// 返回解码后的目标类型向量，若解码失败则返回错误
fn decode_values<T: DeserializeOwned>(values: Vec<SurlValue>) -> anyhow::Result<Vec<T>> {
    values
        .into_iter()
        .map(|value| {
            let json = value.into_json_value();
            serde_json::from_value(json).context("failed to deserialize Surreal value")
        })
        .collect()
}

/// 根据分支构件编号批量查询 Tubi 实例数据
///
/// # 参数
///
/// * `bran_refnos` - 需要查询的分支构件编号切片
///
/// # 返回值
///
/// 返回符合条件的 `TubiInstQuery` 列表
///
/// # 注意
///
/// `tubi_relate` 表的 ID 格式是 `[pe:⟨第一个子元素refno⟩, index]`，
/// 而不是 `[pe:⟨BRAN_refno⟩, index]`。因此需要先查询 BRAN 的第一个子元素，
/// 然后用它来查询 `tubi_relate`。
pub async fn query_tubi_insts_by_brans(
    bran_refnos: &[RefnoEnum],
) -> anyhow::Result<Vec<TubiInstQuery>> {
    if bran_refnos.is_empty() {
        return Ok(Vec::new());
    }

    let mut all_results = Vec::new();
    for bran_refno in bran_refnos {
        let pe_key = bran_refno.to_pe_key();
        // 使用 ID range 查询：tubi_relate 的 ID 格式是 [pe:branch_refno, index]
        // 直接用 range 查询比 WHERE 条件更高效
        let sql = format!(
            r#"
            SELECT
                id[0] as refno,
                in as leave,
                id[0].old_pe as old_refno,
                id[0].owner.noun as generic,
                aabb.d as world_aabb,
                world_trans.d as world_trans,
                record::id(geo) as geo_hash,
                date,
                spec_value
            FROM tubi_relate:[{}, 0]..[{}, 999999]
            }}
            "#,
            pe_key, pe_key
        );
        let mut results: Vec<TubiInstQuery> = SUL_DB.query_take(&sql, 0).await?;

        all_results.append(&mut results);
    }
    Ok(all_results)
}

/// 根据流程构件编号批量查询 Tubi 实例数据
///
/// # 参数
///
/// * `refnos` - 需要查询的流程构件编号切片
///
/// # 返回值
///
/// 返回符合条件的 `TubiInstQuery` 列表
pub async fn query_tubi_insts_by_flow(refnos: &[RefnoEnum]) -> anyhow::Result<Vec<TubiInstQuery>> {
    if refnos.is_empty() {
        return Ok(Vec::new());
    }

    let mut all_results = Vec::new();
    for refno in refnos {
        let pe_key = refno.to_pe_key();
        let sql = format!(
            r#"
            SELECT
                id[0] as refno,
                in as leave,
                id[0].old_pe as old_refno,
                id[0].owner.noun as generic,
                aabb.d as world_aabb,
                world_trans.d as world_trans,
                record::id(geo) as geo_hash,
                id[0].dt as date,
                spec_value
            FROM tubi_relate
            WHERE (in = {} OR out = {})
            "#,
            pe_key, pe_key
        );

        let mut results: Vec<TubiInstQuery> = SUL_DB.query_take(&sql, 0).await?;
        all_results.append(&mut results);
    }

    Ok(all_results)
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Default, SurrealValue)]
pub struct ModelHashInst {
    pub geo_hash: String,
    #[serde(default)]
    pub transform: PlantTransform,
    #[serde(default)]
    pub is_tubi: bool,
    /// 是否为单位 mesh：true=通过 transform 缩放，false=通过 mesh 顶点缩放
    /// SQL 查询需使用 `?? false` 处理 NULL 值
    #[serde(default)]
    pub unit_flag: bool,
}

#[derive(Debug)]
pub struct ModelInstData {
    pub owner: RefnoEnum,
    pub has_neg: bool,
    pub insts: Vec<ModelHashInst>,
    pub generic: PdmsGenericType,
    pub world_trans: Transform,
    pub world_aabb: PlantAabb,
    pub ptset: Vec<Vec3>,
    pub is_bran_tubi: bool,
    pub date: NaiveDateTime,
}

///
/// 几何实例查询结构体
#[derive(Serialize, Deserialize, Debug, SurrealValue)]
pub struct GeomInstQuery {
    /// 构件编号，别名为id
    #[serde(alias = "id")]
    pub refno: RefnoEnum,
    /// 所属构件编号
    pub owner: RefnoEnum,
    /// 世界坐标系下的包围盒（可能为空）
    #[serde(default)]
    pub world_aabb: Option<PlantAabb>,
    /// 世界坐标系下的变换矩阵
    pub world_trans: PlantTransform,
    /// 几何实例列表
    pub insts: Vec<ModelHashInst>,
    /// 是否为布尔运算结果
    /// true: 几何体变换已包含世界变换，导出时直接使用 local_transform
    /// false: 普通几何体，导出时需要 world_transform × local_transform
    #[serde(default)]
    pub has_neg: bool,
}

/// 几何点集查询结构体
#[derive(Serialize, Deserialize, Debug, SurrealValue)]
pub struct GeomPtsQuery {
    /// 构件编号，别名为id
    #[serde(alias = "id")]
    pub refno: RefnoEnum,
    /// 世界坐标系下的变换矩阵
    pub world_trans: PlantTransform,
    /// 世界坐标系下的包围盒（可能为空）
    #[serde(default)]
    pub world_aabb: Option<PlantAabb>,
    /// 点集组，每组包含一个变换矩阵和可选的点集数据
    pub pts_group: Vec<(PlantTransform, Option<Vec<RsVec3>>)>,
}

//=============================================================================
// 导出专用查询结构体和函数
//=============================================================================

/// 导出专用：几何实例的 hash 引用（不含实际数据值）
#[derive(Serialize, Deserialize, Debug, Clone, Default, SurrealValue)]
pub struct ExportInstHash {
    /// 几何体 hash（geo 表的 record ID）
    pub geo_hash: String,
    /// 几何体变换 hash（trans 表的 record ID）
    #[serde(default)]
    pub trans_hash: Option<String>,
    /// 是否为单位 mesh
    #[serde(default)]
    pub unit_flag: bool,
}

/// 导出专用：构件几何实例查询结果（只含 hash 引用）
#[derive(Serialize, Deserialize, Debug, SurrealValue)]
pub struct ExportInstQuery {
    /// 构件编号
    #[serde(alias = "id")]
    pub refno: RefnoEnum,
    /// 所属构件编号
    pub owner: RefnoEnum,
    /// 世界包围盒 hash（aabb 表的 record ID）
    #[serde(default)]
    pub world_aabb_hash: Option<String>,
    /// 世界变换 hash（trans 表的 record ID）
    #[serde(default)]
    pub world_trans_hash: Option<String>,
    /// 几何实例列表（只含 hash 引用）
    pub insts: Vec<ExportInstHash>,
    /// 是否为布尔运算结果
    #[serde(default)]
    pub has_neg: bool,
}

/// 导出专用：查询几何实例的 hash 引用（不查询实际数据值）
///
/// # 参数
/// * `refnos` - 构件编号迭代器
/// * `enable_holes` - 是否启用布尔运算结果查询
///
/// # 返回值
/// 返回只包含 hash 引用的查询结果，用于导出时直接引用 trans.json 和 aabb.json
pub async fn query_insts_for_export(
    refnos: impl IntoIterator<Item = &RefnoEnum>,
    enable_holes: bool,
) -> anyhow::Result<Vec<ExportInstQuery>> {
    let refnos = refnos.into_iter().cloned().collect::<Vec<_>>();
    if refnos.is_empty() {
        return Ok(Vec::new());
    }

    let batch_size = 50;
    let mut results = Vec::new();

    for chunk in refnos.chunks(batch_size) {
        if enable_holes {
            // ========== 路径 A：布尔结果查询 ==========
            let bool_keys: Vec<String> = chunk.iter().map(|r| format!("inst_relate_bool:{}", r)).collect();
            let bool_keys_str = bool_keys.join(",");
            
            // 只查询 hash ID，不查询实际数据
            // 布尔运算结果的 mesh 已经在世界坐标系下，geo_instances 的 trans_hash 应该是单位矩阵 "0"
            let bool_sql = format!(
                r#"
                SELECT
                    refno,
                    refno.owner as owner,
                    record::id((refno->inst_relate_aabb[0].out)) as world_aabb_hash,
                    record::id(type::record("pe_transform", record::id(refno)).world_trans) as world_trans_hash,
                    [{{ "geo_hash": mesh_id, "trans_hash": "0", "unit_flag": false }}] as insts,
                    true as has_neg
                FROM [{bool_keys}]
                WHERE status = 'Success' AND type::record("pe_transform", record::id(refno)).world_trans.d != NONE
                "#,
                bool_keys = bool_keys_str
            );

            let mut bool_results: Vec<ExportInstQuery> = SUL_DB
                .query_take(&bool_sql, 0)
                .await
                .with_context(|| format!("query_insts_for_export bool SQL: {}", bool_sql))?;

            let bool_refnos: std::collections::HashSet<_> =
                bool_results.iter().map(|r| r.refno.clone()).collect();

            results.append(&mut bool_results);

            // ========== 路径 B：原始几何查询（排除已有布尔结果的） ==========
            let non_bool_keys: Vec<String> = chunk
                .iter()
                .filter(|r| !bool_refnos.contains(*r))
                .map(|r| r.to_inst_relate_key())
                .collect();

            if !non_bool_keys.is_empty() {
                let non_bool_keys_str = non_bool_keys.join(",");
                // 只查询 hash ID，不查询实际数据
                let geo_sql = format!(
                    r#"
                    SELECT
                        in as refno,
                        in.owner ?? in as owner,
                        record::id((in->inst_relate_aabb[0].out)) as world_aabb_hash,
                        record::id(type::record("pe_transform", record::id(in)).world_trans) as world_trans_hash,
                        (SELECT record::id(trans) as trans_hash, record::id(out) as geo_hash, out.unit_flag ?? false as unit_flag
                         FROM out->geo_relate
                         WHERE visible && (out.meshed || out.unit_flag || record::id(out) IN ['1','2','3'])
                           && (trans.d ?? NONE) != NONE
                           && geo_type IN ['Pos', 'DesiPos', 'CatePos']) as insts,
                        false as has_neg
                    FROM [{non_bool_keys}]
                    WHERE type::record("pe_transform", record::id(in)).world_trans.d != NONE
                    "#,
                    non_bool_keys = non_bool_keys_str
                );

                let mut geo_results: Vec<ExportInstQuery> = SUL_DB
                    .query_take(&geo_sql, 0)
                    .await
                    .with_context(|| format!("query_insts_for_export geo SQL: {}", geo_sql))?;
                results.append(&mut geo_results);
            }
        } else {
            // ========== enable_holes=false：始终返回原始几何 ==========
            let inst_relate_keys: Vec<String> =
                chunk.iter().map(|r| r.to_inst_relate_key()).collect();
            let inst_relate_keys_str = inst_relate_keys.join(",");

            // 只查询 hash ID，不查询实际数据
            let sql = format!(
                r#"
                SELECT
                    in as refno,
                    in.owner ?? in as owner,
                    record::id((in->inst_relate_aabb[0].out)) as world_aabb_hash,
                    record::id(type::record("pe_transform", record::id(in)).world_trans) as world_trans_hash,
                    (SELECT record::id(trans) as trans_hash, record::id(out) as geo_hash, out.unit_flag ?? false as unit_flag
                     FROM out->geo_relate
                     WHERE visible && (out.meshed || out.unit_flag || record::id(out) IN ['1','2','3'])
                       && (trans.d ?? NONE) != NONE
                       && geo_type IN ['Pos', 'DesiPos', 'CatePos']) as insts,
                    false as has_neg
                FROM [{inst_relate_keys}]
                WHERE type::record("pe_transform", record::id(in)).world_trans.d != NONE
                "#,
                inst_relate_keys = inst_relate_keys_str
            );

            let mut chunk_result: Vec<ExportInstQuery> = SUL_DB
                .query_take(&sql, 0)
                .await
                .with_context(|| format!("query_insts_for_export SQL: {}", sql))?;
            results.append(&mut chunk_result);
        }
    }

    Ok(results)
}

/// 根据最新refno查询最新insts
/// 根据构件编号查询几何实例信息
///
/// # 参数
///
/// * `refnos` - 构件编号迭代器
/// * `enable_holes` - 是否启用孔洞查询
///
/// # 返回值
///
/// 返回几何实例查询结果的向量
pub async fn query_insts(
    refnos: impl IntoIterator<Item = &RefnoEnum>,
    enable_holes: bool,
) -> anyhow::Result<Vec<GeomInstQuery>> {
    query_insts_with_batch(refnos, enable_holes, None).await
}

/// 查询几何实例信息（支持负实体）
///
/// # 参数
///
/// * `refnos` - 构件编号迭代器
/// * `enable_holes` - 是否启用孔洞查询
/// * `include_negative` - 是否包含负实体（Neg 类型）
///
/// # 返回值
///
/// 返回几何实例查询结果的向量
pub async fn query_insts_with_negative(
    refnos: impl IntoIterator<Item = &RefnoEnum>,
    enable_holes: bool,
    _include_negative: bool,
) -> anyhow::Result<Vec<GeomInstQuery>> {
    query_insts_with_batch(refnos, enable_holes, None).await
}

/// 批量查询几何实例信息（支持布尔运算结果）
///
/// # 参数
///
/// * `refnos` - 构件编号迭代器，指定要查询的实例
/// * `enable_holes` - 是否启用孔洞/布尔运算结果查询
///   - `true`: 优先返回布尔运算后的 mesh（如果 inst_relate_bool.status='Success'）
///   - `false`: 始终返回原始 geo_relate 中的 mesh 列表
/// * `batch_size` - 每批查询的数量，默认 50
///
/// # 返回值
///
/// 返回 `GeomInstQuery` 列表，包含：
/// - `refno`: 构件编号
/// - `owner`: 所属构件（从 pe.owner 获取）
/// - `world_aabb`: 世界坐标系下的包围盒（从 inst_relate_aabb 获取）
/// - `world_trans`: 世界坐标系变换矩阵（从 pe_transform 获取）
/// - `insts`: mesh 实例列表（geo_hash + transform）
/// - `has_neg`: 是否有负实体布尔运算结果
///
/// # 简化后的查询逻辑（v2）
///
/// 分两路并行查询，然后合并：
/// 1. **布尔结果路径**：直接从 `inst_relate_bool` 获取 status='Success' 的记录
/// 2. **原始几何路径**：从 `inst_relate` 查询，跳过已有布尔结果的 refnos
///
/// 字段来源：
/// - `owner` → `pe.owner`
/// - `world_trans` → `pe_transform:{refno}.world_trans.d`
/// - `world_aabb` → `inst_relate_aabb:{refno}.out.d`
///
/// # geo_type 语义约定
///
/// | geo_type | 含义 | 是否导出 |
/// |----------|------|----------|
/// | Pos | 原始几何（未布尔运算） | ✅ 导出 |
/// | DesiPos | 设计位置 | ✅ 导出 |
/// | CatePos | 布尔运算后的结果 | ✅ 导出 |
/// | Compound | 组合几何体（包含负实体引用） | ❌ 不导出 |
/// | CateNeg | 负实体 | ❌ 不导出 |
/// | CataCrossNeg | 交叉负实体 | ❌ 不导出 |
///
/// 查询条件：`geo_type IN ['Pos', 'DesiPos', 'CatePos']`
pub async fn query_insts_with_batch(
    refnos: impl IntoIterator<Item = &RefnoEnum>,
    enable_holes: bool,
    batch_size: Option<usize>,
) -> anyhow::Result<Vec<GeomInstQuery>> {
    let refnos = refnos.into_iter().cloned().collect::<Vec<_>>();
    if refnos.is_empty() {
        return Ok(Vec::new());
    }

    let batch = batch_size.unwrap_or(50).max(1);
    let mut results = Vec::new();

    for chunk in refnos.chunks(batch) {
        if enable_holes {
            // ========== 路径 A：布尔结果查询 ==========
            // 直接从 inst_relate_bool:{refno} 获取有成功布尔结果的记录
            let bool_keys: Vec<String> = chunk.iter().map(|r| format!("inst_relate_bool:{}", r)).collect();
            let bool_keys_str = bool_keys.join(",");
            
            // 使用 graph traversal 获取 world_aabb，不依赖计算字段
            let bool_sql = format!(
                r#"
                SELECT
                    refno,
                    refno.owner ?? refno as owner,
                    type::record("pe_transform", record::id(refno)).world_trans.d as world_trans,
                    (refno->inst_relate_aabb[0].out).d as world_aabb,
                    [{{ "transform": type::record("pe_transform", record::id(refno)).world_trans.d, "geo_hash": mesh_id, "is_tubi": false, "unit_flag": false }}] as insts,
                    true as has_neg
                FROM [{bool_keys}]
                WHERE status = 'Success' AND type::record("pe_transform", record::id(refno)).world_trans.d != NONE
                "#,
                bool_keys = bool_keys_str
            );

            let mut bool_results: Vec<GeomInstQuery> = SUL_DB
                .query_take(&bool_sql, 0)
                .await
                .with_context(|| format!("query_insts_with_batch bool SQL: {}", bool_sql))?;

            // 收集已有布尔结果的 refnos
            let bool_refnos: std::collections::HashSet<_> =
                bool_results.iter().map(|r| r.refno.clone()).collect();

            results.append(&mut bool_results);

            // ========== 路径 B：原始几何查询（排除已有布尔结果的） ==========
            let non_bool_keys: Vec<String> = chunk
                .iter()
                .filter(|r| !bool_refnos.contains(*r))
                .map(|r| r.to_inst_relate_key())
                .collect();

            if !non_bool_keys.is_empty() {
                let non_bool_keys_str = non_bool_keys.join(",");
                // 直接从 inst_relate:{refno} 查询
                // 使用 graph traversal 获取 world_aabb
                let geo_sql = format!(
                    r#"
                    SELECT
                        in as refno,
                        in.owner ?? in as owner,
                        type::record("pe_transform", record::id(in)).world_trans.d as world_trans,
                        (in->inst_relate_aabb[0].out).d as world_aabb,
                        (SELECT trans.d as transform, record::id(out) as geo_hash, false as is_tubi, out.unit_flag ?? false as unit_flag
                         FROM out->geo_relate
                         WHERE visible && (out.meshed || out.unit_flag || record::id(out) IN ['1','2','3'])
                           && (trans.d ?? NONE) != NONE
                           && geo_type IN ['Pos', 'CatePos']) as insts,
                        false as has_neg
                    FROM [{non_bool_keys}]
                    WHERE type::record("pe_transform", record::id(in)).world_trans.d != NONE
                    "#,
                    non_bool_keys = non_bool_keys_str
                );

                let mut geo_results: Vec<GeomInstQuery> = SUL_DB
                    .query_take(&geo_sql, 0)
                    .await
                    .with_context(|| format!("query_insts_with_batch geo SQL: {}", geo_sql))?;
                results.append(&mut geo_results);
            }
        } else {
            // ========== enable_holes=false：始终返回原始几何 ==========
            let inst_relate_keys: Vec<String> =
                chunk.iter().map(|r| r.to_inst_relate_key()).collect();
            let inst_relate_keys_str = inst_relate_keys.join(",");

            // 直接从 inst_relate:{refno} 查询
            // 使用 graph traversal 获取 world_aabb
            let sql = format!(
                r#"
                SELECT
                    in as refno,
                    in.owner ?? in as owner,
                    type::record("pe_transform", record::id(in)).world_trans.d as world_trans,
                    (in->inst_relate_aabb[0].out).d as world_aabb,
                    (SELECT trans.d as transform, record::id(out) as geo_hash, false as is_tubi, out.unit_flag ?? false as unit_flag
                     FROM out->geo_relate
                     WHERE visible && (out.meshed || out.unit_flag || record::id(out) IN ['1','2','3'])
                       && (trans.d ?? NONE) != NONE
                       && geo_type IN ['Pos', 'DesiPos', 'CatePos']) as insts,
                    false as has_neg
                FROM [{inst_relate_keys}]
                WHERE type::record("pe_transform", record::id(in)).world_trans.d != NONE
                "#,
                inst_relate_keys = inst_relate_keys_str
            );

            let mut chunk_result: Vec<GeomInstQuery> = SUL_DB
                .query_take(&sql, 0)
                .await
                .with_context(|| format!("query_insts_with_batch SQL: {}", sql))?;
            results.append(&mut chunk_result);
        }
    }

    Ok(results)
}

// todo 生成一个测试案例
// pub async fn query_history_insts(
//     refnos: impl IntoIterator<Item = &(RefnoEnum, u32)>,
// ) -> anyhow::Result<Vec<GeomInstQuery>> {
//     let history_inst_keys = refnos
//         .into_iter()
//         .map(|x| format!("inst_relate:{}_{}", x.0, x.1))
//         .collect::<Vec<_>>()
//         .join(",");

//     //todo 如果是ngmr relate, 也要测试一下有没有问题
//     //ngmr relate 的关系可以直接在inst boolean 做这个处理，不需要单独开方法
//     //ngmr的负实体最后再执行
//     let sql = format!(
//         r#"
//     select in.id as refno, in.owner as owner, generic, aabb.d as world_aabb, world_trans.d as world_trans, out.ptset.d.pt as pts,
//             if (in<-neg_relate)[0] != none && $parent.booled {{ [{{ "geo_hash": record::id(in.id) }}] }} else {{ (select trans.d as transform, record::id(out) as geo_hash from out->geo_relate where visible && trans.d != none && geo_type='Pos')  }} as insts
//             from {history_inst_keys} where aabb.d != none
//             "#
//     );
//     // println!("Query insts: {}", &sql);
//     let mut response = SUL_DB.query_response(sql).await?;
//     let mut geom_insts: Vec<GeomInstQuery> = response.take(0).unwrap();

//     Ok(geom_insts)
// }

//=============================================================================
// inst_relate 数据保存相关函数
//=============================================================================

use crate::geometry::ShapeInstancesData;
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use std::collections::HashMap;

/// 定义 dbnum_info_table 的更新事件
///
/// 当 pe 表有 CREATE/UPDATE/DELETE 事件时，自动更新 dbnum_info_table 的统计信息
#[cfg(feature = "surreal-save")]
pub async fn define_dbnum_event() -> anyhow::Result<()> {
    let event_sql = r#"
    DEFINE EVENT OVERWRITE update_dbnum_event ON pe WHEN $event = "CREATE" OR $event = "UPDATE" OR $event = "DELETE" THEN {
            -- 获取当前记录的 dbnum
            LET $dbnum = $value.dbnum;
            LET $id = record::id($value.id);
            let $id_parts = string::split($id, "_");
            let $ref_0 = <int>array::at($id_parts, 0);
            let $ref_1 = <int>array::at($id_parts, 1);
            let $is_delete = $value.deleted and $event = "UPDATE";
            let $max_sesno = if $after.sesno > $before.sesno?:0 { $after.sesno } else { $before.sesno };
            -- 根据事件类型处理  type::record("dbnum_info_table", $ref_0)
            IF $event = "CREATE"   {
                UPSERT type::record('dbnum_info_table', $ref_0) MERGE {
                    dbnum: $dbnum,
                    count: count?:0 + 1,
                    sesno: $max_sesno,
                    max_ref1: $ref_1,
                    updated_at: time::now()
                };
            } ELSE IF $event = "DELETE" OR $is_delete  {
                UPSERT type::record('dbnum_info_table', $ref_0) MERGE {
                    count: count - 1,
                    sesno: $max_sesno,
                    max_ref1: $ref_1,
                    updated_at: time::now()
                }
                WHERE count > 0;
            }  ELSE IF $event = "UPDATE" {
                UPSERT type::record('dbnum_info_table', $ref_0) MERGE {
                    sesno: $max_sesno,
                    updated_at: time::now()
                };
            };
        };
    "#;

    SUL_DB.query_response(event_sql).await?;
    Ok(())
}

/// 定义 dbnum_info_table 的更新事件 (非 surreal-save feature 时的空实现)
#[cfg(not(feature = "surreal-save"))]
pub async fn define_dbnum_event() -> anyhow::Result<()> {
    Ok(())
}

/// 级联删除 inst_relate 及其关联的 geo_relate 和 inst_geo 数据
///
/// 当 replace_mesh 开启时，需要完全删除之前生成的数据，包括：
/// - inst_geo: 几何体节点
/// - geo_relate: 几何关系边
/// - inst_info: 实例信息节点
/// - inst_relate: 实例关系边
///
/// # 参数
/// * `refnos` - 需要删除的 refno 列表
/// * `chunk_size` - 分批处理的大小
///
/// # 删除顺序
/// 1. inst_geo (最外层)
/// 2. geo_relate (关系边)
/// 3. inst_info (信息节点)
/// 4. inst_relate (关系边)
pub async fn delete_inst_relate_cascade(
    refnos: &[RefnoEnum],
    chunk_size: usize,
) -> anyhow::Result<()> {
    for chunk in refnos.chunks(chunk_size) {
        let mut delete_sql_vec = vec![];

        let mut inst_ids = vec![];
        for &refno in chunk {
            inst_ids.push(refno.to_inst_relate_key());
            let delete_sql = format!(
                r#"
                    delete array::flatten(select value [out, id, in] from {}->inst_info->geo_relate);
                "#,
                refno.to_inst_relate_key()
            );
            delete_sql_vec.push(delete_sql);
        }

        if !delete_sql_vec.is_empty() {
            let mut sql = "BEGIN TRANSACTION;\n".to_string();
            sql.push_str(&delete_sql_vec.join(""));
            sql.push_str(&format!("delete {};", inst_ids.join(",")));
            sql.push_str("\nCOMMIT TRANSACTION;");
            // println!("Delete Sql is {}", &sql);
            SUL_DB
                .query(sql)
                .await
                .expect("delete model insts info failed");
        }
    }

    Ok(())
}

/// 删除所有模型生成相关的数据
///
/// 删除 inst_relate、inst_geo、inst_info、geo_relate 四个表中的所有数据
///
/// # 参数
/// * `chunk_size` - 分批处理的大小
pub async fn delete_all_model_data() -> anyhow::Result<()> {
    let tables = [
        "inst_relate",
        "inst_geo",
        "inst_info",
        "tubi_relate",
        "geo_relate",
        "neg_relate",
        "ngmr_relate",
    ];
    let mut sql = "BEGIN TRANSACTION;\n".to_string();

    for table in &tables {
        sql.push_str(&format!("delete {};\n", table));
    }

    sql.push_str("COMMIT TRANSACTION;");

    println!("Delete Sql is: \n {}", &sql);

    SUL_DB.query(sql).await.unwrap();
    Ok(())
}
