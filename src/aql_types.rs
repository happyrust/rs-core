use serde_derive::{Deserialize, Serialize};

/// 只存储 _from 和 _to 的信息 不加其他边界信息
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct AqlEdge {
    pub _key: String,
    pub _from: String,
    pub _to: String,
}