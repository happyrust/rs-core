use crate::helper::to_e3d_name;
// 导入所需的依赖
use crate::pdms_types::EleTreeNode;
use crate::pe::SPdmsElement;
use crate::{NamedAttrMap, RefnoEnum};
use crate::{SUL_DB, SurlValue, SurrealQueryExt};
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

/// 名称过滤条件
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameFilter {
    /// 关键字，使用 `string::contains` 匹配
    pub keyword: String,
    /// 是否区分大小写
    pub case_sensitive: bool,
}

impl NameFilter {
    /// 创建新的名称过滤条件
    pub fn new(keyword: impl Into<String>, case_sensitive: bool) -> Self {
        Self {
            keyword: keyword.into(),
            case_sensitive,
        }
    }

    /// 获取标准化的关键字
    ///
    /// # 返回值
    /// - 如果关键字为空或只有空白字符，返回 `None`
    /// - 如果区分大小写，返回去除首尾空白的字符串
    /// - 如果不区分大小写，返回去除首尾空白并转换为小写的字符串
    fn normalized_keyword(&self) -> Option<String> {
        let trimmed = self.keyword.trim();
        if trimmed.is_empty() {
            return None;
        }

        if self.case_sensitive {
            Some(trimmed.to_string())
        } else {
            Some(trimmed.to_lowercase())
        }
    }
}

/// 从数据库中获取 MDB 世界下的所有 SITE 节点
///
/// # 功能说明
/// 查询指定 MDB 和模块类型下所有 WORLD 节点的直接子节点中类型为 SITE 的节点，
/// 并返回树形节点结构，包含节点的基本信息和子节点数量。
///
/// # 参数
/// * `mdb` - 要查询的 MDB 名称（会自动标准化为 E3D 格式）
/// * `module` - 数据库模块类型（DESI、CATA、PROP 等）
///
/// # 返回值
/// 返回包含以下字段的 SITE 节点列表：
/// - `refno` - 节点参考号
/// - `noun` - 节点类型（固定为 "SITE"）
/// - `name` - 节点名称（如果为空会自动生成 "SITE N" 格式）
/// - `owner` - 所属的 WORLD 节点
/// - `children_count` - 子节点数量
/// - `order` - 节点在列表中的顺序
///
/// # 缓存
/// 该函数使用 `#[cached]` 宏进行结果缓存，相同参数的重复调用会直接返回缓存结果
///
/// # 示例
/// ```rust
/// let sites = get_mdb_world_site_ele_nodes("/651YK".to_string(), DBType::DESI).await?;
/// for site in sites {
///     println!("SITE: {}, 子节点数: {}", site.name, site.children_count);
/// }
/// ```
#[cached(result = true)]
pub async fn get_mdb_world_site_ele_nodes(
    mdb: String,
    module: DBType,
) -> anyhow::Result<Vec<EleTreeNode>> {
    let db_type: u8 = module.into();
    let mdb_name = to_e3d_name(&mdb);
    let sql = format!(
        r#"
        let $dbnos = select value (select value DBNO from CURD.refno where STYP == {db_type}) from only MDB where NAME == "{mdb}" limit 1;
        let $a = (select value id from (select REFNO.id as id, array::find_index($dbnos, REFNO.dbnum) as o from WORL where REFNO.dbnum in $dbnos order by o));
        select refno, noun, name, owner, array::len(children) as children_count, 0 as order from array::flatten($a.children) where noun='SITE';
        "#,
        db_type = db_type,
        mdb = mdb_name
    );
    //
    // 执行查询
    let mut response = SUL_DB.query_response(&sql).await?;
    // 获取结果
    let mut nodes: Vec<EleTreeNode> = response.take(2).unwrap();
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

/// 创建 MDB 世界站点 PE 关系表
///
/// # 功能说明
/// 为指定 MDB 的所有 SITE 节点创建与 WORLD 节点的关系映射（site_relate），
/// 用于建立站点之间的层级关系。
///
/// # 参数
/// * `mdb` - MDB 名称
/// * `module` - 数据库模块类型
///
/// # 返回值
/// * `Ok(true)` - 成功创建关系表
/// * `Ok(false)` - 没有找到 SITE 节点，未创建
/// * `Err` - 查询或创建过程中发生错误
///
/// # 实现细节
/// 1. 查询 MDB 下指定模块的所有 WORLD 节点
/// 2. 获取这些 WORLD 节点下的所有 SITE 子节点
/// 3. 为每个 SITE 创建 `site_relate` 关系，连接到其所属的 WORLD 节点
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
/// # 功能说明
/// 在指定的数据库编号列表中查询特定类型的所有参考号。
/// 如果数据库编号列表为空，则查询所有数据库。
///
/// # 参数
/// * `nouns` - 要查询的 Noun 类型名称列表（如 ["SITE", "ZONE", "EQUI"]）
/// * `dbnums` - 数据库编号列表（空列表表示查询所有数据库）
///
/// # 返回值
/// * `Vec<RefnoEnum>` - 匹配的参考号列表
///
/// # 示例
/// ```rust
/// // 查询特定数据库中的 SITE 和 ZONE
/// let refnos = query_type_refnos_by_dbnums(&["SITE", "ZONE"], &[3001, 3002]).await?;
///
/// // 查询所有数据库中的 EQUI
/// let all_equi = query_type_refnos_by_dbnums(&["EQUI"], &[]).await?;
/// ```
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
        let mut response = SUL_DB.query_response(&sql).await?;
        let refnos: Vec<RefnoEnum> = response.take(0)?;
        result.extend(refnos);
    }
    Ok(result)
}

