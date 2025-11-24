/// SWEEP 策略实现模块

use super::{TransformStrategy, NposHandler, BangHandler};
use super::spine_strategy::SpineStrategy;
use crate::rs_surreal::spatial::{
    construct_basis_z_opdir, construct_basis_z_y_exact, construct_basis_z_ref_y,
    construct_basis_z_y_hint,
};
use crate::prim_geo::spine::{Spine3D, SpineCurveType};
use crate::{NamedAttrMap, RefnoEnum, get_type_name, get_children_refnos, get_named_attmap, get_children_named_attmaps};
use async_trait::async_trait;
use glam::{DMat4, DQuat, DVec3, Vec3};

pub struct SweepStrategy {
    att: NamedAttrMap,
    parent_att: NamedAttrMap,
}

impl SweepStrategy {
    pub fn new(att: NamedAttrMap, parent_att: NamedAttrMap) -> Self {
        Self { att, parent_att }
    }

    /// 处理 GENSEC 的特殊挤出方向逻辑
    pub async fn extract_sweep_extrusion(
        parent_refno: RefnoEnum,
        att: &NamedAttrMap,
        parent_att: &NamedAttrMap,
    ) -> anyhow::Result<(Option<DVec3>, Option<DVec3>)> {
        // TODO: 实现 SWEEP 特有的挤出方向逻辑
        Ok((None, None))
    }
}

#[async_trait]
impl TransformStrategy for SweepStrategy {
    async fn get_local_transform(
        &mut self,
    ) -> anyhow::Result<Option<DMat4>> {
        // 获取所有需要的数据
        let att = &self.att;
        // 获取父节点引用号和属性
        let gensec_att = &self.parent_att;
        let gensec_refno = gensec_att.get_refno().unwrap();
        
        let cur_type = att.get_type_str();
        let pkdi = att.get_f64("PKDI").unwrap_or_default();
        let zdis = att.get_f64("ZDIS").unwrap_or_default();
        match cur_type {
            "JLDATU" => {
                // JLDATU 使用标准的 ZDIS/PKDI 计算
                // SpineStrategy 需要是有状态的，但我们这里只需要它计算一次
                let mut strategy = SpineStrategy::from_gensec(gensec_refno).await?;
                let transform = strategy.cal_trans_by_pkdi_zdis(pkdi, zdis).await
                    .ok_or_else(|| anyhow::anyhow!("Failed to calculate transform"))?;
                
                return Ok(Some(transform));
            }
            _ => {
                return Ok(None);
            }
        }
    }
}    
        

/// 检查是否为虚拟节点
fn is_virtual_node(cur_type: &str) -> bool {
    // matches!(cur_type, "SPINE" | "CURVE" | "POINSP")
    false
}
