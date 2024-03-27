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
    async fn get_children(&self, refno: RefU64) -> anyhow::Result<Vec<EleTreeNode>>;
    /// 获取bran的ipara
    async fn get_ipara_from_bran(&self, refno: RefU64) -> anyhow::Result<Vec<f32>>;
    /// 通过name获取PdmsElement
    async fn get_ele_from_name(&self, name: &str) -> anyhow::Result<Option<PdmsElement>>;
    /// 获取spre的属性
    async fn get_spre_attr(&self, refno: RefU64) -> anyhow::Result<Option<NamedAttrMap>>;
    /// 获取catr的属性
    async fn get_catr_attr(&self, refno: RefU64) -> anyhow::Result<Option<NamedAttrMap>>;
    /// 获取外键(一层)的属性
    /// catr这种就为两层
    async fn get_foreign_attr(&self, refno: RefU64, foreign_type: &str) -> anyhow::Result<Option<NamedAttrMap>>;
}