/// 统计指定 Noun 在全库范围内的实例数量
///
/// # 参数
/// - `noun`: Noun 类型名称（如 "SITE"、"ZONE"、"EQUI" 等）
///
/// # 返回值
/// - 该类型在整个数据库中的实例数量
///
/// # 示例
/// ```rust
/// let site_count = count_refnos_by_noun("SITE").await?;
/// println!("数据库中共有 {} 个 SITE 节点", site_count);
/// ```
pub async fn count_refnos_by_noun(noun: &str) -> anyhow::Result<u64> {
    let sql = format!("select value count() from only {noun} group all limit 1");
    let mut response = SUL_DB.query_response(&sql).await?;
    let count: Option<u64> = response.take(0)?;
    Ok(count.unwrap_or(0))
}

/// 统计指定 Noun 在指定数据库中的实例数量
///
/// # 参数
/// - `noun`: Noun 类型名称
/// - `dbnums`: 要查询的数据库编号列表（空列表表示查询所有数据库）
pub async fn count_refnos_by_noun_with_dbnums(noun: &str, dbnums: &[u32]) -> anyhow::Result<u64> {
    let sql = if dbnums.is_empty() {
        format!("select value count() from only {noun} group all limit 1")
    } else {
        format!(
            "select value count() from only {noun} where REFNO.dbnum in [{}] group all limit 1",
            dbnums
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join(",")
        )
    };
    let mut response = SUL_DB.query_response(&sql).await?;
    let count: Option<u64> = response.take(0)?;
    Ok(count.unwrap_or(0))
}

/// 按照 LIMIT / START 分页查询指定 Noun 的实例列表
///
/// # 功能说明
/// 对指定类型的所有实例进行分页查询，支持大数据量的分批加载。
///
/// # 参数
/// - `noun`: Noun 类型名称
/// - `start`: 起始偏移量（从 0 开始）
/// - `limit`: 每页数量（0 表示不查询，返回空列表）
///
/// # 返回值
/// - 按 ID 排序的参考号列表（最多 `limit` 个）
///
/// # 示例
/// ```rust
/// // 获取第 1 页（每页 100 条）
/// let page1 = query_refnos_by_noun_page("EQUI", 0, 100).await?;
///
/// // 获取第 2 页
/// let page2 = query_refnos_by_noun_page("EQUI", 100, 100).await?;
/// ```
pub async fn query_refnos_by_noun_page(
    noun: &str,
    start: usize,
    limit: usize,
) -> anyhow::Result<Vec<RefnoEnum>> {
    if limit == 0 {
        return Ok(Vec::new());
    }

    let sql = format!("select value id from {noun} order by id limit {limit} start {start}");

    let mut response = SUL_DB.query_response(&sql).await?;
    let refnos: Vec<RefnoEnum> = response.take(0)?;
    Ok(refnos)
}

