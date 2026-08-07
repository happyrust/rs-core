//! 元素类型（noun）的分类旗标。
//!
//! 数据来自 AVEVA 的 `attlib.dat`——就是 `core.dll` 里 `DB_Noun::primitive()`、
//! `visible()`、`graphicsBehaviour()` 这些访问器读的同一张表。1931 个 noun、
//! 13 个旗标，提取过程见 `docs/aveva-dict/README.md`。

use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

use crate::tool::db_tool::*;

#[derive(Deserialize)]
struct BoolColumn {
    /// 绝大多数 noun 的取值，只有 `exceptions` 里的例外。
    default: bool,
    exceptions: Vec<u32>,
}

#[derive(Deserialize)]
struct IntColumn {
    default: i32,
    values: Vec<(u32, i32)>,
}

#[derive(Deserialize)]
struct RawTable {
    nouns: Vec<u32>,
    bools: HashMap<String, BoolColumn>,
    ints: HashMap<String, IntColumn>,
}

struct Table {
    nouns: Vec<u32>,
    bools: HashMap<String, (bool, HashSet<u32>)>,
    ints: HashMap<String, (i32, HashMap<u32, i32>)>,
}

static TABLE: Lazy<Table> = Lazy::new(|| {
    let raw: RawTable = serde_json::from_slice(include_bytes!("../noun_flags.json")).unwrap();
    Table {
        nouns: raw.nouns,
        bools: raw
            .bools
            .into_iter()
            .map(|(k, v)| (k, (v.default, v.exceptions.into_iter().collect())))
            .collect(),
        ints: raw
            .ints
            .into_iter()
            .map(|(k, v)| (k, (v.default, v.values.into_iter().collect())))
            .collect(),
    }
});

fn flag(field: &str, noun_hash: u32) -> bool {
    match TABLE.bools.get(field) {
        Some((default, exceptions)) => *default != exceptions.contains(&noun_hash),
        None => false,
    }
}

fn int_field(field: &str, noun_hash: u32) -> i32 {
    match TABLE.ints.get(field) {
        Some((default, values)) => *values.get(&noun_hash).unwrap_or(default),
        None => 0,
    }
}

/// 这个类型在字典里有没有定义。
pub fn is_known_hash(noun_hash: u32) -> bool {
    TABLE.nouns.contains(&noun_hash)
}

macro_rules! bool_flag {
    ($name:ident, $hash_name:ident, $field:literal, $doc:literal) => {
        #[doc = $doc]
        pub fn $hash_name(noun_hash: u32) -> bool {
            flag($field, noun_hash)
        }

        #[doc = $doc]
        pub fn $name(noun: &str) -> bool {
            flag($field, db1_hash(noun))
        }
    };
}

bool_flag!(
    is_primitive,
    is_primitive_hash,
    "PRMF",
    "设计侧几何基元（`DB_Noun::primitive()`）。347 个类型为真，BOX / CYLI / ELBO / TUBI / NOZZ 这类。"
);
bool_flag!(
    is_geomset,
    is_geomset_hash,
    "GORP",
    "目录侧几何（`DB_Noun::geomset()`）。44 个类型，SBOX / SCYL / SDSH 及其负体变体、P-point。"
);
bool_flag!(
    is_extrusion,
    is_extrusion_hash,
    "XTRF",
    "拉伸 / 旋转体（`DB_Noun::extrusion()`）。"
);
bool_flag!(
    is_point,
    is_point_hash,
    "POPF",
    "点元素（`DB_Noun::point()`）。"
);
bool_flag!(
    is_visible,
    is_visible_hash,
    "VISI",
    "可见（`DB_Noun::visible()`）。E3D 的 COLLECT 扫描默认用它剪枝。"
);
bool_flag!(
    is_pickable,
    is_pickable_hash,
    "PICK",
    "可拾取（`DB_Noun::pickable()`）。"
);
bool_flag!(
    is_toplevel,
    is_toplevel_hash,
    "TOPF",
    "顶层可绘制单元（`DB_Noun::toplevel()`），EQUI / BRAN / STRU / SCTN / PANE 这类。"
);
bool_flag!(
    default_volume_query,
    default_volume_query_hash,
    "VOLDEF",
    "参与默认体积查询（`DB_Noun::defaultVolumeQuery()`）。"
);
bool_flag!(
    clasher_within,
    clasher_within_hash,
    "CLWTHN",
    "碰撞检查：内部自检（`DB_Noun::clasherWithin()`）。"
);
bool_flag!(
    clasher_section,
    clasher_section_hash,
    "CLRSEC",
    "碰撞检查：截面（`DB_Noun::clasherSection()`）。"
);

