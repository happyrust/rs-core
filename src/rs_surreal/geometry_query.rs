/// 几何查询相关的数据结构和方法
///
/// 本模块提供了用于从 SurrealDB 批量查询几何参数和 AABB 数据的结构体和辅助方法
use crate::error::init_save_database_error;
use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::plant_transform::Transform;
use crate::types::{RefnoEnum, Thing};
use crate::utils::RecordIdExt;
use crate::{SurrealQueryExt, get_inst_relate_keys, model_primary_db};
use dashmap::DashMap;
use parry3d::bounding_volume::Aabb;
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut, Mul};
use surrealdb::types::{self as surrealdb_types, RecordId, RecordIdKey};
use surrealdb::types::{Kind, SurrealValue, Value};

/// 植物变换包装类型
///
/// 为 crate::plant_transform::Transform 提供 SurrealValue 实现的包装类型
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

    fn from_value(value: Value) -> Result<Self, surrealdb::Error> {
        let json = serde_json::Value::from_value(value)?;
        serde_json::from_value(json)
            .map(PlantTransform)
            .map_err(|e| surrealdb::Error::internal(e.to_string()))
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
    #[serde(default)]
    pub has_cata_neg: bool,
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
                select value (select out as geo_id,
                    ($parent<-neg_relate)[0] != none as has_neg_relate,
                    $parent.has_cata_neg ?? false as has_cata_neg
                from $parent.out->geo_relate {})
                from {}
            );
        "#,
        where_clause, inst_keys
    );
    let results: Vec<QueryInstGeoResult> = model_primary_db().query_take(&sql, 0).await?;
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
    // 将逗号分隔的数字 ID 转换为 Thing ID 格式：inst_geo:⟨id⟩
    let thing_ids = inst_geo_ids
        .split(',')
        .map(|id| format!("inst_geo:⟨{}⟩", id.trim()))
        .collect::<Vec<_>>()
        .join(", ");

    let sql = format!("select id, param from [{}] where param != NONE", thing_ids);

    let mut result = model_primary_db().query_take(&sql, 0).await?;

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
                let d = serde_json::to_string(v.value()).unwrap();
                let id_key = if k.starts_with("aabb:") {
                    k.to_string()
                } else {
                    format!("aabb:⟨{}⟩", k)
                };
                sql.push_str(&format!("UPSERT {id_key} SET d = {d};"));
            }
            match model_primary_db().query_response(&sql).await {
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
            match model_primary_db().query_response(&sql).await {
                Ok(_) => {}
                Err(_e) => {
                    init_save_database_error(&sql, &std::panic::Location::caller().to_string());
                }
            };
        }
    }
}
