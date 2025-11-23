use crate::{NamedAttrMap, RefnoEnum};
use async_trait::async_trait;
use glam::DMat4;

#[async_trait]
pub trait TransformStrategy: Send + Sync {
    async fn get_local_transform(
        &self,
        refno: RefnoEnum,
        parent_refno: RefnoEnum,
        att: &NamedAttrMap,
        parent_att: &NamedAttrMap,
    ) -> anyhow::Result<Option<DMat4>>;
}

pub mod default;
pub mod endatu;
pub mod endatu_error;
pub mod endatu_cache;
pub mod endatu_validation;
pub mod gensec;
pub mod sjoi;

// 导出策略
use default::DefaultStrategy;
use gensec::GensecStrategy;
use sjoi::SjoiStrategy;

// 导出属性处理器
pub use default::{ZdisHandler, PoslHandler, YdirHandler, BangHandler, CutpHandler};
pub use endatu::EndAtuZdisHandler;
pub use endatu::EndAtuStrategy;
pub use endatu_error::{EndatuError, EndatuResult};
pub use endatu_cache::{get_cached_endatu_index, get_cache_stats, clear_endatu_cache, print_cache_stats};
pub use endatu_validation::EndatuValidator;
pub use gensec::{GensecBangHandler, GensecExtrusionHandler};
pub use sjoi::{SjoiCrefHandler, SjoiConnectionHandler};

pub struct TransformStrategyFactory;

impl TransformStrategyFactory {
    pub fn get_strategy(noun: &str) -> Box<dyn TransformStrategy> {
        match noun {
            "GENSEC" => Box::new(GensecStrategy),
            "SJOI" => Box::new(SjoiStrategy),
            "ENDATU" => Box::new(EndAtuStrategy),
            // FITT, SCOJ and others fall back to DefaultStrategy which handles POSL/PLIN
            _ => Box::new(DefaultStrategy),
        }
    }
}