/// 按照 LIMIT / START 分页查询指定 Noun 在指定数据库中的实例列表
///
/// # 参数
/// - `noun`: Noun 类型名称
/// - `start`: 起始偏移量
/// - `limit`: 每页数量
/// - `dbnums`: 要查询的数据库编号列表（空列表表示查询所有数据库）
pub async fn query_refnos_by_noun_page_with_dbnums(
    noun: &str,
    start: usize,
    limit: usize,
    dbnums: &[u32],
) -> anyhow::Result<Vec<RefnoEnum>> {
    if limit == 0 {
        return Ok(Vec::new());
    }

    let sql = if dbnums.is_empty() {
        format!("select value id from {noun} order by id limit {limit} start {start}")
    } else {
        format!(
            "select value id from {noun} where REFNO.dbnum in [{}] order by id limit {limit} start {start}",
            dbnums
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join(",")
        )
    };

    let mut response = SUL_DB.query_response(&sql).await?;
    let refnos: Vec<RefnoEnum> = response.take(0)?;
    Ok(refnos)
}

/// 根据子节点存在性过滤参考号列表
///
/// # 参数
/// * `refnos` - 待过滤的 refno 列表
/// * `has_children` - true 表示只保留有子节点的，false 表示只保留没有子节点的
///
/// # 返回值
/// 过滤后的 refno 列表
///
/// # 实现细节
/// - 为避免 SQL 语句过长，采用分批处理策略，每批最多处理 500 个参考号
/// - 查询 PE 表的 children 字段长度来判断是否有子节点
async fn filter_refnos_by_children(
    refnos: Vec<RefnoEnum>,
    has_children: bool,
) -> anyhow::Result<Vec<RefnoEnum>> {
    if refnos.is_empty() {
        return Ok(refnos);
    }

    // 分批处理，每批最多 500 个，避免 SQL 语句过长
    const BATCH_SIZE: usize = 500;
    let mut result = Vec::new();

    for chunk in refnos.chunks(BATCH_SIZE) {
        let pe_keys: Vec<String> = chunk.iter().map(|r| r.to_pe_key()).collect();

        let pe_keys_str = pe_keys.join(", ");
        let sql = format!(
            "select value id from [{}] where array::len(children) {} 0",
            pe_keys_str,
            if has_children { ">" } else { "=" }
        );

        let mut response = SUL_DB.query_response(&sql).await?;
        let mut filtered_refnos: Vec<RefnoEnum> = response.take(0)?;
        result.append(&mut filtered_refnos);
    }

    Ok(result)
}

/// 通过dbnum过滤指定类型的参考号
///
/// # 参数
/// * `nouns` - 要查询的类型名称列表
/// * `dbnum` - 数据库编号
/// * `has_children` - 是否需要有children，方便跳过一些不必要的节点
/// * `only_history` - 是否只查询历史记录
///
/// # 实现说明
/// 使用 SurrealDB 的多表查询语法，直接从类型表（如 ZONE、PLOO 等）查询，
/// 使用逗号分隔的表名实现一次性查询多个类型。
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
    query_type_refnos_by_dbnum_with_filter(nouns, dbnum, has_children, only_history, None).await
}

/// 带名称过滤能力的类型查询
///
/// # 参数
/// * `nouns` - 类型列表
/// * `dbnum` - 数据库编号
/// * `has_children` - 子节点过滤，同 `query_type_refnos_by_dbnum`
/// * `only_history` - 历史记录开关（暂未实现，保持占位）
/// * `name_filter` - 可选的名称过滤条件
pub async fn query_type_refnos_by_dbnum_with_filter(
    nouns: &[&str],
    dbnum: u32,
    has_children: Option<bool>,
    only_history: bool,
    name_filter: Option<&NameFilter>,
) -> anyhow::Result<Vec<RefnoEnum>> {
    // 构建表名列表（支持历史表）
    let tables: Vec<String> = nouns
        .iter()
        .map(|noun| {
            if only_history {
                format!("{noun}_H")
            } else {
                noun.to_string()
            }
        })
        .collect();

    let tables_str = tables.join(", ");

    // 如果有名称过滤，使用多表查询语法
    if let Some(filter) = name_filter {
        if let Some(keyword) = filter.normalized_keyword() {
            let mut sql = format!(
                "select value REFNO from {} where REFNO.dbnum = $dbnum and NAME != NONE",
                tables_str
            );

            if filter.case_sensitive {
                sql.push_str(" and string::contains(NAME, $keyword)");
            } else {
                sql.push_str(" and string::contains(string::lowercase(NAME), $keyword)");
            }

            let kw = keyword.clone();
            let mut query = SUL_DB
                .query(&sql)
                .bind(("dbnum", dbnum))
                .bind(("keyword", kw));

            let mut response = query.await?;
            let refnos: Vec<RefnoEnum> = response.take(0)?;

            // 如果需要过滤 has_children，通过 pe 表来过滤
            return if let Some(has_children_flag) = has_children {
                filter_refnos_by_children(refnos, has_children_flag).await
            } else {
                Ok(refnos)
            };
        }
    }

    // 基本的多表查询：从类型表查询 REFNO
    let mut sql = format!(
        "select value REFNO from {} where REFNO.dbnum = $dbnum",
        tables_str
    );

    let mut query = SUL_DB.query(&sql).bind(("dbnum", dbnum));
    let mut response = query.await?;
    let mut refnos: Vec<RefnoEnum> = response.take(0)?;

    // 如果需要过滤 has_children，通过 pe 表来过滤
    if let Some(has_children_flag) = has_children {
        refnos = filter_refnos_by_children(refnos, has_children_flag).await?;
    }

    Ok(refnos)
}

