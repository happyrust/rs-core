use crate::consts::*;
use crate::geometry::{EleInstGeo, EleInstGeosData};
#[cfg(feature = "sea-orm")]
use crate::orm::*;
use crate::parsed_data::CateAxisParam;
use crate::pe::SPdmsElement;
use crate::shape::pdms_shape::BrepShapeTrait;
use crate::tool::db_tool::{db1_dehash, db1_hash};
use crate::types::attmap::AttrMap;
use crate::types::attval::{AttrVal, AttrValAql};
use crate::types::named_attvalue::NamedAttrValue;
pub use crate::types::*;
#[cfg(feature = "bevy_component")]
use bevy_ecs::component::Component;
use bevy_ecs::prelude::*;
use bevy_math::*;
#[cfg(feature = "reflect")]
use bevy_reflect::Reflect;
#[cfg(feature = "render")]
use bevy_render::prelude::*;
use bevy_transform::prelude::*;
use dashmap::DashMap;
use derive_more::{Deref, DerefMut};
use id_tree::NodeId;
use itertools::Itertools;
#[cfg(feature = "occ")]
use opencascade::primitives::*;
use parry3d::bounding_volume::Aabb;
#[cfg(feature = "sea-orm")]
use sea_orm::entity::prelude::*;
#[cfg(feature = "sea-orm")]
use sea_query::*;
#[cfg(feature = "sea-orm")]
use sea_query::*;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use serde_repr::{Deserialize_repr, Serialize_repr};
use serde_with::{DisplayFromStr, serde_as};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::{Debug, Display, Pointer};
use std::io::{Read, Write};
use std::str::FromStr;
use std::string::ToString;
use surrealdb::types as surrealdb_types;
use surrealdb::types::RecordId;
use surrealdb::types::SurrealValue;

///控制pdms显示的深度层级
pub const LEVEL_VISBLE: u32 = 6;

///非负实体基本体的种类
pub const PRIMITIVE_NOUN_NAMES: [&'static str; 8] = [
    "BOX",  // 盒子
    "CYLI", // 圆柱体
    "SLCY", // 斜圆柱体
    "CONE", // 圆锥体
    "DISH", // 碟形
    "CTOR", // 圆环
    "RTOR", // 圆环
    "PYRA", // 棱锥
];

///基本体几何体相关的属性
pub const PRIMITIVE_GEO_ATTR_NAMES: [&'static str; 8] = [
    "XLEN", // X方向长度
    "YLEN", // Y方向长度
    "ZLEN", // Z方向长度
    "XRAD", // X方向半径
    "YRAD", // Y方向半径
    "ZRAD", // Z方向半径
    "XANG", // X方向角度
    "YANG", // Y方向角度
];

///基本体的种类(包含负实体)
//"SPINE", "GENS",
// 注意：NREV（负旋转体）需要 loop 数据，由 loop_model 处理，不在此列表中
pub const GNERAL_PRIM_NOUN_NAMES: [&'static str; 21] = [
    "BOX",    // 盒子
    "CYLI",   // 圆柱体
    "SLCY",   // 斜圆柱体
    "CONE",   // 圆锥体
    "DISH",   // 碟形
    "CTOR",   // 圆环
    "RTOR",   // 圆环
    "PYRA",   // 棱锥
    "SNOU",   // 球体
    "POHE",   // 多面体
    "NBOX",   // 负盒子
    "NCYL",   // 负圆柱体
    "NSBO",   // 负球体
    "NCON",   // 负圆锥体
    "NSNO",   // 负球体
    "NPYR",   // 负棱锥
    "NDIS",   // 负碟形
    "NCTO",   // 负圆环
    "NRTO",   // 负圆环
    "NSCY",   // 负斜圆柱体
    "POLYHE", // 多面体
];

///有loop的几何体
pub const GNERAL_LOOP_OWNER_NOUN_NAMES: [&'static str; 9] = [
    "AEXTR",  // 轴向拉伸
    "NXTR",   // 负拉伸
    "EXTR",   // 拉伸
    "PANE",   // 面板
    "FLOOR",  // 地板
    "SCREED", // 找平层
    "GWALL",  // 玻璃墙/通用墙
    "NREV",   // 负旋转体
    "REVO",   // 旋转体
];

///使用元件库的实体类型名称
pub const USE_CATE_NOUN_NAMES: [&'static str; 32] = [
    "FIXING", // 固定件
    "GENSEC", // 通用截面
    "SCREED", // 找平层
    "CMPF",   // 复合件
    "GWALL",  // 玻璃墙/通用墙
    "EQUI",   // 设备
    "ANCI",   // 锚固件
    "FITT",   // 管件
    "SJOI",   // 结构连接
    "SBFI",   // 结构基座固定
    "CABLE",  // 电缆
    "CNODE",  // 连接节点
    "SCTN",   // 截面
    "SCOJ",   // 结构连接
    // "PAVE",   // 铺装
    "SUBE",   // 子元素
    "SEVE",   // 服务
    "SUBJ",   // 子连接
    "PLOO",   // 管道循环
    "RNODE",  // 节点
    "PJOI",   // 管道连接
    "SELJ",   // 选择连接
    "STWALL", // 结构墙
    "WALL",   // 墙
    "PALJ",   // 管道对齐
    // "TUBI",   // 管道 - 注释掉，TUBI 数据通过 tubi_relate 关系获取，不是独立表
    "FLOOR",  // 地板
    "CMFI",   // 复合安装
    // "PANE",   // 面板
    "PFIT",   // 管道配件
    "GPART",  // 通用零件
    "PRTELE", // 主要元素
    "NOZZ",   // 接管
    "SPCO",   // 规范组件
    "ELCONN", // 电气连接
];

///使用元件库的几何体相关的属性, todo 继续完善
pub const CATA_GEO_ATTR_NAMES: [&'static str; 4] = [
    "SPRE", // 规范引用
    "CATR", // 元件库引用
    "ZDIS", // Z方向偏移
    "DESP", // 描述
];

