//! inst_relate 和 inst_geo 结构体定义
//!
//! 这个模块包含了 SurrealDB 中 inst_relate、inst_geo 和 geo_relate 表的结构体定义
//! 以及相应的 to_surql 方法用于生成 SurrealDB 插入语句

use crate::RefnoEnum;
use crate::shape::pdms_shape::RsVec3;
use bevy_transform::components::Transform;
use chrono::NaiveDateTime;
use glam::Vec3;
use parry3d::bounding_volume::Aabb;
use serde_derive::{Deserialize, Serialize};
use serde_with::serde_as;
use surrealdb::types as surrealdb_types;
use surrealdb::types::SurrealValue;

/// inst_relate 表结构体
/// 表示实例关系，连接PE元素和几何实例
#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InstRelate {
    /// 关系ID
    pub id: String,
    /// 输入节点 - PE元素引用
    #[serde(rename = "in")]
    pub input: RefnoEnum,
    /// 输出节点 - 几何实例引用
    pub out: String,
    /// 所属构件编号
    pub owner: RefnoEnum,
    /// 构件类型 (PIPE, ELBO, VALVE等)
    pub generic: String,
    /// 包围盒数据
    pub aabb: Option<AabbData>,
    /// 世界坐标变换矩阵
    pub world_trans: Option<TransformData>,
    /// 布尔运算ID (用于孔洞等负实体)
    pub booled_id: Option<String>,
    /// 时间戳
    pub dt: Option<NaiveDateTime>,
    /// 区域参考号
    pub zone_refno: Option<RefnoEnum>,
    /// 历史PE编号
    pub old_pe: Option<RefnoEnum>,
    /// 点集数据
    pub ptset: Option<PtsetData>,
}

/// inst_geo 表结构体
/// 表示几何实例的具体数据
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InstGeo {
    /// 几何实例ID
    pub id: String,
    /// 几何参数数据 (JSON格式存储)
    pub param: serde_json::Value,
    /// 是否已网格化
    pub meshed: bool,
    /// 是否可见
    pub visible: bool,
    /// 变换矩阵
    pub trans: Option<TransformData>,
    /// 几何类型 (Pos, Neg, CataNeg等)
    pub geo_type: String,
    /// 创建时间
    pub created_at: Option<NaiveDateTime>,
    /// 更新时间
    pub updated_at: Option<NaiveDateTime>,
    /// 是否为单位 mesh：true=通过 transform 缩放，false=通过 mesh 顶点缩放
    #[serde(default)]
    pub unit_flag: bool,
}

/// geo_relate 表结构体
/// 表示几何关系，连接实例和具体几何数据
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GeoRelate {
    /// 关系ID
    pub id: String,
    /// 输入节点 - 实例引用
    #[serde(rename = "in")]
    pub input: String,
    /// 输出节点 - 几何数据引用
    pub out: String,
    /// 变换矩阵
    pub trans: Option<TransformData>,
    /// 是否可见
    pub visible: bool,
    /// 是否已网格化
    pub meshed: bool,
    /// 几何类型
    pub geo_type: String,
    /// 几何参考号 (用于索引)
    pub geom_refno: Option<String>,
}

/// 包围盒数据结构
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AabbData {
    /// 包围盒数据
    pub d: Aabb,
}

/// 变换矩阵数据结构
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TransformData {
    /// 变换矩阵数据
    pub d: Transform,
}

/// 点集数据结构
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PtsetData {
    /// 点集数据
    pub d: PtsetContent,
}

/// 点集内容
#[derive(Serialize, Deserialize, Debug, Clone, SurrealValue)]
pub struct PtsetContent {
    /// 点坐标数组
    pub pt: Vec<RsVec3>,
}

impl InstRelate {
    /// 创建新的 InstRelate 实例
    pub fn new(
        id: String,
        input: RefnoEnum,
        out: String,
        owner: RefnoEnum,
        generic: String,
    ) -> Self {
        Self {
            id,
            input,
            out,
            owner,
            generic,
            aabb: None,
            world_trans: None,
            booled_id: None,
            dt: None,
            zone_refno: None,
            old_pe: None,
            ptset: None,
        }
    }

    /// 设置包围盒数据
    pub fn with_aabb(mut self, aabb: Aabb) -> Self {
        self.aabb = Some(AabbData { d: aabb });
        self
    }

    /// 设置世界变换矩阵
    pub fn with_world_trans(mut self, trans: Transform) -> Self {
        self.world_trans = Some(TransformData { d: trans });
        self
    }

    /// 设置布尔运算ID
    pub fn with_booled_id(mut self, booled_id: String) -> Self {
        self.booled_id = Some(booled_id);
        self
    }

