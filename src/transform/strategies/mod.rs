use crate::{NamedAttrMap, RefnoEnum};
use async_trait::async_trait;
use glam::{DMat4, DVec3, DQuat};

#[async_trait]
pub trait TransformStrategy: Send + Sync {
    async fn get_local_transform(
        &mut self,
    ) -> anyhow::Result<Option<DMat4>>;
}

pub mod default;
pub mod sweep_strategy;
// pub mod endatu;
// pub mod endatu_cache;
// pub mod endatu_error;
// pub mod endatu_validation;
pub mod spine_strategy;
// pub mod sjoi;

// 导出策略
pub use default::{DefaultStrategy};
pub use sweep_strategy::SweepStrategy;
use spine_strategy::SpineStrategy;
// use sjoi::SjoiStrategy;

// 导出属性处理器
pub use default::{CutpHandler, PoslHandler, YdirHandler, ZdisHandler};
// pub use endatu::EndAtuStrategy;
// pub use endatu::EndAtuZdisHandler;
// pub use endatu_cache::{
//     clear_endatu_cache, get_cache_stats, get_cached_endatu_index, print_cache_stats,
// };
// pub use endatu_error::{EndatuError, EndatuResult};
// pub use endatu_validation::EndatuValidator;
// pub use spine_strategy::get_spline_path;
// pub use sjoi::{SjoiConnectionHandler, SjoiCrefHandler};

pub struct TransformStrategyFactory;

impl TransformStrategyFactory {
    pub fn get_strategy(att: &NamedAttrMap, parent_att: &NamedAttrMap) -> Box<dyn TransformStrategy> {
        // 基于父节点类型进行策略分发
        match parent_att.get_type_str() {
            "SPINE" => Box::new(SpineStrategy::new(att.clone(), parent_att.clone())),
            "GENSEC" => Box::new(SweepStrategy::new(att.clone(), parent_att.clone())),
            // "STWALL" => Box::new(WallStrategy),
            // "SJOI" => Box::new(SjoiStrategy),
            // "ENDATU" => Box::new(EndAtuStrategy),
            // "STWALL" | "FITT" => Box::new(ComplexStrategy),
            // 父节点不是特殊类型，使用默认策略（仅POS+ORI）
            _ => Box::new(DefaultStrategy::new(att.clone())),
        }
    }
}

/// NPOS 属性处理的公共函数
pub struct NposHandler;

impl NposHandler {
    /// 应用 NPOS 偏移，使用容错处理
    /// 
    /// 适用于大多数策略（如 default、gensec、sjoi），当 NPOS 属性不存在或无效时
    /// 使用默认值 (0,0,0)，不会中断变换计算流程。
    pub fn apply_npos_offset(pos: &mut DVec3, att: &NamedAttrMap) {
        if att.contains_key("NPOS") {
            let npos = att.get_vec3("NPOS").unwrap_or_default();
            *pos += npos.as_dvec3();
        }
    }
    
    /// 严格应用 NPOS 偏移，返回 anyhow 错误
    /// 
    /// 适用于大多数策略，当 NPOS 属性存在但无效时返回 anyhow 错误
    pub fn try_apply_npos_offset(pos: &mut DVec3, att: &NamedAttrMap) -> anyhow::Result<()> {
        if att.contains_key("NPOS") {
            let npos = att.get_vec3("NPOS").ok_or_else(|| {
                anyhow::anyhow!("NPOS 属性存在但无法解析")
            })?;
            *pos += npos.as_dvec3();
        }
        Ok(())
    }
    
}

/// BANG 属性处理的公共函数
pub struct BangHandler;

impl BangHandler {

    /// 应用 BANG 旋转到四元数
    /// 
    /// 统一的 BANG 应用逻辑，沿 Z 轴旋转指定角度
    pub fn apply_bang(quat: &mut DQuat, att: &NamedAttrMap) {
        if let Some(bangle) = att.get_f32("BANG").map(|x| x as f64) {
            *quat = *quat * DQuat::from_rotation_z(bangle.to_radians());
        }
    }

}