/// 查询使用类别的参考号
///
/// # 功能说明
/// 查询指定类型中包含类别信息的参考号，即 SPRE（规格参考）或 CATR（目录参考）不为空的节点。
/// 这些节点通常关联了设备规格或目录信息。
///
/// # 参数
/// * `nouns` - 要查询的 Noun 类型名称列表
/// * `dbnum` - 数据库编号
/// * `only_history` - 是否只查询历史记录（true 时查询 `{NOUN}_H` 表）
///
/// # 返回值
/// * `Vec<RefnoEnum>` - 包含类别信息的参考号列表
///
/// # 示例
/// ```rust
/// // 查询有规格信息的设备和管道
/// let cate_items = query_use_cate_refnos_by_dbnum(&["EQUI", "PIPE"], 3001, false).await?;
/// ```
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
        let mut response = SUL_DB.query_response(&sql).await?;
        let refnos: Vec<RefnoEnum> = response.take(0)?;
        result.extend(refnos);
    }
    Ok(result)
}

/// 通过 MDB 名称和数据库类型查询指定类型的数据
///
/// 这个函数提供了更灵活的查询方式，可以通过 MDB 名称和 DB 类型来确定查询范围，
/// 而不需要手动指定单个 dbnum。
///
/// # 参数
/// * `nouns` - 类型列表（例如 ["SITE", "ZONE", "BRAN"]）
/// * `mdb_name` - MDB 名称（例如 "/651YK"）
/// * `db_type` - 数据库类型（1=DESI, 2=CATA, 3=PROP 等）
/// * `name_filter` - 可选的名称过滤条件
///
/// # 实现说明
/// 1. 首先通过 `fn::query_mdb_db_nums` 获取该 MDB 下指定类型的数据库编号列表
/// 2. 使用逗号拼接多表语法，在单个查询中从所有表中获取数据
/// 3. 使用 `REFNO.dbnum IN [...]` 来过滤数据库编号
///
/// # 示例
/// ```rust
/// let filter = NameFilter::new("R", false);
/// let results = query_type_refnos_in_mdb(
///     &["SITE", "BRAN"],
///     "/651YK",
///     DBType::DESI,
///     Some(&filter)
/// ).await?;
/// ```
pub async fn query_type_refnos_in_mdb(
    nouns: &[&str],
    mdb_name: &str,
    db_type: DBType,
    name_filter: Option<&NameFilter>,
) -> anyhow::Result<Vec<RefnoEnum>> {
    let processed_mdb = crate::helper::to_e3d_name(mdb_name).into_owned();
    let db_type_num: u8 = db_type.into();

    // 构建逗号拼接的表名
    let tables = nouns.join(", ");

    // 使用 fn::query_mdb_db_nums 获取该 MDB 下的数据库编号
    let sql = format!(
        "let $dbnums = fn::query_mdb_db_nums($mdb, {db_type_num}); \
         select value REFNO from {tables} where NAME != NONE \
         and REFNO.dbnum in $dbnums"
    );

    let mut sql = sql;

    // 如果需要名称过滤，添加名称匹配条件
    if let Some(filter) = name_filter {
        if let Some(keyword) = filter.normalized_keyword() {
            if filter.case_sensitive {
                sql.push_str(" and string::contains(NAME, $keyword)");
            } else {
                sql.push_str(" and string::contains(string::lowercase(NAME), $keyword)");
            }

            let mut query = SUL_DB
                .query(&sql)
                .bind(("mdb", processed_mdb.clone()))
                .bind(("keyword", keyword));

            let mut response = query.await?;
            let refnos: Vec<RefnoEnum> = response.take(1)?; // 注意：这里应该是 take(1) 因为有 let 语句
            return Ok(refnos);
        }
    }

    // 不需要名称过滤时
    let mut response = SUL_DB.query(&sql).bind(("mdb", processed_mdb)).await?;
    let refnos: Vec<RefnoEnum> = response.take(1)?; // 注意：这里应该是 take(1) 因为有 let 语句
    Ok(refnos)
}