///方位的相关属性, todo 继续完善
pub const TRANSFORM_ATTR_NAMES: [&'static str; 4] = [
    "POS",  // 位置
    "ORI",  // 方向
    "POSS", // 位置集合
    "POSE", // 位置元素
];

///管道的类型
pub const PIPING_NOUN_NAMES: [&'static str; 26] = [
    "WELD", // 焊缝
    "ELBO", // 弯头
    "VALV", // 阀门
    "FLAN", // 法兰
    "GASK", // 垫片
    "ATTA", // 附件
    "OLET", // 支管
    "FBLI", // 法兰盲板
    "REDU", // 异径管
    "TEE",  // 三通
    "BEND", // 弯管
    "INST", // 仪表
    "TRNS", // 过渡件
    "DAMP", // 阻尼器
    "STRT", // 直管段
    "TAPE", // 胶带
    "THRE", // 螺纹
    "UNIO", // 接头
    "BRCO", // 分支连接
    "OFST", // 偏移
    "CAP",  // 封头
    "PCOM", // 管道组件
    "FTUB", // 管件
    "STIF", // 加强环
    "SILE", // 静音器
    "COUP", // 联轴器
];

///负实体基本体的种类
pub const GENRAL_NEG_NOUN_NAMES: [&'static str; 13] = [
    "NBOX", // 负盒子
    "NCYL", // 负圆柱体
    "NLCY", // 负斜圆柱体
    "NSBO", // 负球体
    "NCON", // 负圆锥体
    "NSNO", // 负球体
    "NPYR", // 负棱锥
    "NDIS", // 负碟形
    "NXTR", // 负拉伸
    "NCTO", // 负圆环
    "NRTO", // 负圆环
    "NREV", // 负旋转体
    "NSCY", // 负斜圆柱体
];

///元件库的负实体类型
pub const CATE_NEG_NOUN_NAMES: [&'static str; 13] = [
    "NSBO", // 负球体
    "NSCO", // 负圆锥体
    "NLSN", // 负长球体
    "NSSP", // 负球体
    "NLCY", // 负斜圆柱体
    "NSCY", // 负斜圆柱体
    "NSCT", // 负圆锥体
    "NSRT", // 负圆环
    "NSDS", // 负碟形
    "NSSL", // 负球体
    "NLPY", // 负棱锥
    "NSEX", // 负拉伸
    "NSRE", // 负旋转体
];

///所有负实体类型的名称（包括基本体和元件库的负实体）
pub const TOTAL_NEG_NOUN_NAMES: [&'static str; 26] = [
    "NBOX", // 负盒子（基本体）
    "NCYL", // 负圆柱体（基本体）
    "NLCY", // 负斜圆柱体（基本体）
    "NSBO", // 负球体（基本体）
    "NCON", // 负圆锥体（基本体）
    "NSNO", // 负球体（基本体）
    "NPYR", // 负棱锥（基本体）
    "NDIS", // 负碟形（基本体）
    "NXTR", // 负拉伸（基本体）
    "NCTO", // 负圆环（基本体）
    "NRTO", // 负圆环（基本体）
    "NREV", // 负旋转体（基本体）
    "NSCY", // 负斜圆柱体（基本体）
    "NSBO", // 负球体（元件库）
    "NSCO", // 负圆锥体（元件库）
    "NLSN", // 负长球体（元件库）
    "NSSP", // 负球体（元件库）
    "NLCY", // 负斜圆柱体（元件库）
    "NSCY", // 负斜圆柱体（元件库）
    "NSCT", // 负圆锥体（元件库）
    "NSRT", // 负圆环（元件库）
    "NSDS", // 负碟形（元件库）
    "NSSL", // 负球体（元件库）
    "NLPY", // 负棱锥（元件库）
    "NSEX", // 负拉伸（元件库）
    "NSRE", // 负旋转体（元件库）
];

///顶点相关的实体类型名称
pub const TOTAL_VERT_NOUN_NAMES: [&'static str; 2] = [
    "VERT", // 顶点
    "PAVE", // 铺装
];

///循环相关的实体类型名称
pub const TOTAL_LOOP_NOUN_NAMES: [&'static str; 2] = [
    "LOOP", // 循环
    "PLOO", // 管道循环
];

///连接点类型
pub const JOINT_TYPES: [&'static str; 2] = [
    "SJOI", // 结构连接
    "PJOI", // 管道连接
];

///正实体基本体的种类
pub const GENRAL_POS_NOUN_NAMES: [&'static str; 25] = [
    "BOX",   // 盒子
    "CYLI",  // 圆柱体
    "SLCY",  // 斜圆柱体
    "CONE",  // 圆锥体
    "DISH",  // 碟形
    "CTOR",  // 圆环
    "RTOR",  // 圆环
    "PYRA",  // 棱锥
    "SNOU",  // 球体
    "FLOOR", // 地板
    "PANEL", // 面板
    "SBOX",  // 元件库盒子
    "SCYL",  // 元件库圆柱体
    "LCYL",  // 长圆柱体
    "SSPH",  // 元件库球体
    "LCYL",  // 长圆柱体（重复）
    "SCON",  // 元件库圆锥体
    "LSNO",  // 长球体
    "LPYR",  // 长棱锥
    "SDSH",  // 元件库碟形
    "SCTO",  // 元件库圆环
    "SEXT",  // 元件库拉伸
    "SREV",  // 元件库旋转体
    "SRTO",  // 元件库圆环
    "SSLC",  // 元件库斜圆柱体
];

