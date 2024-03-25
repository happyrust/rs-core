use crate::{AttrMap, NamedAttrMap, RefU64};
use crate::pdms_types::{EleTreeNode, PdmsElement};
use async_trait::async_trait;

pub mod aios_mgr;

#[async_trait]
pub trait PdmsDataInterface {
    /// 获取 world 节点
    async fn get_world(&self, mdb_name: &str) -> anyhow::Result<Option<PdmsElement>>;
    ///获得属性
    async fn get_named_attr(&self, refno: RefU64) -> anyhow::Result<NamedAttrMap>;
    /// 获取children
    async fn get_children(&self, refno:RefU64) -> anyhow::Result<Vec<EleTreeNode>>;
}