/// 使用默认 MDB 配置查询指定类型的数据
///
/// 这是 `query_type_refnos_in_mdb` 的便捷包装器，自动使用 `DbOption` 中配置的 `mdb_name`。
///
/// # 参数
/// * `nouns` - 类型列表（例如 ["SITE", "ZONE", "BRAN"]）
/// * `db_type` - 数据库类型（1=DESI, 2=CATA, 3=PROP 等）
/// * `name_filter` - 可选的名称过滤条件
///
/// # 示例
/// ```rust
/// let filter = NameFilter::new("B", false);
/// let results = query_type_refnos(
///     &["BRAN"],
///     DBType::DESI,
///     Some(&filter)
/// ).await?;
/// ```
pub async fn query_type_refnos(
    nouns: &[&str],
    db_type: DBType,
    name_filter: Option<&NameFilter>,
) -> anyhow::Result<Vec<RefnoEnum>> {
    let mdb_name = &get_db_option().mdb_name;
    query_type_refnos_in_mdb(nouns, mdb_name, db_type, name_filter).await
}

/// 查询数据库中所有 MDB 的名称列表
///
/// # 功能说明
/// 从 SurrealDB 的 MDB 表中查询所有 MDB 的名称，返回去重后的名称列表。
/// 用于在项目设置界面提供 MDB 选择下拉框。
///
/// # 返回值
/// * `Vec<String>` - MDB 名称列表，按字母顺序排序，最多返回 100 个
///
/// # 示例
/// ```rust
/// let mdb_names = query_all_mdb_names().await?;
/// for name in mdb_names {
///     println!("MDB: {}", name);
/// }
/// ```
pub async fn query_all_mdb_names() -> anyhow::Result<Vec<String>> {
    let sql = "SELECT VALUE NAME FROM MDB WHERE NAME != NONE ORDER BY NAME LIMIT 100";
    let mut response = SUL_DB.query_response(&sql).await?;
    let names: Vec<String> = response.take(0)?;
    Ok(names)
}

/// 查询 MDB 的数据库编号列表
///
/// # 功能说明
/// 根据 MDB 名称和模块类型，查询该 MDB 下对应模块的所有数据库编号（DBNO）。
/// 一个 MDB 可能包含多个数据库，此函数返回指定模块类型的所有数据库编号。
///
/// # 参数
/// * `mdb` - MDB 名称（可选，为 None 时使用默认配置中的 MDB）
/// * `module` - 数据库模块类型（DESI=1, CATA=2, PROP=3 等）
///
/// # 返回值
/// * `Vec<u32>` - 数据库编号列表
///
/// # 缓存
/// 使用 `#[cached]` 宏缓存查询结果，提高重复查询性能
///
/// # 示例
/// ```rust
/// // 查询 /651YK 的 DESI 模块数据库编号
/// let dbnos = query_mdb_db_nums(Some("/651YK".to_string()), DBType::DESI).await?;
///
/// // 使用默认 MDB
/// let dbnos = query_mdb_db_nums(None, DBType::DESI).await?;
/// ```
#[cached(result = true)]
pub async fn query_mdb_db_nums(mdb: Option<String>, module: DBType) -> anyhow::Result<Vec<u32>> {
    let db_type: u8 = module.into();
    let mdb = mdb.unwrap_or_else(|| crate::get_db_option().mdb_name.clone());
    let processed_mdb = crate::helper::to_e3d_name(&mdb).into_owned();
    let sql = format!(
        " select value (select value DBNO from CURD.refno where STYP={db_type}) from only MDB where NAME='{processed_mdb}' limit 1"
    );
    println!("Executing SQL: {}", sql);
    let mut response = SUL_DB.query_response(&sql).await.unwrap();
    let pe: Vec<u32> = response.take(0)?;
    Ok(pe)
}