///所有几何体类型的名称（包括正实体和负实体）
pub const TOTAL_GEO_NOUN_NAMES: [&'static str; 40] = [
    "BOX",  // 盒子
    "CYLI", // 圆柱体
    "SLCY", // 斜圆柱体
    "CONE", // 圆锥体
    "DISH", // 碟形
    "CTOR", // 圆环
    "RTOR", // 圆环
    "PYRA", // 棱锥
    "SNOU", // 球体
    "PLOO", // 管道循环
    "LOOP", // 循环
    "POHE", // 多面体
    "SBOX", // 元件库盒子
    "SCYL", // 元件库圆柱体
    "SSPH", // 元件库球体
    "LCYL", // 长圆柱体
    "SCON", // 元件库圆锥体
    "LSNO", // 长球体
    "LPYR", // 长棱锥
    "SDSH", // 元件库碟形
    "SCTO", // 元件库圆环
    "SEXT", // 元件库拉伸
    "SREV", // 元件库旋转体
    "SRTO", // 元件库圆环
    "SSLC", // 元件库斜圆柱体
    "SPRO", // 元件库投影
    "SREC", // 元件库矩形
    "NBOX", // 负盒子
    "NCYL", // 负圆柱体
    "NLCY", // 负斜圆柱体
    "NSBO", // 负球体
    "NCON", // 负圆锥体
    "NSNO", // 负球体
    "NPYR", // 负棱锥
    "NDIS", // 负碟形
    "NXTR", // 负拉伸
    "NCTO", // 负圆环
    "NRTO", // 负圆环
    "NREV", // 负旋转体
    "NSCY", // 负斜圆柱体
];

///所有元件库几何体类型的名称（包括正实体和负实体）
pub const TOTAL_CATA_GEO_NOUN_NAMES: [&'static str; 31] = [
    "SBOX", // 元件库盒子
    "SCYL", // 元件库圆柱体
    "SSPH", // 元件库球体
    "LCYL", // 长圆柱体
    "SCON", // 元件库圆锥体
    "LSNO", // 长球体
    "LPYR", // 长棱锥
    "SDSH", // 元件库碟形
    "SCTO", // 元件库圆环
    "SEXT", // 元件库拉伸
    "SREV", // 元件库旋转体
    "SRTO", // 元件库圆环
    "SSLC", // 元件库斜圆柱体
    "SPRO", // 元件库投影
    "SANN", // 元件库环
    "BOXI", // 盒子实例
    "TUBE", // 管
    "SREC", // 元件库矩形
    "NSBO", // 负球体
    "NSCO", // 负圆锥体
    "NLSN", // 负长球体
    "NSSP", // 负球体
    "NLCY", // 负斜圆柱体
    "NSCY", // 负斜圆柱体
    "NSCT", // 负圆锥体
    "NSRT", // 负圆环
    "NSDS", // 负碟形
    "NSSL", // 负球体
    "NLPY", // 负棱锥
    "NSEX", // 负拉伸
    "NSRE", // 负旋转体
];

///可能会与ngmr发生作用的类型
pub const TOTAL_CONTAIN_NGMR_GEO_NAEMS: [&'static str; 6] = [
    "WALL",   // 墙
    "STWALL", // 结构墙
    "GWALL",  // 玻璃墙/通用墙
    "SCTN",   // 截面
    "PANEL",  // 面板
    "FLOOR",  // 地板
];