    /// 设置时间戳
    pub fn with_dt(mut self, dt: NaiveDateTime) -> Self {
        self.dt = Some(dt);
        self
    }

    /// 设置区域参考号
    pub fn with_zone_refno(mut self, zone_refno: RefnoEnum) -> Self {
        self.zone_refno = Some(zone_refno);
        self
    }

    /// 设置历史PE编号
    pub fn with_old_pe(mut self, old_pe: RefnoEnum) -> Self {
        self.old_pe = Some(old_pe);
        self
    }

    /// 设置点集数据
    pub fn with_ptset(mut self, points: Vec<RsVec3>) -> Self {
        self.ptset = Some(PtsetData {
            d: PtsetContent { pt: points },
        });
        self
    }

    /// 生成 SurrealDB 插入语句
    /// 参考现有的 gen_sur_json 模式
    pub fn to_surql(&self) -> String {
        let aabb_str = match &self.aabb {
            Some(aabb) => format!(
                "{{ d: {} }}",
                serde_json::to_string(&aabb.d).unwrap_or_default()
            ),
            None => "NONE".to_string(),
        };

        let world_trans_str = match &self.world_trans {
            Some(trans) => format!(
                "{{ d: {} }}",
                serde_json::to_string(&trans.d).unwrap_or_default()
            ),
            None => "NONE".to_string(),
        };

        let ptset_str = match &self.ptset {
            Some(ptset) => format!(
                "{{ d: {} }}",
                serde_json::to_string(&ptset.d).unwrap_or_default()
            ),
            None => "NONE".to_string(),
        };

        let booled_id_str = match &self.booled_id {
            Some(id) => format!("'{}'", id),
            None => "NONE".to_string(),
        };

        let dt_str = match &self.dt {
            Some(dt) => format!("d'{}'", dt.format("%Y-%m-%dT%H:%M:%S")),
            None => "time::now()".to_string(),
        };

        let zone_refno_str = match &self.zone_refno {
            Some(refno) => format!("'{}'", refno),
            None => "NONE".to_string(),
        };

        let old_pe_str = match &self.old_pe {
            Some(refno) => format!("'{}'", refno),
            None => "NONE".to_string(),
        };

        format!(
            r#"CREATE inst_relate:{} SET
                in = pe:{},
                out = {},
                owner = pe:{},
                generic = '{}',
                aabb = {},
                world_trans = {},
                booled_id = {},
                dt = {},
                zone_refno = {},
                old_pe = {},
                ptset = {};
UPDATE pe:{} SET inst_relate_id = inst_relate:{};"#,
            self.id,
            self.input,
            self.out,
            self.owner,
            self.generic,
            aabb_str,
            world_trans_str,
            booled_id_str,
            dt_str,
            zone_refno_str,
            old_pe_str,
            ptset_str,
            self.input,
            self.id
        )
    }

    /// 生成 SurrealDB JSON 格式数据
    /// 参考现有的 gen_sur_json 模式
    pub fn gen_sur_json(&self) -> String {
        let mut json = serde_json::json!({
            "in": format!("pe:{}", self.input),
            "out": self.out,
            "owner": format!("pe:{}", self.owner),
            "generic": self.generic,
            "aabb": self.aabb,
            "world_trans": self.world_trans,
            "booled_id": self.booled_id,
            "dt": self.dt,
            "zone_refno": self.zone_refno,
            "old_pe": self.old_pe,
            "ptset": self.ptset,
        });

        // 添加 ID
        json["id"] = serde_json::json!(format!("inst_relate:{}", self.id));

        serde_json::to_string(&json).unwrap_or_default()
    }
}

impl InstGeo {
    /// 创建新的 InstGeo 实例
    pub fn new(
        id: String,
        param: serde_json::Value,
        meshed: bool,
        visible: bool,
        geo_type: String,
        unit_flag: bool,
    ) -> Self {
        Self {
            id,
            param,
            meshed,
            visible,
            trans: None,
            geo_type,
            created_at: None,
            updated_at: None,
            unit_flag,
        }
    }

    /// 设置变换矩阵
    pub fn with_trans(mut self, trans: Transform) -> Self {
        self.trans = Some(TransformData { d: trans });
        self
    }

    /// 设置创建时间
    pub fn with_created_at(mut self, created_at: NaiveDateTime) -> Self {
        self.created_at = Some(created_at);
        self
    }

    /// 设置更新时间
    pub fn with_updated_at(mut self, updated_at: NaiveDateTime) -> Self {
        self.updated_at = Some(updated_at);
        self
    }

