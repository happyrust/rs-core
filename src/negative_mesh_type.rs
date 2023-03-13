use crate::shape::pdms_shape::PdmsMesh;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NegativeEles {
    pub _key: String,
    pub mesh: PdmsMesh,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NegativeEdges {
    pub _key: String,
    pub _from: String,
    pub _to: String,
}