///多面体（Polyhedron）类型
pub const POHE_GEO_NAMES: [&'static str; 1] = ["POHE"];

///元件库的种类
pub const CATA_GEO_NAMES: [&'static str; 26] = [
    "BRAN",   // 分支
    "HANG",   // 悬挂
    "ELCONN", // 电气连接
    "CMPF",   // 复合件
    "WALL",   // 墙
    "STWALL", // 结构墙
    "GWALL",  // 玻璃墙/通用墙
    "FIXING", // 固定件
    "SJOI",   // 结构连接
    "PJOI",   // 管道连接
    "PFIT",   // 管道配件
    "GENSEC", // 通用截面
    "RNODE",  // 节点
    "PRTELE", // 主要元素
    "GPART",  // 通用零件
    "SCREED", // 找平层
    "NOZZ",   // 接管
    "PALJ",   // 管道对齐
    "CABLE",  // 电缆
    "BATT",   // 护板/电池
    "CMFI",   // 复合安装
    "SCOJ",   // 结构连接
    "SEVE",   // 服务
    "SBFI",   // 结构基座固定
    "SCTN",   // 截面
    "FITT",   // 管件
];

///有tubi的类型
pub const CATA_HAS_TUBI_GEO_NAMES: [&'static str; 2] = [
    "BRAN", // 分支
    "HANG", // 悬挂
];

///可以重用的类型
pub const CATA_SINGLE_REUSE_GEO_NAMES: [&'static str; 0] = [];

///不可重用的元件库几何体类型名称
pub const CATA_WITHOUT_REUSE_GEO_NAMES: [&'static str; 23] = [
    "ELCONN", // 电气连接
    "CMPF",   // 复合件
    "WALL",   // 墙
    "GWALL",  // 玻璃墙/通用墙
    "SJOI",   // 结构连接
    "FITT",   // 管件
    "PFIT",   // 管道配件
    "FIXING", // 固定件
    "PJOI",   // 管道连接
    "GENSEC", // 通用截面
    "RNODE",  // 节点
    "PRTELE", // 主要元素
    "GPART",  // 通用零件
    "SCREED", // 找平层
    "PALJ",   // 管道对齐
    "CABLE",  // 电缆
    "BATT",   // 护板/电池
    "CMFI",   // 复合安装
    "SCOJ",   // 结构连接
    "SEVE",   // 服务
    "SBFI",   // 结构基座固定
    "STWALL", // 结构墙
    "SCTN",   // 截面
];

///可见的几何体类型名称
pub const VISBILE_GEO_NOUNS: [&'static str; 39] = [
    "BOX",    // 盒子
    "CYLI",   // 圆柱体
    "SLCY",   // 斜圆柱体
    "CONE",   // 圆锥体
    "DISH",   // 碟形
    "CTOR",   // 圆环
    "RTOR",   // 圆环
    "PYRA",   // 棱锥
    "SNOU",   // 球体
    "POHE",   // 多面体
    "POLYHE", // 多面体
    "EXTR",   // 拉伸
    "REVO",   // 旋转体
    "FLOOR",  // 地板
    "PANE",   // 面板
    "ELCONN", // 电气连接
    "CMPF",   // 复合件
    "WALL",   // 墙
    "GWALL",  // 玻璃墙/通用墙
    "SJOI",   // 结构连接
    "FITT",   // 管件
    "PFIT",   // 管道配件
    "FIXING", // 固定件
    "PJOI",   // 管道连接
    "GENSEC", // 通用截面
    "RNODE",  // 节点
    "PRTELE", // 主要元素
    "GPART",  // 通用零件
    "SCREED", // 找平层
    "PALJ",   // 管道对齐
    "CABLE",  // 电缆
    "BATT",   // 护板/电池
    "CMFI",   // 复合安装
    "SCOJ",   // 结构连接
    "SEVE",   // 服务
    "SBFI",   // 结构基座固定
    "STWALL", // 结构墙
    "SCTN",   // 截面
    "NOZZ",   // 接管
];

///站点规格值枚举
#[derive(Serialize, Deserialize, Clone, Debug, Default, Copy, Eq, PartialEq, Hash)]
pub enum SiteSpecValue {
    #[default]
    Unknown = 0,  // 未知或其他
    Pipe = 1,     // 管道系统
    Elec = 2,     // 电气系统
    Inst = 3,     // 仪表系统
    Hvac = 4,     // 暖通空调系统
}

impl SiteSpecValue {
    /// 从站点名称解析规格值
    pub fn from_site_name(name: &str) -> Self {
        if name.to_uppercase().contains("PIPE") {
            SiteSpecValue::Pipe
        } else if name.to_uppercase().contains("ELEC") {
            SiteSpecValue::Elec
        } else if name.to_uppercase().contains("INST") {
            SiteSpecValue::Inst
        } else if name.to_uppercase().contains("HVAC") {
            SiteSpecValue::Hvac
        } else {
            SiteSpecValue::Unknown
        }
    }
    
    /// 转换为 i64 值（用于存储到数据库）
    pub fn to_i64(self) -> i64 {
        self as i64
    }
    
    /// 从 i64 值创建枚举
    pub fn from_i64(value: i64) -> Self {
        match value {
            0 => SiteSpecValue::Unknown,
            1 => SiteSpecValue::Pipe,
            2 => SiteSpecValue::Elec,
            3 => SiteSpecValue::Inst,
            4 => SiteSpecValue::Hvac,
            _ => SiteSpecValue::Unknown,
        }
    }
    
    /// 获取描述文本
    pub fn description(self) -> &'static str {
        match self {
            SiteSpecValue::Unknown => "未知或其他",
            SiteSpecValue::Pipe => "管道系统",
            SiteSpecValue::Elec => "电气系统",
            SiteSpecValue::Inst => "仪表系统",
            SiteSpecValue::Hvac => "暖通空调系统",
        }
    }
}

///连接类型枚举
#[derive(Serialize, Deserialize, Clone, Debug, Default, Copy, Eq, PartialEq, Hash)]
pub enum SjusType {
    #[default]
    UNSET, // 未设置
    UTOP, // 顶部连接
    UBOT, // 底部连接
    UCEN, // 中心连接
}

///JSGF结构体（用于序列化/反序列化）
#[derive(Serialize, Deserialize, Debug)]
struct Jsgf {
    #[serde(with = "string")]
    u: u64, // 无符号整数
    #[serde(with = "string")]
    i: i64, // 有符号整数
}

///字符串序列化/反序列化模块
pub mod string {
    use std::fmt::Display;
    use std::str::FromStr;

    use serde::{Deserialize, Deserializer, Serializer, de};

    ///将值序列化为字符串
    pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: Display,
        S: Serializer,
    {
        serializer.collect_str(value)
    }

    ///从字符串反序列化值
    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where
        T: FromStr,
        T::Err: Display,
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(de::Error::custom)
    }
}

///属性值差异结构体，用于记录属性值的变化
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifferenceValue {
    ///属性名称
    pub noun: String,
    ///旧值，新增时old_value为None
    pub old_value: Option<NamedAttrValue>,
    ///新值，删除时new_value为None
    pub new_value: Option<NamedAttrValue>,
}

///默认的属性名称哈希值数组
pub const DEFAULT_NOUNS: [NounHash; 5] = [TYPE_HASH, NAME_HASH, REFNO_HASH, OWNER_HASH, TAG_NAME_HASH];
///默认的属性名称数组
pub const DEFAULT_NAMED_NOUNS: [&'static str; 5] = ["TYPE", "NAME", "REFNO", "OWNER", "TAG_NAME"];

///PDMS通用类型枚举
#[repr(C)]
#[derive(
    Component,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
    Serialize,
    Deserialize,
    Default,
    Clone,
    strum_macros::Display,
    strum_macros::EnumString,
    Debug,
    Copy,
    Eq,
    PartialEq,
    Hash,
)]
pub enum PdmsGenericType {
    #[default]
    UNKOWN = 0, // 未知类型
    CE,      // 设备
    PIPE,    // 管道
    STRU,    // 结构
    EQUI,    // 设备
    ROOM,    // 房间
    SCTN,    // 截面
    WALL,    // 墙
    STWALL,  // 结构墙
    CWALL,   // 混凝土墙
    GWALL,   // 玻璃墙/通用墙
    GENSEC,  // 通用截面
    HANG,    // 悬挂
    HANDRA,  // 扶手
    PANE,    // 面板
    CFLOOR,  // 混凝土地板
    FLOOR,   // 地板
    EXTR,    // 拉伸
    CWBRAN,  // 冷弯分支
    REVO,    // 旋转体
    CTWALL,  // 混凝土墙
    AREADEF, // 区域定义
    DEMOPA,  // 演示面板
    INSURQ,  // 安装请求
    STRLNG,  // 结构长度
    HVAC,    // 暖通空调
}

///从字符串反序列化为u64
fn de_from_str<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    s.parse::<u64>().map_err(de::Error::custom)
}

