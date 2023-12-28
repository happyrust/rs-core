pub mod test_attrmap;
pub mod test_shape;

pub mod test_schema;

// pub mod test_sql;
pub mod test_sync2;

pub mod test_refno;

pub mod test_rotation;

pub mod test_hash;

pub mod test_serde;

pub mod test_spatial_caculation;


#[cfg(not(target_arch = "wasm32"))]
pub mod test_surreal;
