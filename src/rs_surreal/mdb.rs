// 导入所需的依赖
use crate::pdms_types::EleTreeNode;
use crate::pe::SPdmsElement;
use crate::{NamedAttrMap, RefnoEnum};
use crate::{SUL_DB, SurlValue};
use crate::{get_db_option, helper, types::*};
use cached::proc_macro::cached;
use indexmap::IndexMap;
use itertools::Itertools;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::f32::consts::E;
use std::sync::Mutex;

/// 数据库类型枚举
/// 用于区分不同类型的数据库模块
#[derive(IntoPrimitive, TryFromPrimitive, Clone, Copy, Hash, Eq, PartialEq, Debug)]
#[repr(u8)]
pub enum DBType {
    DESI = 1,  // 设计数据库
    CATA = 2,  // 目录数据库
    PROP = 3,  // 属性数据库
    ISOD = 4,  // ISO图数据库
    PADD = 5,  // 管道数据库
    DICT = 6,  // 字典数据库
    ENGI = 7,  // 工程数据库
    SCHE = 14, // 图纸数据库
    UNSET,     // 未设置类型
}

/// 从数据库中获取MDB和DB表的信息
///
/// # 参数说明
///
/// * `mdb` - 要查询的MDB名称
/// * `db_type` - 数据库类型过滤条件
///
/// # 返回值
///
/// 返回包含refno、noun、name、owner和children_count字段的查询结果
#[cached(result = true)]
pub async fn get_mdb_world_site_ele_nodes(
    mdb: String,
    module: DBType,
) -> anyhow::Result<Vec<EleTreeNode>> {
    let db_type: u8 = module.into();
    let sql = format!(
        r#"
        let $dbnos = select value (select value DBNO from CURD.refno where STYP == {db_type}) from only MDB where NAME == "{mdb}" limit 1;
        let $a = (select value id from (select REFNO.id as id, array::find_index($dbnos, REFNO.dbnum) as o from WORL where REFNO.dbnum in $dbnos order by o));
        select refno, noun, name, owner, array::len(select value in from <-pe_owner) as children_count from array::flatten(select value in from $a<-pe_owner) where noun='SITE';
        "#,
        db_type = db_type,
        mdb = mdb
    );
    // 执行查询
    let mut response = SUL_DB.query(&sql).await.unwrap();
    // 获取结果
    let mut nodes: Vec<EleTreeNode> = response.take(2)?;
    // 处理节点顺序和名称
    for (i, node) in nodes.iter_mut().enumerate() {
        node.order = i as _;
        if node.name.is_empty() {
            node.name = format!("SITE {}", i + 1);
        }
    }
    //检查名称，如果没有给名字的，需要给上默认值, todo 后续如果是删除了又增加，名称后面的数字可能会继续增加
    Ok(nodes)
}