///从字符串反序列化为RefU64
fn de_refno_from_str<'de, D>(deserializer: D) -> Result<RefU64, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    RefU64::from_str(&s).map_err(de::Error::custom)
}

///从字符串反序列化为RefU64的HashSet
fn de_hashset_from_str<'de, D>(deserializer: D) -> Result<HashSet<RefU64>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = String::deserialize(deserializer).unwrap_or_default();
    Ok(serde_json::from_str::<HashSet<String>>(s.as_str())
        .unwrap_or_default()
        .into_iter()
        .map(|x| RefU64::from_str(x.as_str()).unwrap_or_default())
        .collect())
}

///将RefU64的HashSet序列化为JSON字符串
pub fn ser_hashset_as_str<S>(refnos: &HashSet<RefU64>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let set = refnos
        .into_iter()
        .map(|x| x.to_string())
        .collect::<HashSet<_>>();
    s.serialize_str(serde_json::to_string(&set).unwrap_or_default().as_str())
    // s.ser(&set)
}

///将u64序列化为字符串
pub fn ser_u64_as_str<S>(id: &u64, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str((*id).to_string().as_str())
}

///将RefU64序列化为字符串
pub fn ser_refno_as_str<S>(refno: &RefU64, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(refno.to_string().as_str())
}

///PDMS节点特征trait，定义了PDMS节点应具备的基本功能
pub trait PdmsNodeTrait: Default {
    ///获取节点的参考号
    #[inline]
    fn get_refno(&self) -> RefU64 {
        RefU64::default()
    }

    ///获取节点的记录ID
    #[inline]
    fn get_id(&self) -> Option<&RecordId> {
        None
    }

    ///获取节点的名称
    #[inline]
    fn get_name(&self) -> &str {
        ""
    }

    ///获取节点的修改次数
    #[inline]
    fn get_mod_cnt(&self) -> u32 {
        0
    }

    ///获取节点的状态
    #[inline]
    fn get_status(&self) -> &str {
        ""
    }

    ///获取节点类型的哈希值
    #[inline]
    fn get_noun_hash(&self) -> u32 {
        0
    }

    ///获取节点类型名称
    #[inline]
    fn get_type_name(&self) -> &str {
        ""
    }

    ///获取子节点数量
    #[inline]
    fn get_children_count(&self) -> usize {
        0
    }

    ///获取节点的顺序
    #[inline]
    fn get_order(&self) -> usize {
        0
    }
}

///初始状态码
pub const STATE_CODE_INIT: &'static str = "D00";

///元素树节点结构体，表示PDMS中的树形结构节点
#[derive(Serialize, Deserialize, Clone, Debug, Default, SurrealValue)]
pub struct EleTreeNode {
    ///节点参考号
    pub refno: RefnoEnum,
    ///节点类型名称
    pub noun: String,
    ///节点名称
    #[serde(default)]
    pub name: String,
    ///父节点参考号
    pub owner: RefnoEnum,
    ///节点顺序
    #[serde(default)]
    #[surreal(value = 0)]
    pub order: u16,
    ///子节点数量
    pub children_count: u16,
    // #[serde(default)]
    // pub op: EleOperation,
    ///修改次数
    pub mod_cnt: Option<u32>,
    ///子节点是否已更新
    #[serde(default)]
    pub children_updated: Option<bool>,
    ///状态码
    pub status_code: Option<String>,
}

impl EleTreeNode {
    ///创建新的元素树节点
    pub fn new(
        refno: RefnoEnum,
        noun: String,
        name: String,
        owner: RefnoEnum,
        order: u16,
        children_count: u16,
        op: EleOperation,
    ) -> Self {
        Self {
            refno,
            noun,
            name,
            owner,
            order,
            children_count,
            // op,
            mod_cnt: None,
            children_updated: None,
            status_code: None,
        }
    }

    ///转换为对外接口的结构体
    pub fn into_handle_struct(self) -> PdmsElementHandle {
        PdmsElementHandle {
            refno: self.refno.to_pdms_str(),
            owner: self.owner.to_pdms_str(),
            name: self.name,
            noun: self.noun,
            version: 0,
            children_count: self.children_count as _,
        }
    }

    ///获取最新的参考号
    #[inline]
    pub fn latest_refno(&self) -> RefU64 {
        self.refno.refno()
    }

    ///获取最新的父节点参考号
    #[inline]
    pub fn latest_owner(&self) -> RefU64 {
        self.owner.refno()
    }
}

impl From<PdmsElement> for EleTreeNode {
    fn from(value: PdmsElement) -> Self {
        EleTreeNode {
            refno: value.refno.into(),
            noun: value.noun,
            name: value.name,
            owner: value.owner.into(),
            order: 0,
            children_count: value.children_count as _,
            // op: EleOperation::Modified,
            mod_cnt: None,
            children_updated: None,
            status_code: None,
        }
    }
}

impl PdmsNodeTrait for EleTreeNode {
    #[inline]
    fn get_refno(&self) -> RefU64 {
        self.refno.refno()
    }

    #[inline]
    fn get_name(&self) -> &str {
        self.name.as_str()
    }

    #[inline]
    fn get_mod_cnt(&self) -> u32 {
        self.mod_cnt.unwrap_or_default()
    }

    #[inline]
    fn get_status(&self) -> &str {
        self.status_code.as_deref().unwrap_or_default()
    }

    #[inline]
    fn get_noun_hash(&self) -> u32 {
        db1_hash(&self.noun.to_uppercase())
    }

    #[inline]
    fn get_type_name(&self) -> &str {
        self.noun.as_str()
    }

    #[inline]
    fn get_children_count(&self) -> usize {
        self.children_count as _
    }

    #[inline]
    fn get_order(&self) -> usize {
        self.order as _
    }
}