/// 查询 MDB 的 WORLD 下的所有 SITE PE 元素
///
/// # 功能说明
/// 获取指定 MDB 和模块下所有 WORLD 节点的直接 SITE 子节点的完整 PE 元素信息。
/// 与 `get_mdb_world_site_ele_nodes` 不同，本函数返回完整的 `SPdmsElement` 数据。
///
/// # 参数
/// * `mdb` - MDB 名称
/// * `module` - 数据库模块类型
///
/// # 返回值
/// * `Vec<SPdmsElement>` - SITE 类型的 PE 元素列表
///
/// # 缓存
/// 使用 `#[cached]` 宏缓存查询结果
///
/// # 实现细节
/// 1. 查询 MDB 下指定模块的数据库编号列表
/// 2. 找到这些数据库对应的 WORLD 节点
/// 3. 通过 pe_owner 关系反向查找 WORLD 的子节点
/// 4. 过滤出 noun = 'SITE' 的节点
#[cached(result = true)]
pub async fn get_mdb_world_site_pes(
    mdb: String,
    module: DBType,
) -> anyhow::Result<Vec<SPdmsElement>> {
    let db_type: u8 = module.into();
    let mdb_name = to_e3d_name(&mdb);
    let sql = format!(
        r#"
        let $dbnos = select value (select value DBNO from CURD.refno where STYP={db_type}) from only MDB where NAME='{mdb}' limit 1;
        let $a = (select value id from (select REFNO.id as id, array::find_index($dbnos, REFNO.dbnum) as o from WORL where REFNO.dbnum in $dbnos order by o));
        array::flatten(select value in.* from $a<-pe_owner)[?noun = 'SITE']
        "#,
        db_type = db_type,
        mdb = mdb_name
    );
    //
    let mut response = SUL_DB.query_response(&sql).await?;
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

/// 获取 WORLD 世界节点
///
/// # 功能说明
/// 查询指定 MDB 的 DESI 模块下的 WORLD 根节点。
/// WORLD 节点是整个层级结构的根，包含所有 SITE、ZONE 等子节点。
///
/// # 参数
/// * `mdb` - MDB 名称（会自动标准化为 E3D 格式）
///
/// # 返回值
/// * `Option<SPdmsElement>` - WORLD 节点元素（如果不存在则返回 None）
///
/// # 缓存
/// 使用 `#[cached]` 宏缓存查询结果
///
/// # 示例
/// ```rust
/// let world = get_world("/651YK".to_string()).await?;
/// if let Some(world_node) = world {
///     println!("WORLD 节点: {:?}", world_node.refno);
/// }
/// ```
#[cached(result = true)]
pub async fn get_world(mdb: String) -> anyhow::Result<Option<SPdmsElement>> {
    let mdb_name = to_e3d_name(&mdb);
    let sql = format!(
        " \
            let $f = (select value (select value DBNO from CURD.refno where STYP=1) from only MDB where NAME='{}' limit 1)[0]; \
            (select value REFNO.* from WORL where REFNO.dbnum=$f and REFNO.noun='WORL' limit 1)[0]",
        mdb_name
    );
    // println!("Executing SQL: {}", sql);
    let mut response = SUL_DB.query_response(&sql).await?;
    // dbg!(&response);
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
// #[cached(result = true)]
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
        "
            let $f = (select value (select value DBNO from CURD.refno where STYP=1) from only MDB where NAME='{}' limit 1)[0]; \
            (select value REFNO from WORL where REFNO.dbnum=$f and REFNO.noun='WORL' limit 1)[0]",
        mdb_name
    );

    println!("Executing SQL: {}", sql);

    // 执行查询并获取结果
    let id: Option<RefnoEnum> = SUL_DB.query_take(&sql, 1).await?;
    // dbg!(&id);
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
    async fn test_query_type_refnos_by_dbnum_with_name_filter() {
        init_test_surreal().await;

        let db_nums = query_mdb_db_nums(Some(get_db_option().mdb_name.clone()), DBType::DESI)
            .await
            .unwrap();
        if db_nums.is_empty() {
            println!("⚠️  没有可用的数据库编号，跳过测试");
            return;
        }

        let dbnum = db_nums[0];
        let nouns = &["BRAN", "PIPE", "SITE", "ZONE"];
        let baseline = match query_type_refnos_by_dbnum(nouns, dbnum, None, false).await {
            Ok(data) => data,
            Err(err) => {
                panic!("查询基础数据失败: {err}");
            }
        };

        if baseline.is_empty() {
            println!("⚠️  当前数据库下没有匹配的参考号，跳过测试");
            return;
        }

        let mut target_pe: Option<SPdmsElement> = None;
        for refno in &baseline {
            if let Ok(Some(pe)) = crate::rs_surreal::query::get_pe(*refno).await {
                if !pe.name.trim().is_empty() {
                    target_pe = Some(pe);
                    break;
                }
            }
        }

        let Some(target_pe) = target_pe else {
            println!("⚠️  未找到带名称的节点，跳过测试");
            return;
        };

        let target_refno = target_pe.refno;
        let noun = target_pe.noun.clone();
        let mut keyword: String = target_pe.name.chars().take(3).collect();
        if keyword.is_empty() {
            keyword = target_pe.name.clone();
        }

        if keyword.trim().is_empty() {
            println!("⚠️  目标节点名称为空，跳过测试");
            return;
        }

        let noun_refs = vec![noun.as_str()];

        let filter_cs = NameFilter::new(keyword.clone(), true);
        let result_cs = query_type_refnos_by_dbnum_with_filter(
            &noun_refs,
            dbnum,
            None,
            false,
            Some(&filter_cs),
        )
        .await
        .expect("名称过滤（区分大小写）执行失败");
        assert!(
            result_cs.contains(&target_refno),
            "名称过滤（区分大小写）应命中目标节点"
        );

        let filter_ci = NameFilter::new(keyword.to_lowercase(), false);
        let result_ci = query_type_refnos_by_dbnum_with_filter(
            &noun_refs,
            dbnum,
            None,
            false,
            Some(&filter_ci),
        )
        .await
        .expect("名称过滤（忽略大小写）执行失败");
        assert!(
            result_ci.contains(&target_refno),
            "名称过滤（忽略大小写）应命中目标节点"
        );
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

    #[tokio::test]
    async fn test_query_type_refnos_with_name_filter() {
        init_test_surreal().await;

        println!("🧪 测试 query_type_refnos - 查询 DESI 的 BRAN，名称包含 'B'");

        let filter = NameFilter::new("B", false);
        let result = query_type_refnos(&["BRAN"], DBType::DESI, Some(&filter)).await;

        if let Err(ref e) = result {
            println!("   ❌ 查询失败: {:?}", e);
        }
        assert!(result.is_ok(), "查询应该成功: {:?}", result.err());

        let refnos = result.unwrap();
        println!("   ✅ 查询到 {} 个 BRAN 节点（名称包含 'B'）", refnos.len());

        // 验证结果
        if !refnos.is_empty() {
            println!("   前3个参考号: {:?}", &refnos[..refnos.len().min(3)]);

            // 验证查询到的节点名称确实包含 'B'
            for refno in refnos.iter().take(5) {
                if let Ok(Some(pe)) = crate::rs_surreal::query::get_pe(*refno).await {
                    println!("      BRAN: noun={}, name={}", pe.noun, pe.name);
                    assert_eq!(pe.noun, "BRAN", "节点类型应为 BRAN");
                    assert!(
                        pe.name.to_lowercase().contains("b"),
                        "名称应包含字符 'B'（不区分大小写）"
                    );
                }
            }
        }
    }
}

/// 测试简单的数据库连接
///
/// # 功能说明
/// 执行一个简单的 SurrealDB 查询以验证数据库连接是否正常
///
/// # 返回值
/// - `Ok(())` - 连接成功
/// - `Err` - 连接失败或查询出错
///
/// # 示例
/// ```rust
/// test_simple_query().await?;
/// ```
pub async fn test_simple_query() -> anyhow::Result<()> {
    let mut response = SUL_DB.query_response("RETURN 1").await?;
    let result: Vec<i32> = response.take(0)?;
    println!("Simple query result: {:?}", result);
    Ok(())
}