/// 创建MDB世界站点PE表
///
/// # 参数
/// * `mdb` - MDB名称
/// * `module` - 数据库类型
///
/// # 返回值
/// * `bool` - 创建是否成功
pub async fn create_mdb_world_site_pes_table(mdb: String, module: DBType) -> anyhow::Result<bool> {
    let db_type: u8 = module.into();
    let mut response = SUL_DB
        .query(r#"
            let $dbnos = select value (select value DBNO from CURD.refno where STYP=$db_type) from only MDB where NAME=$mdb limit 1;
            let $a = (select value id from (select REFNO.id as id, array::find_index($dbnos, REFNO.dbnum) as o from WORL where REFNO.dbnum in $dbnos order by o));
            array::flatten(select value in.* from $a<-pe_owner[? in.noun='SITE'])
        "#)
        .bind(("mdb", mdb))
        .bind(("db_type", db_type))
        .await?;
    let sites: Vec<SPdmsElement> = response.take(2)?;
    if sites.is_empty() {
        return Ok(false);
    }
    let mut relate_sql = String::new();
    let mdb_world = sites[0].owner.to_pe_key();
    for (i, site) in sites.into_iter().enumerate() {
        relate_sql.push_str(&format!(
            "relate {}->site_relate:[{}, {i}]->{};",
            site.refno.to_pe_key(),
            &mdb_world,
            &mdb_world
        ));
    }

    Ok(true)
}

/// 通过数据库编号列表查询指定类型的参考号
///
/// # 参数
/// * `nouns` - 要查询的类型名称列表
/// * `dbnums` - 数据库编号列表
///
/// # 返回值
/// * `Vec<RefnoEnum>` - 参考号列表
pub async fn query_type_refnos_by_dbnums(
    nouns: &[&str],
    dbnums: &[u32],
) -> anyhow::Result<Vec<RefnoEnum>> {
    let mut result = vec![];
    for noun in nouns {
        let sql = if dbnums.is_empty() {
            format!("select value id from {noun}")
        } else {
            format!(
                "select value id from {noun} where REFNO.dbnum in [{}]",
                dbnums.into_iter().map(|x| x.to_string()).join(",")
            )
        };
        let mut response = SUL_DB.query(&sql).await?;
        let refnos: Vec<RefnoEnum> = response.take(0)?;
        result.extend(refnos);
    }
    Ok(result)
}

///通过dbnum过滤指定类型的参考号
/// 通过has_children 指定是否需要有children，方便跳过一些不变要的节点
/// todo 在属性里直接加上DBNO这个属性，而不是需要去pe里去取
pub async fn query_type_refnos_by_dbnum(
    nouns: &[&str],
    dbnum: u32,
    has_children: Option<bool>,
    only_history: bool,
) -> anyhow::Result<Vec<RefnoEnum>> {
    let mut result = vec![];
    for noun in nouns {
        let table = if only_history {
            format!("{noun}_H")
        } else {
            format!("{noun}")
        };
        // 优先使用 id 字段（对于大部分表，id 就是 refno）
        // 但对于某些表(如 SITE, ZONE), id 不是有效 refno，会 fallback 到从 REFNO 计算
        let sql = match has_children {
            Some(true) => {
                format!(
                    "select id, REFNO from {table} where REFNO.dbnum={dbnum} and (REFNO<-pe_owner.in)[0] != none"
                )
            }
            Some(false) => {
                format!(
                    "select id, REFNO from {table} where REFNO.dbnum={dbnum} and (REFNO<-pe_owner.in)[0] == none"
                )
            }
            None => {
                format!("select id, REFNO from {table} where REFNO.dbnum={dbnum}")
            }
        };
        // println!("query_type_refnos_by_dbnum sql: {}", sql);
        let mut response = SUL_DB.query(&sql).await?;

        // 使用 serde_json::Value 以兼容不同类型的 REFNO 字段
        let records: Vec<serde_json::Value> = response.take(0)?;

        use crate::types::RefU64;
        for record in records {
            // 先尝试从 id 解析
            if let Some(id_val) = record.get("id") {
                // 尝试将 id 解析为 Thing
                if let Ok(thing) = serde_json::from_value::<surrealdb::sql::Thing>(id_val.clone()) {
                    if let Ok(refno) = RefnoEnum::try_from(thing) {
                        result.push(refno);
                        continue;
                    }
                }
            }

            // id 解析失败，尝试从 REFNO 对象计算
            if let Some(refno_val) = record.get("REFNO") {
                if let Some(refno_obj) = refno_val.as_object() {
                    let dbnum = refno_obj.get("dbnum").and_then(|v| v.as_u64()).unwrap_or(0);
                    let nume = refno_obj.get("nume").and_then(|v| v.as_u64()).unwrap_or(0);
                    result.push(RefnoEnum::from(RefU64::from(dbnum * 1000000 + nume)));
                }
            }
        }
    }
    Ok(result)
}

/// 查询使用类别参考号
/// 额外检查SPRE和CATR不能同时为空
///
/// # 参数
/// * `nouns` - 要查询的类型名称列表
/// * `dbnum` - 数据库编号
/// * `only_history` - 是否只查询历史记录
pub async fn query_use_cate_refnos_by_dbnum(
    nouns: &[&str],
    dbnum: u32,
    only_history: bool,
) -> anyhow::Result<Vec<RefnoEnum>> {
    let mut result = vec![];
    for noun in nouns {
        let table = if only_history {
            format!("{noun}_H")
        } else {
            format!("{noun}")
        };
        let sql = format!(
            "select value id from {table} where REFNO.dbnum={dbnum} and (SPRE != none or CATR != none)"
        );
        let mut response = SUL_DB.query(&sql).await?;
        let refnos: Vec<RefnoEnum> = response.take(0)?;
        result.extend(refnos);
    }
    Ok(result)
}

/// 去掉父类型是BRAN和HANGER的记录
// pub async fn query_type_refnos_by_dbnum_exclude_bran_hang(nouns: &[&str], dbnum: u32) -> anyhow::Result<Vec<RefnoEnum>> {
//     let mut result = vec![];
//     for noun in nouns {
//         let sql = format!("select value id from {noun} where REFNO.dbnum={dbnum} and OWNER.noun not in ['BRAN', 'HANG']");
//         let mut response = SUL_DB.query(&sql).await?;
//         let refnos: Vec<RefnoEnum> = response.take(0)?;
//         result.extend(refnos);
//     }
//     Ok(result)
// }

/// 查询MDB数据库编号
///
/// # 参数
/// * `module` - 数据库类型
///
/// # 返回值
/// * `Vec<u32>` - 数据库编号列表
#[cached(result = true)]
pub async fn query_mdb_db_nums(module: DBType) -> anyhow::Result<Vec<u32>> {
    let db_type: u8 = module.into();
    let mdb = &get_db_option().mdb_name;
    let mdb = crate::helper::to_e3d_name(mdb);
    let mut response = SUL_DB
        .query(r#"
            let $dbnos = select value (select value DBNO from CURD.refno where STYP=$db_type) from only MDB where NAME=$mdb limit 1;
            select value dbnum from (select REFNO.dbnum as dbnum, array::find_index($dbnos, REFNO.dbnum) as o
                from WORL where REFNO.dbnum in $dbnos order by o);
        "#)
        .bind(("mdb", mdb))
        .bind(("db_type", db_type))
        .await?;
    dbg!(&response);
    let pe: Vec<u32> = response.take(1)?;
    Ok(pe)
}

/// 查询MDB的world下的所有PE
///
/// # 参数
/// * `mdb` - MDB名称
/// * `module` - 数据库类型
///
/// # 返回值
/// * `Vec<SPdmsElement>` - PE元素列表
#[cached(result = true)]
pub async fn get_mdb_world_site_pes(
    mdb: String,
    module: DBType,
) -> anyhow::Result<Vec<SPdmsElement>> {
    let db_type: u8 = module.into();
    let mut response = SUL_DB
        .query(r#"
            let $dbnos = select value (select value DBNO from CURD.refno where STYP=$db_type) from only MDB where NAME=$mdb limit 1;
            let $a = (select value id from (select REFNO.id as id, array::find_index($dbnos, REFNO.dbnum) as o from WORL where REFNO.dbnum in $dbnos order by o));
            array::flatten(select value in.* from $a<-pe_owner)[?noun = 'SITE']
        "#)
        .bind(("mdb", mdb))
        .bind(("db_type", db_type))
        .await?;
    let pe: Vec<SPdmsElement> = response.take(2)?;
    Ok(pe)
}

/// 通过 dbnum 查询该数据库下的所有 SITE 节点
///
/// ## 功能说明
/// 查询指定 dbnum 下所有 WORL 节点的直接子节点中类型为 SITE 的节点
///
/// ## 查询逻辑
/// 1. 从 WORL 表查找 dbnum 对应的世界节点
/// 2. 通过 pe_owner 关系反向查找 WORL 的子节点
/// 3. 筛选出 noun = 'SITE' 的节点
///
/// ## 与 get_mdb_world_site_pes 的区别
/// - `get_mdb_world_site_pes`: 通过 MDB 名称查询，支持多个 dbnum，保持原有顺序
/// - `get_site_pes_by_dbnum`: 直接通过单个 dbnum 查询，更快速直接
///
/// # 参数
/// * `dbnum` - 数据库编号
///
/// # 返回值
/// * `Vec<SPdmsElement>` - SITE 元素列表
///
/// # 示例
/// ```rust
/// let sites = get_site_pes_by_dbnum(3001).await?;
/// for site in sites {
///     println!("SITE: {}, refno: {}", site.name, site.refno());
/// }
/// ```
pub async fn get_site_pes_by_dbnum(dbnum: u32) -> anyhow::Result<Vec<SPdmsElement>> {
    let mut response = SUL_DB
        .query(r#"
            let $world = (select value REFNO from WORL where REFNO.dbnum = $dbnum and REFNO.noun = 'WORL' limit 1)[0];
            select value in.* from $world<-pe_owner where in.noun = 'SITE' and in.deleted = false
        "#)
        .bind(("dbnum", dbnum))
        .await?;
    let sites: Vec<SPdmsElement> = response.take(1)?;
    Ok(sites)
}

/// 获取世界节点
///
/// # 参数
/// * `mdb` - MDB名称
///
/// # 返回值
/// * `Option<SPdmsElement>` - 世界节点元素
#[cached(result = true)]
pub async fn get_world(mdb: String) -> anyhow::Result<Option<SPdmsElement>> {
    let sql = format!(
        " \
            let $f = (select value (select value DBNO from CURD.refno where STYP=1) from only MDB where NAME='{}' limit 1)[0]; \
            (select value REFNO.* from WORL where REFNO.dbnum=$f and REFNO.noun='WORL' limit 1)[0]",
        mdb
    );
    let mut response = SUL_DB.query(sql).await.unwrap();
    let pe: Option<SPdmsElement> = response.take(1)?;
    Ok(pe)
}

/// 获取世界参考号
///
/// # 参数
/// * `mdb` - MDB数据库名称
///
/// # 返回值
/// * `RefnoEnum` - 世界节点的参考号
///
/// # 说明
/// * 使用缓存优化查询性能
/// * 从WORL表中查询指定MDB下的世界节点参考号
/// * 如果未找到则返回默认值
#[cached(result = true)]
pub async fn get_world_refno(mdb: String) -> anyhow::Result<RefnoEnum> {
    // 标准化MDB名称,确保以'/'开头
    let mdb_name = if mdb.starts_with('/') {
        mdb.clone()
    } else {
        format!("/{}", mdb)
    };

    // 构建SQL查询
    // 1. 首先获取MDB对应的DBNO(数据库编号)
    // 2. 然后查询该DBNO下类型为WORL的参考号
    let sql = format!(
        " \
            let $f = (select value (select value DBNO from CURD.refno where STYP=1) from only MDB where NAME='{}' limit 1)[0]; \
            (select value REFNO from WORL where REFNO.dbnum=$f and REFNO.noun='WORL' limit 1)[0]",
        mdb_name
    );

    // 执行查询并获取结果
    let mut response = SUL_DB.query(sql).await?;
    let id: Option<RefnoEnum> = response.take(1)?;
    Ok(id.unwrap_or_default())
}