    /// 生成 SurrealDB 插入语句
    pub fn to_surql(&self) -> String {
        let trans_str = match &self.trans {
            Some(trans) => format!(
                "{{ d: {} }}",
                serde_json::to_string(&trans.d).unwrap_or_default()
            ),
            None => "NONE".to_string(),
        };

        let created_at_str = match &self.created_at {
            Some(dt) => format!("d'{}'", dt.format("%Y-%m-%dT%H:%M:%S")),
            None => "time::now()".to_string(),
        };

        let updated_at_str = match &self.updated_at {
            Some(dt) => format!("d'{}'", dt.format("%Y-%m-%dT%H:%M:%S")),
            None => "time::now()".to_string(),
        };

        format!(
            r#"CREATE inst_geo:{} SET
                param = {},
                meshed = {},
                visible = {},
                trans = {},
                geo_type = '{}',
                created_at = {},
                updated_at = {},
                unit_flag = {};"#,
            self.id,
            self.param.to_string(),
            self.meshed,
            self.visible,
            trans_str,
            self.geo_type,
            created_at_str,
            updated_at_str,
            self.unit_flag
        )
    }

    /// 生成 SurrealDB JSON 格式数据
    pub fn gen_sur_json(&self) -> String {
        let mut json = serde_json::json!({
            "param": self.param,
            "meshed": self.meshed,
            "visible": self.visible,
            "trans": self.trans,
            "geo_type": self.geo_type,
            "created_at": self.created_at,
            "updated_at": self.updated_at,
            "unit_flag": self.unit_flag,
        });

        // 添加 ID
        json["id"] = serde_json::json!(format!("inst_geo:{}", self.id));

        serde_json::to_string(&json).unwrap_or_default()
    }
}

impl GeoRelate {
    /// 创建新的 GeoRelate 实例
    pub fn new(
        id: String,
        input: String,
        out: String,
        visible: bool,
        meshed: bool,
        geo_type: String,
    ) -> Self {
        Self {
            id,
            input,
            out,
            trans: None,
            visible,
            meshed,
            geo_type,
            geom_refno: None,
        }
    }

    /// 设置变换矩阵
    pub fn with_trans(mut self, trans: Transform) -> Self {
        self.trans = Some(TransformData { d: trans });
        self
    }

    /// 设置几何参考号
    pub fn with_geom_refno(mut self, geom_refno: String) -> Self {
        self.geom_refno = Some(geom_refno);
        self
    }

    /// 生成 SurrealDB 插入语句
    pub fn to_surql(&self) -> String {
        let trans_str = match &self.trans {
            Some(trans) => format!(
                "{{ d: {} }}",
                serde_json::to_string(&trans.d).unwrap_or_default()
            ),
            None => "NONE".to_string(),
        };

        let geom_refno_str = match &self.geom_refno {
            Some(refno) => format!("'{}'", refno),
            None => "NONE".to_string(),
        };

        format!(
            r#"CREATE geo_relate:{} SET
                in = {},
                out = {},
                trans = {},
                visible = {},
                meshed = {},
                geo_type = '{}',
                geom_refno = {};"#,
            self.id,
            self.input,
            self.out,
            trans_str,
            self.visible,
            self.meshed,
            self.geo_type,
            geom_refno_str
        )
    }

    /// 生成 SurrealDB JSON 格式数据
    pub fn gen_sur_json(&self) -> String {
        let mut json = serde_json::json!({
            "in": self.input,
            "out": self.out,
            "trans": self.trans,
            "visible": self.visible,
            "meshed": self.meshed,
            "geo_type": self.geo_type,
            "geom_refno": self.geom_refno,
        });

        // 添加 ID
        json["id"] = serde_json::json!(format!("geo_relate:{}", self.id));

        serde_json::to_string(&json).unwrap_or_default()
    }
}

/// tubi_relate 表结构体
/// 表示管道关系，连接管道的起点和终点PE元素
#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TubiRelate {
    /// 关系ID
    pub id: String,
    /// 起点PE引用 (in)
    #[serde(rename = "in")]
    pub input: RefnoEnum,
    /// 终点PE引用 (out)
    pub out: RefnoEnum,
    /// 几何引用
    pub geo: Option<String>,
    /// 管道起点
    pub start_pt: Option<Vec3>,
    /// 管道终点
    pub end_pt: Option<Vec3>,
    /// 所属系统编号
    pub system: Option<RefnoEnum>,
    /// 时间戳
    pub dt: Option<NaiveDateTime>,
}

impl TubiRelate {
    /// 创建新的 TubiRelate 实例
    pub fn new(id: String, input: RefnoEnum, out: RefnoEnum) -> Self {
        Self {
            id,
            input,
            out,
            geo: None,
            start_pt: None,
            end_pt: None,
            system: None,
            dt: None,
        }
    }