///元素节点结构体，用于表示PDMS元素的简化信息
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct EleNode {
    ///参考号
    pub refno: RefU64,
    ///父节点参考号
    pub owner: RefU64,
    ///名称哈希值
    pub name_hash: AiosStrHash,
    // pub name: AiosStr,
    ///类型哈希值
    pub noun: u32,
    ///版本号
    pub version: u32,
    // pub children_count: usize,
    ///子节点数量
    pub children_count: usize,
}

///子节点结构体，用于表示子节点的基本信息
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ChildrenNode {
    ///参考号
    pub refno: RefU64,
    ///名称
    pub name: String,
    ///类型名称
    pub noun: String,
}

///元件库哈希值与参考号的键值对结构体
#[serde_as]
#[derive(Serialize, Deserialize, Clone, Debug, Default, Component)]
pub struct CataHashRefnoKV {
    ///元件库哈希值
    #[serde(default)]
    pub cata_hash: String,
    ///分组参考号列表
    #[serde(default)]
    pub group_refnos: Vec<RefnoEnum>,
    ///是否存在实例
    pub exist_inst: bool,
    ///参数点集合
    pub ptset: Option<BTreeMap<i32, CateAxisParam>>,
}

///PDMS元素结构体，表示PDMS数据库中的基本元素
#[serde_as]
#[derive(Serialize, Deserialize, Clone, Debug, Default, Eq, PartialEq, Component, SurrealValue)]
pub struct PdmsElement {
    ///参考号（主键）
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "_key")]
    pub refno: RefU64,
    ///父节点参考号
    #[serde_as(as = "DisplayFromStr")]
    pub owner: RefU64,
    ///元素名称
    pub name: String,
    ///元素类型名称
    pub noun: String,
    ///版本号
    #[serde(default)]
    pub version: u32,
    ///子节点数量
    #[serde(default)]
    pub children_count: usize,
}

impl PdmsNodeTrait for PdmsElement {
    #[inline]
    fn get_refno(&self) -> RefU64 {
        self.refno
    }

    #[inline]
    fn get_name(&self) -> &str {
        self.name.as_str()
    }

    #[inline]
    fn get_noun_hash(&self) -> u32 {
        db1_hash(&self.noun.to_uppercase())
    }

    #[inline]
    fn get_type_name(&self) -> &str {
        self.noun.as_str()
    }

    #[inline]
    fn get_children_count(&self) -> usize {
        self.children_count
    }
}

impl PdmsElement {
    ///获取Enso表格的表头
    pub fn get_enso_headers() -> Vec<String> {
        vec![
            "refno".to_string(),
            "owner".to_string(),
            "name".to_string(),
            "noun".to_string(),
            "version".to_string(),
            "children_count".to_string(),
        ]
    }

    ///转换为Enso值JSON格式
    pub fn into_enso_value_json(self) -> Vec<NamedAttrValue> {
        vec![
            NamedAttrValue::StringType(self.refno.to_string()),
            NamedAttrValue::StringType(self.owner.to_string()),
            NamedAttrValue::StringType(self.name),
            NamedAttrValue::StringType(self.noun),
            NamedAttrValue::IntegerType(self.version as i32),
            NamedAttrValue::IntegerType(self.children_count as i32),
        ]
    }
    ///转换为对外接口的结构体
    pub fn into_handle_struct(self) -> PdmsElementHandle {
        PdmsElementHandle {
            refno: self.refno.to_pdms_str(),
            owner: self.owner.to_pdms_str(),
            name: self.name,
            noun: self.noun,
            version: self.version,
            children_count: self.children_count,
        }
    }
}

impl From<EleTreeNode> for PdmsElement {
    fn from(value: EleTreeNode) -> Self {
        Self {
            refno: value.refno.refno(),
            owner: value.owner.refno(),
            name: value.name,
            noun: value.noun,
            version: 0,
            children_count: value.children_count as usize,
        }
    }
}

impl From<SPdmsElement> for PdmsElement {
    fn from(value: SPdmsElement) -> Self {
        Self {
            refno: value.refno.refno(),
            owner: value.owner.refno(),
            name: value.name,
            noun: value.noun,
            version: 0,
            children_count: 0,
        }
    }
}

///PDMS元素向量包装结构体
#[derive(Serialize, Deserialize, Clone, Debug, Default, Deref, DerefMut)]
pub struct PdmsElementVec(pub Vec<PdmsElement>);

impl EleNode {
    ///设置默认名称哈希值
    pub fn set_default_name(name_hash: AiosStrHash) -> EleNode {
        EleNode {
            name_hash,
            ..Default::default()
        }
    }
}

///PDMS节点ID包装结构体
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PdmsNodeId(pub NodeId);

///数据库编号与版本对应关系结构体
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct DbnoVersion {
    ///数据库编号
    pub dbno: u32,
    ///版本号
    pub version: u32,
}

///PDMS元素句柄结构体，用于对外接口
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PdmsElementHandle {
    ///参考号（字符串格式）
    pub refno: String,
    ///父节点参考号（字符串格式）
    pub owner: String,
    ///元素名称
    pub name: String,
    ///元素类型名称
    pub noun: String,
    ///版本号
    #[serde(default)]
    pub version: u32,
    ///子节点数量
    #[serde(default)]
    pub children_count: usize,
}

#[test]
fn test_dashmap() {
    let dashmap_1 = DashMap::new();
    dashmap_1.insert("1", "hello");
    let dashmap_2 = DashMap::new();
    dashmap_2.insert("2", "world");
    let dashmap_3 = DashMap::new();
    dashmap_1.iter().for_each(|m| {
        dashmap_3.insert(m.key().clone(), m.value().clone());
    });
    dashmap_2.iter().for_each(|m| {
        dashmap_3.insert(m.key().clone(), m.value().clone());
    });
}

