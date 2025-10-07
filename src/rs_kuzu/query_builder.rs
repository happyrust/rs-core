//! Kuzu 查询构建器
//!
//! 提供类型安全的 Cypher 查询构建功能

use crate::types::RefnoEnum;
use itertools::Itertools;

/// Kuzu 查询构建器 trait
pub trait KuzuQueryBuilder {
    /// 构建查询字符串
    fn build(&self) -> String;

    /// 添加 WHERE 条件
    fn with_condition(self, condition: &str) -> Self;

    /// 设置返回字段
    fn returns(self, fields: &[&str]) -> Self;
}

/// 层级查询构建器
#[derive(Debug, Clone)]
pub struct HierarchyQueryBuilder {
    refno: RefnoEnum,
    direction: TraversalDirection,
    min_depth: usize,
    max_depth: Option<usize>,
    noun_filter: Vec<String>,
    with_deleted: bool,
    return_fields: Vec<String>,
    extra_conditions: Vec<String>,
}

/// 遍历方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraversalDirection {
    /// 向下遍历 (子孙)
    Children,
    /// 向上遍历 (祖先)
    Ancestors,
}

impl HierarchyQueryBuilder {
    /// 创建新的层级查询构建器 - 查询子节点
    pub fn children(refno: RefnoEnum) -> Self {
        Self {
            refno,
            direction: TraversalDirection::Children,
            min_depth: 1,
            max_depth: Some(1),
            noun_filter: Vec::new(),
            with_deleted: false,
            return_fields: vec!["refno".to_string()],
            extra_conditions: Vec::new(),
        }
    }

    /// 创建新的层级查询构建器 - 查询祖先
    pub fn ancestors(refno: RefnoEnum) -> Self {
        Self {
            refno,
            direction: TraversalDirection::Ancestors,
            min_depth: 1,
            max_depth: None,
            noun_filter: Vec::new(),
            with_deleted: false,
            return_fields: vec!["refno".to_string()],
            extra_conditions: Vec::new(),
        }
    }

    /// 设置递归深度
    pub fn depth(mut self, min: usize, max: Option<usize>) -> Self {
        self.min_depth = min;
        self.max_depth = max;
        self
    }

    /// 设置单一深度
    pub fn single_depth(mut self, depth: usize) -> Self {
        self.min_depth = depth;
        self.max_depth = Some(depth);
        self
    }

    /// 无限深度递归
    pub fn unlimited_depth(mut self) -> Self {
        self.min_depth = 1;
        self.max_depth = None;
        self
    }

    /// 过滤特定 noun 类型
    pub fn filter_nouns(mut self, nouns: &[&str]) -> Self {
        self.noun_filter = nouns.iter().map(|s| s.to_string()).collect();
        self
    }

    /// 包含已删除的节点
    pub fn include_deleted(mut self, include: bool) -> Self {
        self.with_deleted = include;
        self
    }

    /// 设置返回字段
    pub fn return_fields(mut self, fields: &[&str]) -> Self {
        self.return_fields = fields.iter().map(|s| s.to_string()).collect();
        self
    }

    /// 添加额外条件
    pub fn add_condition(mut self, condition: String) -> Self {
        self.extra_conditions.push(condition);
        self
    }

    /// 构建查询字符串
    pub fn build(&self) -> String {
        let depth_spec = match (self.min_depth, self.max_depth) {
            (1, Some(1)) => "".to_string(),
            (min, Some(max)) if min == max => format!("*{}", min),
            (min, Some(max)) => format!("*{}..{}", min, max),
            (min, None) => format!("*{}..", min),
        };

        let (arrow, node_var) = match self.direction {
            TraversalDirection::Children => (
                format!("-[:OWNS{}]->", depth_spec),
                "descendant"
            ),
            TraversalDirection::Ancestors => (
                format!("<-[:OWNS{}]-", depth_spec),
                "ancestor"
            ),
        };

        let mut conditions = Vec::new();

        // 删除过滤
        if !self.with_deleted {
            conditions.push(format!("{}.deleted = false", node_var));
        }

        // noun 类型过滤
        if !self.noun_filter.is_empty() {
            let nouns = self.noun_filter.iter()
                .map(|n| format!("'{}'", n))
                .join(", ");
            conditions.push(format!("{}.noun IN [{}]", node_var, nouns));
        }

        // 额外条件
        conditions.extend(self.extra_conditions.iter().cloned());

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("\n WHERE {}", conditions.join("\n   AND "))
        };

        let return_fields = self.return_fields.iter()
            .map(|f| format!("{}.{}", node_var, f))
            .join(", ");

        let distinct = if self.max_depth.is_none() || self.max_depth.unwrap() > 1 {
            "DISTINCT "
        } else {
            ""
        };

        format!(
            "MATCH (start:PE {{refno: {}}}){arrow}({node_var}:PE){where_clause}\nRETURN {distinct}{return_fields}",
            self.refno.refno().0,
            arrow = arrow,
            node_var = node_var,
            where_clause = where_clause,
            distinct = distinct,
            return_fields = return_fields
        )
    }
}

/// 类型过滤查询构建器
#[derive(Debug, Clone)]
pub struct TypeFilterQueryBuilder {
    dbnum: Option<u32>,
    dbnums: Vec<u32>,
    nouns: Vec<String>,
    has_children: Option<bool>,
    with_deleted: bool,
    return_fields: Vec<String>,
    extra_conditions: Vec<String>,
    limit: Option<usize>,
}

