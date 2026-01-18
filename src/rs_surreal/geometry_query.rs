/// 几何查询相关的数据结构和方法
///
/// 本模块提供了用于从 SurrealDB 批量查询几何参数和 AABB 数据的结构体和辅助方法
use crate::error::init_save_database_error;
use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::types::{PlantAabb, RefnoEnum, Thing};
use crate::utils::RecordIdExt;
use crate::{SUL_DB, SurrealQueryExt, gen_bytes_hash, get_inst_relate_keys};
use anyhow::anyhow;
use bevy_transform::prelude::Transform;
use dashmap::DashMap;
use parry3d::bounding_volume::{Aabb, BoundingVolume};
use parry3d::math::Isometry;
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut, Mul};
use surrealdb::types::{self as surrealdb_types, RecordId, RecordIdKey};
use surrealdb::types::{Kind, SurrealValue, Value};

/// 植物变换包装类型
///
/// 为 bevy_transform::prelude::Transform 提供 SurrealValue 实现的包装类型
/// 支持序列化、反序列化和数据库存储
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct PlantTransform(pub Transform);

impl Deref for PlantTransform {
    type Target = Transform;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for PlantTransform {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Transform> for PlantTransform {
    fn from(transform: Transform) -> Self {
        PlantTransform(transform)
    }
}

impl Mul<PlantTransform> for PlantTransform {
    type Output = PlantTransform;
    fn mul(self, other: PlantTransform) -> PlantTransform {
        PlantTransform(self.0 * other.0)
    }
}

impl Mul<Transform> for PlantTransform {
    type Output = PlantTransform;
    fn mul(self, other: Transform) -> PlantTransform {
        PlantTransform(self.0 * other)
    }
}

impl Default for PlantTransform {
    fn default() -> Self {
        PlantTransform(Transform::IDENTITY)
    }
}

impl Mul<&PlantTransform> for PlantTransform {
    type Output = PlantTransform;
    fn mul(self, other: &PlantTransform) -> PlantTransform {
        PlantTransform(self.0 * other.0)
    }
}

impl SurrealValue for PlantTransform {
    fn kind_of() -> Kind {
        Kind::Object
    }

    fn into_value(self) -> Value {
        serde_json::to_value(&self.0)
            .expect("序列化 PlantTransform 失败")
            .into_value()
    }

    fn from_value(value: Value) -> anyhow::Result<Self> {
        let json = serde_json::Value::from_value(value)?;
        Ok(PlantTransform(serde_json::from_value(json)?))
    }
}

/// 几何参数查询结构体
///
/// 用于分批查询 inst_geo 的几何参数，配合网格生成的并发处理
///
/// # 字段
///
/// * `id` - inst_geo 的原始记录 ID（来自 SurrealDB：record::id(id) 字符串化）
/// * `param` - PDMS 几何参数（用于生成 OCC 形体与后续网格化）
#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
pub struct QueryGeoParam {
    pub id: RecordId,
    pub param: PdmsGeoParam,
    /// 是否为单位 mesh：true=通过 transform 缩放，false=通过 mesh 顶点缩放
    /// SQL 查询需使用 `?? false` 处理 NULL 值
    #[serde(default)]
    pub unit_flag: bool,
}

/// 单个几何的变换与局部 AABB
///
/// 用于计算实例的全局包围盒
///
/// # 字段
///
/// * `trans` - 从几何到实例的局部变换
/// * `aabb` - 几何的局部包围盒
#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
pub struct GeoAabbTrans {
    pub trans: PlantTransform,
    pub aabb: PlantAabb,
}

/// inst_geo 查询结果
///
/// 用于表示 inst_geo 的查询结果
///
/// # 字段
///
/// * `geo_id` - inst_geo 的 Thing ID
/// * `has_neg_relate` - 是否存在负实体关系（影响容差选择）
#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
pub struct QueryInstGeoResult {
    pub geo_id: RecordId,
    #[serde(default)]
    pub refno: Option<RefnoEnum>,
    pub has_neg_relate: bool,
}

/// AABB 查询参数结构体
///
/// 用于查询 inst_relate 的 AABB 计算所需字段
///
/// # 字段
///
/// * `id` - inst_relate 的 RecordId
/// * `refno` - 实例的参考号
/// * `noun` - 实例的类型名称
/// * `geo_aabbs` - 关联的几何 AABB 和变换列表
/// * `world_trans` - 实例的世界坐标变换
#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
pub struct QueryAabbParam {
    // pub id: RecordId,
    pub refno: RefnoEnum,
    pub noun: String,
    pub geo_aabbs: Vec<GeoAabbTrans>,
    pub world_trans: Option<PlantTransform>,
}

impl QueryAabbParam {
    pub fn refno(&self) -> RefnoEnum {
        self.refno.clone().into()
    }
}

/// 查询 inst_geo 的几何参数
///
/// 根据参考号列表查询对应的 inst_geo 几何参数
///
/// # 参数
///
/// * `refnos` - 参考号数组，会转换为 inst_relate key
/// * `replace_exist` - 是否替换已存在的几何数据
///   - true: 不过滤 aabb/meshed，允许覆盖，但仍过滤 bad
///   - false: 仅选择 aabb 为空、未网格化且非 bad 的几何
///
/// # 返回值
///
/// 返回 `QueryInstGeoResult` 列表，包含几何 ID 和是否存在负实体关系
pub async fn query_inst_geo_ids(
    refnos: &[RefnoEnum],
    replace_exist: bool,
) -> anyhow::Result<Vec<QueryInstGeoResult>> {
    let inst_keys = get_inst_relate_keys(refnos);

    let where_clause = if replace_exist {
        "where !out.bad"
    } else {
        "where out.aabb.d=none and !out.meshed and !out.bad"
    };

    let sql = format!(
        r#"
            array::group(
                select value (select  out as geo_id, ($parent<-neg_relate)[0] != none as has_neg_relate from out->geo_relate {})
                from {}
            );
        "#,
        where_clause, inst_keys
    );
    let results: Vec<QueryInstGeoResult> = SUL_DB.query_take(&sql, 0).await?;
    Ok(results)
}

/// 批量查询几何参数
///
/// 根据 inst_geo Thing ID 集合查询对应的几何参数
///
/// # 参数
///
/// * `inst_geo_ids` - inst_geo 的 Thing ID 字符串列表（逗号分隔的数字 ID）
///
/// # 返回值
///
/// 返回 `QueryGeoParam` 列表，包含几何 ID 和参数
pub async fn query_geo_params(inst_geo_ids: &str) -> anyhow::Result<Vec<QueryGeoParam>> {
    use crate::SUL_DB;

    // 将逗号分隔的数字 ID 转换为 Thing ID 格式：inst_geo:⟨id⟩
    let thing_ids = inst_geo_ids
        .split(',')
        .map(|id| format!("inst_geo:⟨{}⟩", id.trim()))
        .collect::<Vec<_>>()
        .join(", ");

    let sql = format!(
        "select id, param, unit_flag ?? false as unit_flag from [{}] where param != NONE",
        thing_ids
    );

    let mut result = SUL_DB.query_take(&sql, 0).await?;

    Ok(result)
}

/// 查询 inst_relate 的 AABB 计算所需数据
///
/// 根据 inst_relate 键集合查询实例的世界变换和关联几何的 AABB
///
/// # 参数
///
/// * `inst_keys` - inst_relate 的键集合字符串（SurrealDB 查询范围）
/// * `replace_exist` - 是否替换已存在的 AABB
///   - true: 查询所有实例
///   - false: 仅查询尚未写入 inst_relate_aabb 的实例（增量回填）
///
/// # 返回值
///
/// 返回 `QueryAabbParam` 列表，包含实例 ID、变换和关联几何的 AABB
pub async fn query_aabb_params(
    inst_keys: &str,
    replace_exist: bool,
) -> anyhow::Result<Vec<QueryAabbParam>> {
    use crate::SUL_DB;

    // 从 pe_transform 获取 world_trans（允许为 None，由调用方过滤）
    let mut sql = format!(
        r#"select id, in as refno,
        type::record("pe_transform", record::id(in)).world_trans.d as world_trans,
        in.noun as noun,
        (select out.aabb.d as aabb, trans.d as trans from out->geo_relate where out.aabb.d != none and trans.d != none)
        as geo_aabbs from {inst_keys}"#,
    );

    if !replace_exist {
        sql.push_str(" where array::len(in->inst_relate_aabb) = 0");
    }

    // println!("Executing SQL: {}", sql);
    let mut response = SUL_DB.query_response(&sql).await?;
    let result: Vec<QueryAabbParam> = response.take(0)?;

    Ok(result)
}

/// 保存 AABB 数据到 SurrealDB
///
/// 批量将 AABB 数据保存到 aabb 表中
///
/// # 参数
///
/// * `aabb_map` - AABB 哈希到 AABB 对象的映射
pub async fn save_aabb_to_surreal(aabb_map: &DashMap<String, Aabb>) {
    if !aabb_map.is_empty() {
        let keys = aabb_map
            .iter()
            .map(|kv| kv.key().clone())
            .collect::<Vec<_>>();
        for chunk in keys.chunks(300) {
            let mut sql = String::new();
            for k in chunk {
                let v = aabb_map.get(k).unwrap();
                let json = format!(
                    "{{'id':aabb:⟨{}⟩, 'd':{}}}",
                    k,
                    serde_json::to_string(v.value()).unwrap()
                );
                sql.push_str(&format!("INSERT IGNORE INTO aabb {};", json));
            }
            match SUL_DB.query_response(&sql).await {
                Ok(_) => {}
                Err(_) => {
                    init_save_database_error(&sql, &std::panic::Location::caller().to_string());
                }
            }
        }
    }
}

/// 保存点集数据到 SurrealDB
///
/// 批量将 Vec3 数据保存到 vec3 表中
///
/// # 参数
///
/// * `vec3_map` - Vec3 ID 到 JSON 字符串的映射
pub async fn save_pts_to_surreal(vec3_map: &DashMap<u64, String>) {
    if !vec3_map.is_empty() {
        let keys = vec3_map.iter().map(|kv| *kv.key()).collect::<Vec<_>>();
        for chunk in keys.chunks(100) {
            let mut sql = String::new();
            for &k in chunk {
                let v = vec3_map.get(&k).unwrap();
                let json = format!("{{'id':vec3:⟨{}⟩, 'd':{}}}", k, v.value());
                sql.push_str(&format!("INSERT IGNORE INTO vec3 {};", json));
            }
            match SUL_DB.query_response(&sql).await {
                Ok(_) => {}
                Err(_e) => {
                    init_save_database_error(&sql, &std::panic::Location::caller().to_string());
                }
            };
        }
    }
}

/// 更新实例关联的包围盒数据
///
/// 根据参考号批量计算并写入 inst_relate_aabb（不再更新 inst_relate.aabb）
///
/// # 参数
///
/// * `refnos` - 参考号数组
/// * `replace_exist` - 是否替换已存在的包围盒数据
///   - true: 替换所有 AABB
///   - false: 仅回填尚未写入 inst_relate_aabb 的实例（增量写入）
///
/// # 返回值
///
/// 返回 `anyhow::Result<()>` 表示更新是否成功
///
/// # 说明
///
/// 该方法会：
/// 1. 查询 inst_relate 的世界变换和关联几何的 AABB
/// 2. 计算每个实例的全局 AABB（通过变换合并所有几何 AABB）
/// 3. 批量更新到 SurrealDB
/// 4. 保存 AABB 数据到 aabb 表中去重存储
///
/// # SQL 说明
///
/// - world_trans.d != none：仅处理拥有世界变换的实例
/// - 子查询 out->geo_relate 仅保留 out.aabb.d != none 且 trans.d != none 的几何（有局部AABB且有变换）
/// - 若 !replace_exist 则追加条件 and array::len(in->inst_relate_aabb)=0，避免覆盖已存在的实例 AABB（增量回填）
pub async fn update_inst_relate_aabbs_by_refnos(
    refnos: &[RefnoEnum],
    replace_exist: bool,
) -> anyhow::Result<()> {
    const CHUNK: usize = 100;

    let aabb_map = DashMap::new();
    for chunk in refnos.chunks(CHUNK) {
        if chunk.is_empty() {
            continue;
        }
        let inst_keys = get_inst_relate_keys(chunk);

        // 查询 AABB 参数
        let result = query_aabb_params(&inst_keys, replace_exist).await?;

        let mut relate_sql = String::new();
        for r in result {
            // 过滤 world_trans 为 None 的记录
            let Some(world_trans) = r.world_trans else { continue };
            
            // 计算合并后的 AABB
            let mut aabb = Aabb::new_invalid();
            for g in &r.geo_aabbs {
                let t = world_trans * &g.trans;
                let tmp_aabb = g.aabb.scaled(&t.scale.into());
                let tmp_aabb = tmp_aabb.transform_by(&Isometry {
                    rotation: t.rotation.into(),
                    translation: t.translation.into(),
                });
                aabb.merge(&tmp_aabb);
            }

            // 过滤无效 AABB
            if aabb.extents().magnitude().is_nan() || aabb.extents().magnitude().is_infinite() {
                #[cfg(feature = "debug_model")]
                eprintln!("发现无效 AABB for refno: {:?}", r.refno);
                continue;
            }

            let aabb_hash = gen_bytes_hash(&aabb).to_string();
            aabb_map.entry(aabb_hash.clone()).or_insert(aabb);

            let refno = r.refno();
            let edge_id = refno.to_table_key("inst_relate_aabb");
            let pe_key = refno.to_pe_key();

            if replace_exist {
                relate_sql.push_str(&format!("DELETE {};", edge_id));
            }

            let sql = format!(
                "INSERT IGNORE INTO inst_relate_aabb {{ id: {}, in: {}, out: aabb:⟨{}⟩ }};",
                edge_id, pe_key, aabb_hash
            );
            relate_sql.push_str(&sql);
        }

        if !relate_sql.is_empty() {
            SUL_DB.query_response(&relate_sql).await?;
        }
    }

    // 批量保存 AABB 到 aabb 表
    save_aabb_to_surreal(&aabb_map).await;

    Ok(())
}
