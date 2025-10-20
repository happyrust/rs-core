use crate::helper::to_e3d_name;
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
    let mut response = SUL_DB.query(&sql).await?;
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

/// 通过dbnum过滤指定类型的参考号
///
/// # 参数
/// * `nouns` - 要查询的类型名称列表
/// * `dbnum` - 数据库编号
/// * `has_children` - 是否需要有children，方便跳过一些不必要的节点
/// * `only_history` - 是否只查询历史记录（暂未实现）
///
/// # 实现说明
/// 直接查询 pe 表，使用 `noun IN [...]` 条件一次性获取所有类型的数据，
/// 比循环查询多个类型表更高效。
///
/// # 示例
/// ```ignore
/// // 查询所有 ZONE 节点
/// let zones = query_type_refnos_by_dbnum(&["ZONE"], 1112, None, false).await?;
///
/// // 查询多个类型
/// let elements = query_type_refnos_by_dbnum(&["SITE", "ZONE", "EQUI"], 1112, None, false).await?;
///
/// // 只查询有子节点的 ZONE
/// let parent_zones = query_type_refnos_by_dbnum(&["ZONE"], 1112, Some(true), false).await?;
/// ```
pub async fn query_type_refnos_by_dbnum(
    nouns: &[&str],
    dbnum: u32,
    has_children: Option<bool>,
    only_history: bool,
) -> anyhow::Result<Vec<RefnoEnum>> {
    // 将 nouns 转换为 SQL 数组格式 ['SITE', 'ZONE', ...]
    let nouns_array = nouns
        .iter()
        .map(|n| format!("'{}'", n))
        .collect::<Vec<_>>()
        .join(", ");

    // 构建 SQL 查询，直接查询 pe 表，使用 noun IN 条件
    // 根据 has_children 参数动态拼接子节点过滤条件
    let mut sql =
        format!("SELECT value id FROM pe WHERE dbnum = {dbnum} AND noun IN [{nouns_array}]");

    // 根据 has_children 参数添加额外的过滤条件
    match has_children {
        Some(true) => sql.push_str(" AND array::len(children) > 0"),
        Some(false) => sql.push_str(" AND (children == none OR array::len(children) = 0)"),
        None => {} // 不添加任何子节点过滤条件
    }

    let mut response = SUL_DB.query(&sql).await?;
    let refnos: Vec<RefnoEnum> = response.take(0)?;

    Ok(refnos)
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
/// * `mdb` - MDB名称
/// * `module` - 数据库类型
///
/// # 返回值
/// * `Vec<u32>` - 数据库编号列表
#[cached(result = true)]
pub async fn query_mdb_db_nums(mdb: Option<String>, module: DBType) -> anyhow::Result<Vec<u32>> {
    let db_type: u8 = module.into();
    let mdb = mdb.unwrap_or_else(|| crate::get_db_option().mdb_name.clone());
    let processed_mdb = crate::helper::to_e3d_name(&mdb).into_owned();
    let sql = format!("select value dbnum from CURD.refno where STYP={db_type} and NAME=$mdb");
    println!("Executing SQL: {}", sql);
    let mut response = SUL_DB.query(&sql).bind(("mdb", processed_mdb)).await?;
    let pe: Vec<u32> = response.take(0)?;
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
            select status_code ?? NONE as status_code,  * from $world.children where noun = 'SITE' and deleted = false;
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
    let mdb_name = to_e3d_name(&mdb);
    let sql = format!(
        " \
            let $f = (select value (select value DBNO from CURD.refno where STYP=1) from only MDB where NAME='{}' limit 1)[0]; \
            (select * from WORL.REFNO where dbnum=$f and noun='WORL' limit 1)[0]",
        mdb_name
    );
    let mut response = SUL_DB.query(sql).await?;
    dbg!(&response);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init_test_surreal;

    #[tokio::test]
    async fn test_get_world_refno() {
        init_test_surreal().await;

        let mdb = get_db_option().mdb_name.clone();
        println!("🧪 测试 get_world_refno, MDB: {}", mdb);

        let result = get_world_refno(mdb.clone()).await;
        assert!(result.is_ok(), "查询世界参考号应该成功");

        let refno = result.unwrap();
        println!("   ✅ 世界参考号: {:?}", refno);
        assert_ne!(refno, RefnoEnum::default(), "参考号不应为默认值");
    }

    #[tokio::test]
    async fn test_query_mdb_db_nums() {
        init_test_surreal().await;

        println!("🧪 测试 query_mdb_db_nums");

        let mdb = get_db_option().mdb_name.clone();
        let result = query_mdb_db_nums(Some(mdb), DBType::DESI).await;
        assert!(result.is_ok(), "查询数据库编号应该成功");

        let db_nums = result.unwrap();
        println!("   ✅ 查询到 {} 个数据库编号", db_nums.len());
        if !db_nums.is_empty() {
            println!("   数据库编号列表: {:?}", db_nums);
            assert!(db_nums.iter().all(|&n| n > 0), "所有数据库编号应大于0");
        }
    }

    #[tokio::test]
    async fn test_get_site_pes_by_dbnum() {
        init_test_surreal().await;

        let db_nums = query_mdb_db_nums(Some(get_db_option().mdb_name.clone()), DBType::DESI)
            .await
            .unwrap();
        if db_nums.is_empty() {
            println!("⚠️  没有可用的数据库编号，跳过测试");
            return;
        }

        let dbnum = db_nums[0];
        println!("🧪 测试 get_site_pes_by_dbnum, dbnum: {}", dbnum);

        let result = get_site_pes_by_dbnum(dbnum).await;
        assert!(result.is_ok(), "查询SITE节点应该成功");

        let sites = result.unwrap();
        println!("   ✅ 查询到 {} 个SITE节点", sites.len());

        for (i, site) in sites.iter().take(3).enumerate() {
            println!(
                "   SITE[{}]: noun={}, name={:?}, refno={:?}",
                i, site.noun, site.name, site.refno
            );
            assert_eq!(site.noun, "SITE", "节点类型应为SITE");
            assert!(!site.deleted, "SITE节点不应被删除");
        }
    }

    #[tokio::test]
    async fn test_query_type_refnos_by_dbnum() {
        init_test_surreal().await;

        let db_nums = query_mdb_db_nums(Some(get_db_option().mdb_name.clone()), DBType::DESI)
            .await
            .unwrap();
        if db_nums.is_empty() {
            println!("⚠️  没有可用的数据库编号，跳过测试");
            return;
        }

        let dbnum = db_nums[0];
        let nouns = &["SITE", "ZONE"];
        println!(
            "🧪 测试 query_type_refnos_by_dbnum, dbnum: {}, nouns: {:?}",
            dbnum, nouns
        );

        let result = query_type_refnos_by_dbnum(nouns, dbnum, None, false).await;
        assert!(result.is_ok(), "查询参考号应该成功");

        let refnos = result.unwrap();
        println!("   ✅ 查询到 {} 个参考号", refnos.len());

        if !refnos.is_empty() {
            println!("   前3个参考号: {:?}", &refnos[..refnos.len().min(3)]);
        }
    }

    #[tokio::test]
    async fn test_query_type_refnos_by_dbnum_with_children() {
        init_test_surreal().await;

        let db_nums = query_mdb_db_nums(Some(get_db_option().mdb_name.clone()), DBType::DESI)
            .await
            .unwrap();
        if db_nums.is_empty() {
            println!("⚠️  没有可用的数据库编号，跳过测试");
            return;
        }

        let dbnum = db_nums[0];
        let nouns = &["ZONE"];
        println!(
            "🧪 测试 query_type_refnos_by_dbnum (has_children=true), dbnum: {}",
            dbnum
        );

        let result = query_type_refnos_by_dbnum(nouns, dbnum, Some(true), false).await;
        assert!(result.is_ok(), "查询有子节点的参考号应该成功");

        let refnos = result.unwrap();
        println!("   ✅ 查询到 {} 个有子节点的ZONE", refnos.len());
    }

    #[tokio::test]
    async fn test_get_mdb_world_site_pes() {
        init_test_surreal().await;

        let mdb = get_db_option().mdb_name.clone();
        println!("🧪 测试 get_mdb_world_site_pes, MDB: {}", mdb);

        let result = get_mdb_world_site_pes(mdb.clone(), DBType::DESI).await;
        assert!(result.is_ok(), "查询SITE元素应该成功");

        let sites = result.unwrap();
        println!("   ✅ 查询到 {} 个SITE元素", sites.len());

        for (i, site) in sites.iter().take(3).enumerate() {
            println!("   SITE[{}]: noun={}, name={:?}", i, site.noun, site.name);
            assert_eq!(site.noun, "SITE");
        }
    }

    #[tokio::test]
    async fn test_get_mdb_world_site_ele_nodes() {
        init_test_surreal().await;

        let mdb = get_db_option().mdb_name.clone();
        println!("🧪 测试 get_mdb_world_site_ele_nodes, MDB: {}", mdb);

        let result = get_mdb_world_site_ele_nodes(mdb.clone(), DBType::DESI).await;
        assert!(result.is_ok(), "查询树形节点应该成功");

        let nodes = result.unwrap();
        println!("   ✅ 查询到 {} 个节点", nodes.len());

        for (i, node) in nodes.iter().take(3).enumerate() {
            println!(
                "   节点[{}]: order={}, name={}, noun={}, children_count={}",
                i, node.order, node.name, node.noun, node.children_count
            );
            assert_eq!(node.noun, "SITE");
            assert!(!node.name.is_empty(), "节点名称不应为空");
        }
    }

    #[tokio::test]
    async fn test_query_type_refnos_by_dbnums() {
        init_test_surreal().await;

        let db_nums = query_mdb_db_nums(Some(get_db_option().mdb_name.clone()), DBType::DESI)
            .await
            .unwrap();
        if db_nums.is_empty() {
            println!("⚠️  没有可用的数据库编号，跳过测试");
            return;
        }

        let nouns = &["WORL"];
        println!(
            "🧪 测试 query_type_refnos_by_dbnums, dbnums: {:?}, nouns: {:?}",
            db_nums, nouns
        );

        let result = query_type_refnos_by_dbnums(nouns, &db_nums).await;
        assert!(result.is_ok(), "查询参考号列表应该成功");

        let refnos = result.unwrap();
        println!("   ✅ 查询到 {} 个WORL参考号", refnos.len());
        assert_eq!(refnos.len(), db_nums.len(), "WORL数量应等于数据库数量");
    }

    #[tokio::test]
    async fn test_query_use_cate_refnos_by_dbnum() {
        init_test_surreal().await;

        let db_nums = query_mdb_db_nums(Some(get_db_option().mdb_name.clone()), DBType::DESI)
            .await
            .unwrap();
        if db_nums.is_empty() {
            println!("⚠️  没有可用的数据库编号，跳过测试");
            return;
        }

        let dbnum = db_nums[0];
        let nouns = &["EQUI", "PIPE"];
        println!(
            "🧪 测试 query_use_cate_refnos_by_dbnum, dbnum: {}, nouns: {:?}",
            dbnum, nouns
        );

        let result = query_use_cate_refnos_by_dbnum(nouns, dbnum, false).await;
        assert!(result.is_ok(), "查询类别参考号应该成功");

        let refnos = result.unwrap();
        println!("   ✅ 查询到 {} 个有类别信息的参考号", refnos.len());
    }
}

/// 测试简单的数据库连接
pub async fn test_simple_query() -> anyhow::Result<()> {
    let mut response = SUL_DB.query("RETURN 1").await?;
    let result: Vec<i32> = response.take(0)?;
    println!("Simple query result: {:?}", result);
    Ok(())
}
