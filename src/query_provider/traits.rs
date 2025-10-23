//! 统一查询接口 Trait 定义

use super::error::QueryResult;
use crate::RefnoEnum;
use crate::types::{NamedAttrMap as NamedAttMap, SPdmsElement as PE};
use async_trait::async_trait;

/// 层级查询接口
///
/// 定义了所有与层级关系相关的查询方法
#[async_trait]
pub trait HierarchyQuery: Send + Sync {
    /// 获取直接子节点
    ///
    /// # 参数
    /// - `refno`: 父节点的 refno
    ///
    /// # 返回
    /// 子节点的 refno 列表
    async fn get_children(&self, refno: RefnoEnum) -> QueryResult<Vec<RefnoEnum>>;

    /// 查询深层子孙节点（递归）
    ///
    /// # 参数
    /// - `refno`: 根节点的 refno
    /// - `max_depth`: 最大递归深度，None 表示不限制
    ///
    /// # 返回
    /// 所有子孙节点的 refno 列表
    async fn get_descendants(
        &self,
        refno: RefnoEnum,
        max_depth: Option<usize>,
    ) -> QueryResult<Vec<RefnoEnum>>;

    /// 查询祖先节点
    ///
    /// # 参数
    /// - `refno`: 子节点的 refno
    ///
    /// # 返回
    /// 所有祖先节点的 refno 列表（从直接父节点到根节点）
    async fn get_ancestors(&self, refno: RefnoEnum) -> QueryResult<Vec<RefnoEnum>>;

    /// 查询特定类型的祖先
    ///
    /// # 参数
    /// - `refno`: 子节点的 refno
    /// - `nouns`: 要查询的类型列表
    ///
    /// # 返回
    /// 匹配类型的祖先节点 refno 列表
    async fn get_ancestors_of_type(
        &self,
        refno: RefnoEnum,
        nouns: &[&str],
    ) -> QueryResult<Vec<RefnoEnum>>;

    /// 查询过滤后的深层子孙
    ///
    /// # 参数
    /// - `refno`: 根节点的 refno
    /// - `nouns`: 要过滤的类型列表
    /// - `max_depth`: 最大递归深度
    ///
    /// # 返回
    /// 匹配类型的子孙节点 refno 列表
    async fn get_descendants_filtered(
        &self,
        refno: RefnoEnum,
        nouns: &[&str],
        max_depth: Option<usize>,
    ) -> QueryResult<Vec<RefnoEnum>>;

    /// 获取子节点的完整 PE 信息
    ///
    /// # 参数
    /// - `refno`: 父节点的 refno
    ///
    /// # 返回
    /// 子节点的完整 PE 列表
    async fn get_children_pes(&self, refno: RefnoEnum) -> QueryResult<Vec<PE>>;
}

/// 类型过滤查询接口
///
/// 定义了基于类型（noun）的查询方法
#[async_trait]
pub trait TypeQuery: Send + Sync {
    /// 按类型和数据库编号查询
    ///
    /// # 参数
    /// - `nouns`: 类型列表（如 ["PIPE", "ELBO"]）
    /// - `dbnum`: 数据库编号
    /// - `has_children`: 是否过滤有子节点的元素
    ///   - `Some(true)`: 只返回有子节点的
    ///   - `Some(false)`: 只返回没有子节点的
    ///   - `None`: 不过滤
    ///
    /// # 返回
    /// 匹配条件的 refno 列表
    async fn query_by_type(
        &self,
        nouns: &[&str],
        dbnum: i32,
        has_children: Option<bool>,
    ) -> QueryResult<Vec<RefnoEnum>>;

    /// 按类型和多个数据库编号查询
    ///
    /// # 参数
    /// - `nouns`: 类型列表
    /// - `dbnums`: 数据库编号列表
    ///
    /// # 返回
    /// 匹配条件的 refno 列表
    async fn query_by_type_multi_db(
        &self,
        nouns: &[&str],
        dbnums: &[i32],
    ) -> QueryResult<Vec<RefnoEnum>>;

    /// 获取指定数据库的 World 节点
    ///
    /// # 参数
    /// - `dbnum`: 数据库编号
    ///
    /// # 返回
    /// World 节点的 refno，如果不存在返回 None
    async fn get_world(&self, dbnum: i32) -> QueryResult<Option<RefnoEnum>>;