/// 绘制行为分类（`DB_Noun::graphicsBehaviour()`）。
///
/// 注意它**不是**“有没有几何”：BOX 和 EQUI 都是 0，SITE / ZONE / WORL 是 2，
/// 制图类是 1，电缆桥架与辅助元素是 3。0 表示普通。
pub fn graphics_behaviour(noun: &str) -> i32 {
    int_field("GRAPH", db1_hash(noun))
}

/// 空间索引分级（`DB_Noun::spatialMap()`）。
pub fn spatial_map(noun: &str) -> i32 {
    int_field("IMAP", db1_hash(noun))
}

/// 次级层级（`DB_Noun::secondaryHierarchy()`）。
pub fn secondary_hierarchy(noun: &str) -> i32 {
    int_field("SECOND", db1_hash(noun))
}

/// 列出某个布尔旗标为真的所有类型，等价于 `DB_Noun::findAllNouns(field, true, ...)`。
///
/// 字段名用字典里的原名：`PRMF` `GORP` `XTRF` `POPF` `VISI` `PICK` `TOPF`
/// `VOLDEF` `CLWTHN` `CLRSEC`。
pub fn nouns_with_flag(field: &str) -> Vec<String> {
    let Some((default, exceptions)) = TABLE.bools.get(field) else {
        return vec![];
    };
    let mut out: Vec<String> = if *default {
        TABLE
            .nouns
            .iter()
            .filter(|h| !exceptions.contains(h))
            .map(|h| db1_dehash(*h))
            .collect()
    } else {
        exceptions.iter().map(|h| db1_dehash(*h)).collect()
    };
    out.sort();
    out
}

/// 所有设计侧几何基元，`is_primitive` 为真的那 347 个。
pub fn primitive_nouns() -> Vec<String> {
    nouns_with_flag("PRMF")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive() {
        for n in ["BOX", "CYLI", "ELBO", "TUBI", "NOZZ", "NBOX", "SNOU"] {
            assert!(is_primitive(n), "{n} should be a primitive");
        }
        for n in [
            "SITE", "ZONE", "EQUI", "PIPE", "BRAN", "STRU", "WORL", "TEXT",
        ] {
            assert!(!is_primitive(n), "{n} should not be a primitive");
        }
        assert_eq!(primitive_nouns().len(), 347);
    }

    #[test]
    fn test_geomset_is_a_separate_set() {
        // 目录侧与设计侧是两套旗标，不能混用
        assert!(is_geomset("SBOX") && !is_primitive("SBOX"));
        assert!(is_primitive("BOX") && !is_geomset("BOX"));
        assert_eq!(nouns_with_flag("GORP").len(), 44);
    }

    #[test]
    fn test_int_fields() {
        // GRAPH 是绘制行为分类，不是“有没有几何”
        assert_eq!(graphics_behaviour("BOX"), 0);
        assert_eq!(graphics_behaviour("EQUI"), 0);
        assert_eq!(graphics_behaviour("SITE"), 2);
        assert_eq!(graphics_behaviour("ZONE"), 2);

        assert_eq!(spatial_map("BOX"), 5);
        assert_eq!(spatial_map("ELBO"), 1);
        assert_eq!(spatial_map("ZONE"), 2);
    }

    #[test]
    fn test_bool_defaults_and_exceptions() {
        // VISI 的多数值是真，只有 118 个例外
        assert!(is_visible("EQUI"));
        assert!(is_toplevel("EQUI") && is_toplevel("BRAN"));
        assert!(!is_toplevel("BOX"));
        assert!(default_volume_query("EQUI"));
        assert!(!default_volume_query("BOX"));
        assert!(!is_primitive("NOT_A_NOUN"));
    }

    #[test]
    fn test_gate_from_flags() {
        use crate::noun_graph::{NOUN_GRAPH, descend_gate};
        let prims = primitive_nouns();
        let refs: Vec<&str> = prims.iter().map(String::as_str).collect();
        let gate = descend_gate(&refs);

        // 层级容器一个都不能漏，否则会漏查几何
        for n in [
            "WORL", "SITE", "ZONE", "EQUI", "SUBE", "PIPE", "BRAN", "STRU", "FRMW", "SCTN",
        ] {
            assert!(gate.contains(&n.to_string()), "gate missing {n}");
        }
        // 纯叶子的非几何元素不该进白名单
        for n in ["TEXT", "PPLIST"] {
            assert!(
                !gate.contains(&n.to_string()),
                "gate should not contain {n}"
            );
        }
        // 白名单要显著小于全图，否则等于没剪
        assert!(
            gate.len() * 2 < NOUN_GRAPH.node_count(),
            "gate {} prunes too little of {}",
            gate.len(),
            NOUN_GRAPH.node_count()
        );
    }
}