    /// 设置几何引用
    pub fn with_geo(mut self, geo: String) -> Self {
        self.geo = Some(geo);
        self
    }

    /// 设置起点和终点
    pub fn with_points(mut self, start: Vec3, end: Vec3) -> Self {
        self.start_pt = Some(start);
        self.end_pt = Some(end);
        self
    }

    /// 设置所属系统
    pub fn with_system(mut self, system: RefnoEnum) -> Self {
        self.system = Some(system);
        self
    }

    /// 设置时间戳
    pub fn with_dt(mut self, dt: NaiveDateTime) -> Self {
        self.dt = Some(dt);
        self
    }

    /// 生成 SurrealDB 插入语句并同步更新关联的PE
    pub fn to_surql(&self) -> String {
        let start_pt_str = match &self.start_pt {
            Some(pt) => format!("{{ x: {}, y: {}, z: {} }}", pt.x, pt.y, pt.z),
            None => "NONE".to_string(),
        };

        let end_pt_str = match &self.end_pt {
            Some(pt) => format!("{{ x: {}, y: {}, z: {} }}", pt.x, pt.y, pt.z),
            None => "NONE".to_string(),
        };

        let system_str = match &self.system {
            Some(system) => format!("pe:{}", system),
            None => "NONE".to_string(),
        };

        let geo_str = match &self.geo {
            Some(geo) => format!("inst_geo:{}", geo),
            None => "NONE".to_string(),
        };

        let dt_str = match &self.dt {
            Some(dt) => format!("d'{}'", dt.format("%Y-%m-%dT%H:%M:%S")),
            None => "time::now()".to_string(),
        };

        format!(
            r#"CREATE tubi_relate:{} SET
                in = pe:{},
                out = pe:{},
                geo = {},
                start_pt = {},
                end_pt = {},
                system = {},
                dt = {};
UPDATE pe:{} SET tubi_id = array::push(tubi_id?:[], tubi_relate:{});
UPDATE pe:{} SET tubi_id = array::push(tubi_id?:[], tubi_relate:{});"#,
            self.id,
            self.input,
            self.out,
            geo_str,
            start_pt_str,
            end_pt_str,
            system_str,
            dt_str,
            self.input,
            self.id,
            self.out,
            self.id
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_transform::components::Transform;
    use chrono::Utc;
    use glam::{Quat, Vec3};
    use parry3d::bounding_volume::Aabb;
    use serde_json::json;

    #[test]
    fn test_inst_relate_creation() {
        let inst_relate = InstRelate::new(
            "test_id".to_string(),
            RefnoEnum::from("12345"),
            "geo_instance_1".to_string(),
            RefnoEnum::from("67890"),
            "PIPE".to_string(),
        );

        assert_eq!(inst_relate.id, "test_id");
        assert_eq!(inst_relate.input, RefnoEnum::from("12345"));
        assert_eq!(inst_relate.out, "geo_instance_1");
        assert_eq!(inst_relate.owner, RefnoEnum::from("67890"));
        assert_eq!(inst_relate.generic, "PIPE");
    }

    #[test]
    fn test_inst_relate_with_aabb() {
        let aabb = Aabb::new(
            Vec3::new(-1.0, -1.0, -1.0).into(),
            Vec3::new(1.0, 1.0, 1.0).into(),
        );

        let inst_relate = InstRelate::new(
            "test_id".to_string(),
            RefnoEnum::from("12345"),
            "geo_instance_1".to_string(),
            RefnoEnum::from("67890"),
            "PIPE".to_string(),
        )
        .with_aabb(aabb);

        assert!(inst_relate.aabb.is_some());
    }

    #[test]
    fn test_inst_relate_to_surql() {
        let inst_relate = InstRelate::new(
            "test_id".to_string(),
            RefnoEnum::from("12345"),
            "geo_instance_1".to_string(),
            RefnoEnum::from("67890"),
            "PIPE".to_string(),
        );

        let sql = inst_relate.to_surql();
        println!("Generated SQL:\n{}", sql);
        assert!(sql.contains("CREATE inst_relate:test_id"));
        assert!(sql.contains("in = pe:"));
        assert!(sql.contains("out = geo_instance_1"));
        assert!(sql.contains("owner = pe:"));
        assert!(sql.contains("generic = 'PIPE'"));
        // 验证UPDATE语句将inst_relate_id添加到pe记录
        assert!(sql.contains("UPDATE pe:"));
        assert!(sql.contains("inst_relate_id = inst_relate:test_id"));
    }

    #[test]
    fn test_inst_relate_gen_sur_json() {
        let inst_relate = InstRelate::new(
            "test_id".to_string(),
            RefnoEnum::from("12345"),
            "geo_instance_1".to_string(),
            RefnoEnum::from("67890"),
            "PIPE".to_string(),
        );

        let json_str = inst_relate.gen_sur_json();
        println!("Generated JSON:\n{}", json_str);
        let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(json["id"], "inst_relate:test_id");
        assert!(json["in"].as_str().unwrap().contains("pe:"));
        assert_eq!(json["out"], "geo_instance_1");
        assert!(json["owner"].as_str().unwrap().contains("pe:"));
        assert_eq!(json["generic"], "PIPE");
    }

    #[test]
    fn test_inst_geo_creation() {
        let param = json!({
            "type": "cylinder",
            "radius": 1.0,
            "height": 2.0
        });

        let inst_geo = InstGeo::new(
            "geo_123".to_string(),
            param.clone(),
            true,
            true,
            "Pos".to_string(),
            true, // 单位 mesh
        );

        assert_eq!(inst_geo.id, "geo_123");
        assert_eq!(inst_geo.param, param);
        assert_eq!(inst_geo.meshed, true);
        assert_eq!(inst_geo.visible, true);
        assert_eq!(inst_geo.geo_type, "Pos");
        assert_eq!(inst_geo.unit_flag, true);
    }

    #[test]
    fn test_inst_geo_to_surql() {
        let param = json!({
            "type": "cylinder",
            "radius": 1.0,
            "height": 2.0
        });

        let inst_geo = InstGeo::new("geo_123".to_string(), param, true, true, "Pos".to_string(), true); // 单位 mesh

        let sql = inst_geo.to_surql();
        assert!(sql.contains("CREATE inst_geo:geo_123"));
        assert!(sql.contains("meshed = true"));
        assert!(sql.contains("visible = true"));
        assert!(sql.contains("geo_type = 'Pos'"));
        assert!(sql.contains("unit_flag = true"));
    }

    #[test]
    fn test_geo_relate_creation() {
        let geo_relate = GeoRelate::new(
            "relate_123".to_string(),
            "inst_456".to_string(),
            "geo_789".to_string(),
            true,
            true,
            "Pos".to_string(),
        );

        assert_eq!(geo_relate.id, "relate_123");
        assert_eq!(geo_relate.input, "inst_456");
        assert_eq!(geo_relate.out, "geo_789");
        assert_eq!(geo_relate.visible, true);
        assert_eq!(geo_relate.meshed, true);
        assert_eq!(geo_relate.geo_type, "Pos");
    }

    #[test]
    fn test_geo_relate_to_surql() {
        let geo_relate = GeoRelate::new(
            "relate_123".to_string(),
            "inst_456".to_string(),
            "geo_789".to_string(),
            true,
            true,
            "Pos".to_string(),
        );

        let sql = geo_relate.to_surql();
        assert!(sql.contains("CREATE geo_relate:relate_123"));
        assert!(sql.contains("in = inst_456"));
        assert!(sql.contains("out = geo_789"));
        assert!(sql.contains("visible = true"));
        assert!(sql.contains("meshed = true"));
        assert!(sql.contains("geo_type = 'Pos'"));
    }

    #[test]
    fn test_tubi_relate_creation() {
        let tubi_relate = TubiRelate::new(
            "tubi_123".to_string(),
            RefnoEnum::from("11111"),
            RefnoEnum::from("22222"),
        );

        assert_eq!(tubi_relate.id, "tubi_123");
        assert_eq!(tubi_relate.input, RefnoEnum::from("11111"));
        assert_eq!(tubi_relate.out, RefnoEnum::from("22222"));
    }

    #[test]
    fn test_tubi_relate_to_surql() {
        let tubi_relate = TubiRelate::new(
            "tubi_123".to_string(),
            RefnoEnum::from("11111"),
            RefnoEnum::from("22222"),
        )
        .with_points(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0))
        .with_geo("geo_hash".to_string());

        let sql = tubi_relate.to_surql();
        println!("Generated TubiRelate SQL:\n{}", sql);
        assert!(sql.contains("CREATE tubi_relate:tubi_123"));
        assert!(sql.contains("in = pe:"));
        assert!(sql.contains("out = pe:"));
        assert!(sql.contains("geo = inst_geo:geo_hash"));
        // 验证UPDATE语句将tubi_id添加到pe记录
        assert!(sql.contains("UPDATE pe:"));
        assert!(sql.contains("tubi_id = array::push(tubi_id?:[], tubi_relate:tubi_123)"));
    }
}