impl TypeFilterQueryBuilder {
    /// 创建新的类型过滤查询构建器
    pub fn new() -> Self {
        Self {
            dbnum: None,
            dbnums: Vec::new(),
            nouns: Vec::new(),
            has_children: None,
            with_deleted: false,
            return_fields: vec!["refno".to_string()],
            extra_conditions: Vec::new(),
            limit: None,
        }
    }

    /// 设置单一 dbnum
    pub fn dbnum(mut self, dbnum: u32) -> Self {
        self.dbnum = Some(dbnum);
        self
    }

    /// 设置多个 dbnum
    pub fn dbnums(mut self, dbnums: &[u32]) -> Self {
        self.dbnums = dbnums.to_vec();
        self
    }

    /// 设置 noun 类型过滤
    pub fn nouns(mut self, nouns: &[&str]) -> Self {
        self.nouns = nouns.iter().map(|s| s.to_string()).collect();
        self
    }

    /// 过滤是否有子节点
    pub fn with_children(mut self, has: Option<bool>) -> Self {
        self.has_children = has;
        self
    }

    /// 包含已删除的节点
    pub fn include_deleted(mut self, include: bool) -> Self {
        self.with_deleted = include;
        self
    }

    /// 设置返回字段
    pub fn return_fields(mut self, fields: &[&str]) -> Self {
        self.return_fields = fields.iter().map(|s| s.to_string()).collect();
        self
    }

    /// 添加额外条件
    pub fn add_condition(mut self, condition: String) -> Self {
        self.extra_conditions.push(condition);
        self
    }

    /// 设置结果数量限制
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// 构建查询字符串
    pub fn build(&self) -> String {
        let mut conditions = Vec::new();

        // dbnum 过滤
        if let Some(dbnum) = self.dbnum {
            conditions.push(format!("p.dbnum = {}", dbnum));
        } else if !self.dbnums.is_empty() {
            let dbnums = self.dbnums.iter().map(|d| d.to_string()).join(", ");
            conditions.push(format!("p.dbnum IN [{}]", dbnums));
        }

        // noun 类型过滤
        if !self.nouns.is_empty() {
            let nouns = self.nouns.iter()
                .map(|n| format!("'{}'", n))
                .join(", ");
            conditions.push(format!("p.noun IN [{}]", nouns));
        }

        // 删除过滤
        if !self.with_deleted {
            conditions.push("p.deleted = false".to_string());
        }

        // has_children 过滤
        if let Some(has_children) = self.has_children {
            if has_children {
                conditions.push("EXISTS { MATCH (p)-[:OWNS]->() }".to_string());
            } else {
                conditions.push("NOT EXISTS { MATCH (p)-[:OWNS]->() }".to_string());
            }
        }

        // 额外条件
        conditions.extend(self.extra_conditions.iter().cloned());

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("\n WHERE {}", conditions.join("\n   AND "))
        };

        let return_fields = self.return_fields.iter()
            .map(|f| format!("p.{}", f))
            .join(", ");

        let limit_clause = self.limit
            .map(|l| format!("\n LIMIT {}", l))
            .unwrap_or_default();

        format!(
            "MATCH (p:PE){where_clause}\nRETURN {return_fields}{limit_clause}",
            where_clause = where_clause,
            return_fields = return_fields,
            limit_clause = limit_clause
        )
    }
}

impl Default for TypeFilterQueryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::RefU64;

    #[test]
    fn test_children_query_builder() {
        let refno = RefnoEnum::from(RefU64(123));
        let query = HierarchyQueryBuilder::children(refno)
            .build();

        assert!(query.contains("MATCH (start:PE {refno: 123})"));
        assert!(query.contains("-[:OWNS]->"));
        assert!(query.contains("descendant.deleted = false"));
    }

    #[test]
    fn test_deep_children_query_builder() {
        let refno = RefnoEnum::from(RefU64(123));
        let query = HierarchyQueryBuilder::children(refno)
            .depth(1, Some(12))
            .filter_nouns(&["PIPE", "EQUI"])
            .build();

        assert!(query.contains("-[:OWNS*1..12]->"));
        assert!(query.contains("noun IN ['PIPE', 'EQUI']"));
        assert!(query.contains("DISTINCT"));
    }

    #[test]
    fn test_ancestors_query_builder() {
        let refno = RefnoEnum::from(RefU64(123));
        let query = HierarchyQueryBuilder::ancestors(refno)
            .unlimited_depth()
            .build();

        assert!(query.contains("MATCH (start:PE {refno: 123})"));
        assert!(query.contains("<-[:OWNS*1..-"));
        assert!(query.contains("DISTINCT"));
    }

    #[test]
    fn test_type_filter_query_builder() {
        let query = TypeFilterQueryBuilder::new()
            .dbnum(1112)
            .nouns(&["PIPE", "EQUI"])
            .with_children(Some(true))
            .limit(100)
            .build();

        assert!(query.contains("p.dbnum = 1112"));
        assert!(query.contains("p.noun IN ['PIPE', 'EQUI']"));
        assert!(query.contains("EXISTS { MATCH (p)-[:OWNS]->() }"));
        assert!(query.contains("LIMIT 100"));
    }
}