///数据库属性类型枚举
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DbAttributeType {
    #[default]
    Unknown, // 未知类型
    INTEGER = 1, // 整数
    DOUBLE,      // 双精度浮点数
    BOOL,        // 布尔值
    STRING,      // 字符串
    ELEMENT,     // 元素引用
    WORD,        // 字
    DIRECTION,   // 方向
    POSITION,    // 位置
    ORIENTATION, // 方向
    DATETIME,    // 日期时间

    //todo remove these
    DOUBLEVEC, // 双精度向量
    INTVEC,    // 整数向量
    FLOATVEC,  // 浮点向量
    TYPEX,     // 类型X
    Vec3Type,  // 三维向量类型
    RefU64Vec, // RefU64向量
}

///属性信息结构体
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AttrInfo {
    ///属性名称
    pub name: String,
    ///属性哈希值
    pub hash: i32,
    ///属性偏移量
    pub offset: u32,
    ///默认值
    pub default_val: AttrVal,
    ///属性类型
    pub att_type: DbAttributeType,
}

impl AttrInfo {}

///PDMS数据库信息结构体
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PDMSDBInfo {
    ///数据库名称
    pub name: String,
    ///数据库编号
    pub db_no: i32,
    ///数据库类型
    pub db_type: String,
    ///版本号
    pub version: u32,
}

///PDMS参考号结构体
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PdmsRefno {
    ///参考号
    pub ref_no: String,
    ///数据库
    pub db: String,
    ///类型名称
    pub type_name: String,
}

///Aios字符串哈希值类型别名
pub type AiosStrHash = u32;

///Aios字符串包装结构体
#[derive(
    Debug,
    Clone,
    Default,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    Hash,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub struct AiosStr(pub String);

impl AiosStr {
    ///获取u32哈希值
    #[inline]
    pub fn get_u32_hash(&self) -> u32 {
        use hash32::{FnvHasher, Hasher};
        use std::hash::Hash;
        let mut fnv = FnvHasher::default();
        self.hash(&mut fnv);
        fnv.finish32()
    }
    ///消费自身，返回内部字符串
    pub fn take(self) -> String {
        self.0
    }

    ///获取字符串切片
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Deref for AiosStr {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

///参考号与节点ID关联结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefnoNodeId {
    ///参考号
    pub refno: u64,
    ///参考号对应的小版本
    pub version: u32,
    ///参考号在树中对应的nodeId
    pub node_id: NodeId,
}

///项目数据库编号结构体
#[derive(Serialize, Deserialize, Clone, Debug, Default, Component)]
pub struct ProjectDbno {
    ///主数据库编号
    pub mdb: u32,
    ///主数据库编号
    pub main_db: u32,
    ///每个模块（DESI,CATA等）对应的数据库编号列表
    pub dbs: HashMap<String, Vec<u32>>,
}

///TxXt结构体（用于存储键值对映射）
#[derive(Debug, Serialize, Deserialize)]
pub struct TxXt {
    ///映射表
    pub map: HashMap<String, String>,
}

///YkGd结构体（用于存储键值对映射）
#[derive(Debug, Serialize, Deserialize)]
pub struct YkGd {
    ///映射表
    pub map: HashMap<String, String>,
}

///每种类型对应的所有UDA名称和默认值
#[derive(Debug, Serialize, Deserialize)]
pub struct Uda {
    ///引用类型
    pub reference_type: String,
    ///数据列表（名称，默认值）
    pub data: Vec<(String, String)>,
}

///数据状态对应的数据结构
#[derive(Default, Clone, Debug, Serialize, Deserialize, Component)]
pub struct DataState {
    ///参考号
    pub refno: RefU64,
    ///属性类型
    pub att_type: String,
    ///名称
    pub name: String,
    ///状态
    pub state: String,
}

///数据状态向量结构体
#[derive(Default, Clone, Debug, Serialize, Deserialize, Component)]
pub struct DataStateVec {
    ///数据状态列表
    pub data_states: Vec<DataState>,
}

///数据状态需要显示的PDMS属性
#[derive(Default, Clone, Debug, Serialize, Deserialize, Component)]
pub struct DataScope {
    ///参考号
    pub refno: RefU64,
    ///属性类型
    pub att_type: String,
    ///名称
    pub name: String,
}

///数据范围向量结构体
#[derive(Default, Clone, Debug, Serialize, Deserialize, Component)]
pub struct DataScopeVec {
    ///数据范围列表
    pub data_scopes: Vec<DataScope>,
}

unsafe impl Send for DataScopeVec {}

unsafe impl Sync for DataScopeVec {}

///增量数据SQL结构体，用于记录数据的增量变化
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct IncrementDataSql {
    ///记录ID
    pub id: String,
    ///参考号
    pub refno: RefU64,
    ///操作类型
    pub operate: EleOperation,
    ///版本号
    pub version: u32,
    ///用户
    pub user: String,
    ///旧数据
    pub old_data: AttrMap,
    ///新数据
    pub new_data: AttrMap,
    ///时间戳
    pub time: String,
}

///UDA专业类型枚举，表示PDMS中UDA（用户定义属性）的专业分类
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum UdaMajorType {
    /// 工艺
    T,
    /// 通风
    V,
    /// 电气
    E,
    /// 仪控
    I,
    /// 核岛水工
    W,
    /// BOP-暖通
    N,
    /// BOP-水工
    Z,
    /// 通信
    K,
    /// 设备
    S,
    /// 照明
    L,
    /// 辐射安全
    F,
    /// 反应堆热工水力
    H,
    /// 辐射监测
    R,
    /// 建筑
    A,
    /// 结构
    J,
    /// NPIC管道
    P,
    /// NPIC设备
    B,
    /// NPIC电气
    C,
    /// NPIC仪表
    Y,
    /// 多专业
    X,

    /// 未知/空值
    NULL,
}

impl UdaMajorType {
    ///从字符串创建UDA专业类型
    pub fn from_str(input: &str) -> Self {
        match input.to_uppercase().as_str() {
            "T" => Self::T,
            "V" => Self::V,
            "E" => Self::E,
            "I" => Self::I,
            "W" => Self::W,
            "N" => Self::N,
            "Z" => Self::Z,
            "K" => Self::K,
            "S" => Self::S,
            _ => Self::NULL,
        }
    }

