use crate::shape::pdms_shape::PdmsMesh;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct NegativeEles {
    pub _key: String,
    pub mesh: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NegativeEdges {
    pub _key: String,
    pub _from: String,
    pub _to: String,
}