    /// 获取指定数据库的所有 Site 节点
    ///
    /// # 参数
    /// - `dbnum`: 数据库编号
    ///
    /// # 返回
    /// Site 节点的 refno 列表
    async fn get_sites(&self, dbnum: i32) -> QueryResult<Vec<RefnoEnum>>;

    /// 统计指定类型的元素数量
    ///
    /// # 参数
    /// - `noun`: 类型名称
    /// - `dbnum`: 数据库编号
    ///
    /// # 返回
    /// 元素数量
    async fn count_by_type(&self, noun: &str, dbnum: i32) -> QueryResult<usize>;
}

/// 批量查询接口
///
/// 定义了批量操作的查询方法
#[async_trait]
pub trait BatchQuery: Send + Sync {
    /// 批量获取 PE 信息
    ///
    /// # 参数
    /// - `refnos`: refno 列表
    ///
    /// # 返回
    /// PE 列表（保持顺序，如果某个 refno 不存在则跳过）
    async fn get_pes_batch(&self, refnos: &[RefnoEnum]) -> QueryResult<Vec<PE>>;

    /// 批量获取属性映射
    ///
    /// # 参数
    /// - `refnos`: refno 列表
    ///
    /// # 返回
    /// NamedAttMap 列表
    async fn get_attmaps_batch(&self, refnos: &[RefnoEnum]) -> QueryResult<Vec<NamedAttMap>>;

    /// 批量查询全名
    ///
    /// # 参数
    /// - `refnos`: refno 列表
    ///
    /// # 返回
    /// (refno, full_name) 元组列表
    async fn get_full_names_batch(
        &self,
        refnos: &[RefnoEnum],
    ) -> QueryResult<Vec<(RefnoEnum, String)>>;
}

/// 图遍历查询接口
///
/// 定义了复杂的图遍历查询方法
#[async_trait]
pub trait GraphQuery: Send + Sync {
    /// 多起点、多类型的深层子孙查询
    ///
    /// # 参数
    /// - `refnos`: 起点节点列表
    /// - `nouns`: 要过滤的类型列表
    ///
    /// # 返回
    /// 匹配条件的 refno 列表
    async fn query_multi_descendants(
        &self,
        refnos: &[RefnoEnum],
        nouns: &[&str],
    ) -> QueryResult<Vec<RefnoEnum>>;

    /// 查询两个节点之间的最短路径
    ///
    /// # 参数
    /// - `from`: 起点 refno
    /// - `to`: 终点 refno
    ///
    /// # 返回
    /// 路径上的 refno 列表（包含起点和终点），如果不可达返回空列表
    async fn find_shortest_path(
        &self,
        from: RefnoEnum,
        to: RefnoEnum,
    ) -> QueryResult<Vec<RefnoEnum>>;

    /// 查询节点的深度（到根节点的距离）
    ///
    /// # 参数
    /// - `refno`: 节点 refno
    ///
    /// # 返回
    /// 深度值，根节点为 0
    async fn get_node_depth(&self, refno: RefnoEnum) -> QueryResult<usize>;
}

/// 统一查询提供者接口
///
/// 组合了所有查询接口，提供完整的查询能力
#[async_trait]
pub trait QueryProvider:
    HierarchyQuery + TypeQuery + BatchQuery + GraphQuery + Send + Sync
{
    /// 获取单个 PE 信息
    ///
    /// # 参数
    /// - `refno`: PE 的 refno
    ///
    /// # 返回
    /// PE 信息，如果不存在返回 None
    async fn get_pe(&self, refno: RefnoEnum) -> QueryResult<Option<PE>>;

    /// 获取 PE 的属性映射
    ///
    /// # 参数
    /// - `refno`: PE 的 refno
    ///
    /// # 返回
    /// 属性映射，如果不存在返回 None
    async fn get_attmap(&self, refno: RefnoEnum) -> QueryResult<Option<NamedAttMap>>;

    /// 检查 PE 是否存在
    ///
    /// # 参数
    /// - `refno`: PE 的 refno
    ///
    /// # 返回
    /// 如果存在返回 true
    async fn exists(&self, refno: RefnoEnum) -> QueryResult<bool>;

    /// 获取查询提供者的名称（用于调试和日志）
    fn provider_name(&self) -> &str;

    /// 健康检查
    ///
    /// # 返回
    /// 如果数据库连接正常返回 true
    async fn health_check(&self) -> QueryResult<bool>;
}