    ///转换为专业类型字符串
    pub fn to_major_str(&self) -> String {
        match self {
            UdaMajorType::T => "T".to_string(),
            UdaMajorType::V => "V".to_string(),
            UdaMajorType::E => "E".to_string(),
            UdaMajorType::I => "I".to_string(),
            UdaMajorType::W => "W".to_string(),
            UdaMajorType::N => "N".to_string(),
            UdaMajorType::Z => "Z".to_string(),
            UdaMajorType::K => "K".to_string(),
            UdaMajorType::S => "S".to_string(),
            UdaMajorType::L => "L".to_string(),
            UdaMajorType::F => "F".to_string(),
            UdaMajorType::H => "H".to_string(),
            UdaMajorType::R => "R".to_string(),
            UdaMajorType::A => "A".to_string(),
            UdaMajorType::J => "J".to_string(),
            UdaMajorType::P => "P".to_string(),
            UdaMajorType::B => "B".to_string(),
            UdaMajorType::C => "C".to_string(),
            UdaMajorType::Y => "Y".to_string(),
            UdaMajorType::X => "X".to_string(),
            UdaMajorType::NULL => "NULL".to_string(),
        }
    }

    ///转换为中文名称
    pub fn to_chinese_name(&self) -> String {
        match self {
            UdaMajorType::T => "工艺".to_string(),
            UdaMajorType::V => "通风".to_string(),
            UdaMajorType::E => "电气".to_string(),
            UdaMajorType::I => "仪控".to_string(),
            UdaMajorType::W => "给排水".to_string(),
            UdaMajorType::N => "BOP暖".to_string(),
            UdaMajorType::Z => "BOP水".to_string(),
            UdaMajorType::K => "通信".to_string(),
            UdaMajorType::S => "设备".to_string(),
            UdaMajorType::L => "照明".to_string(),
            UdaMajorType::F => "辐射安全".to_string(),
            UdaMajorType::H => "反应堆热工水力".to_string(),
            UdaMajorType::R => "辐射监测".to_string(),
            UdaMajorType::A => "建筑".to_string(),
            UdaMajorType::J => "结构".to_string(),
            UdaMajorType::P => "NPIC管道".to_string(),
            UdaMajorType::B => "NPIC设备".to_string(),
            UdaMajorType::C => "NPIC电气".to_string(),
            UdaMajorType::Y => "NPIC仪表".to_string(),
            UdaMajorType::X => "多专业".to_string(),
            UdaMajorType::NULL => "未知".to_string(),
        }
    }

    ///从中文描述创建UDA专业类型
    pub fn from_chinese_description(input: &str) -> Self {
        match input {
            "管道" | "工艺" => Self::T,
            "电气" => Self::E,
            "设备" => Self::S,
            "通风" => Self::V,
            "仪控" => Self::I,
            "照明" => Self::L,
            "通信" => Self::K,
            "给排水" => Self::W,
            "暖通" => Self::N,
            "辐射安全" => Self::F,
            "反应堆热工水力" => Self::H,
            "辐射监测" => Self::R,
            "建筑" => Self::A,
            "结构" => Self::J,
            "BOP水" => Self::Z,
            "BOP暖" => Self::N,
            "NPIC管道" => Self::P,
            "NPIC设备" => Self::B,
            "NPIC电气" => Self::C,
            "NPIC仪表" => Self::Y,
            "多专业" => Self::X,
            _ => Self::NULL,
        }
    }
}

///PDMS属性ArangoDB结构体
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PdmsAttrArangodb {
    ///主键
    pub _key: String,
    ///属性映射表
    #[serde(flatten)]
    pub map: HashMap<String, AttrValAql>,
}

///参考号属于哪个房间的结构体
#[serde_as]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PdmsNodeBelongRoomName {
    ///参考号
    #[serde_as(as = "DisplayFromStr")]
    pub refno: RefU64,
    ///房间名称
    pub room_name: String,
}

///PDMS名称与房间名称关联结构体
#[serde_as]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PdmsNameBelongRoomName {
    ///参考号
    #[serde_as(as = "DisplayFromStr")]
    pub refno: RefU64,
    ///名称
    #[serde_as(as = "DisplayFromStr")]
    pub name: String,
    ///房间名称
    pub room_name: String,
}

///房间下的所有节点结构体
#[serde_as]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RoomNodes {
    ///房间名称
    #[serde_as(as = "DisplayFromStr")]
    pub room_name: String,
    ///节点列表
    pub nodes: Vec<String>,
}

///元素操作类型枚举
#[derive(
    PartialEq, Debug, Default, Clone, Copy, Serialize_repr, Deserialize_repr, SurrealValue,
)]
#[repr(i32)]
pub enum EleOperation {
    #[default]
    Add = 0, // 添加
    Modified = 1,         // 修改
    GeometryModified = 2, // 几何修改
    Deleted = 3,          // 删除
    Duplicate = 4,        // 复制
    None = 5,             // 无操作
}

impl EleOperation {
    ///转换为数字
    pub fn into_num(&self) -> i32 {
        match &self {
            EleOperation::Add => 0,
            EleOperation::Modified => 1,
            EleOperation::GeometryModified => 2,
            EleOperation::Deleted => 3,
            EleOperation::Duplicate => 4,
            EleOperation::None => 5,
        }
    }
}

impl From<i32> for EleOperation {
    fn from(v: i32) -> Self {
        match v {
            0 => Self::Add,
            1 => Self::Modified,
            2 => Self::Deleted,
            3 => Self::Duplicate,
            _ => Self::None,
        }
    }
}

impl ToString for EleOperation {
    fn to_string(&self) -> String {
        match &self {
            Self::None => "未知".to_string(),
            EleOperation::Add => "增加".to_string(),
            EleOperation::Modified | EleOperation::GeometryModified => "修改".to_string(),
            EleOperation::Deleted => "删除".to_string(),
            EleOperation::Duplicate => "复制".to_string(),
        }
    }
}
