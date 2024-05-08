pub mod test_attmap;
pub mod test_shape;

#[cfg(feature = "occ")]
pub mod test_wire;

pub mod test_schema;


pub mod test_refno;

pub mod test_rotation;

pub mod test_hash;

pub mod test_serde;

pub mod test_spatial_caculation;


#[cfg(not(target_arch = "wasm32"))]
pub mod test_surreal;
