pub mod test_attmap;
pub mod test_rsvec3_conversion;
pub mod test_shape;
pub mod test_uv_generation;

#[cfg(feature = "occ")]
pub mod test_wire;

pub mod test_schema;

pub mod test_refno;

pub mod test_rotation;

pub mod test_hash;

pub mod test_serde;

pub mod test_spatial_calculation;

pub mod test_parse_dir;

#[cfg(not(target_arch = "wasm32"))]
pub mod test_surreal;

pub mod test_query_provider;

// pub mod test_db_adapter;

pub mod test_expression;

pub mod test_different_dbs;

// pub mod test_transform;

// pub mod test_material;

// #[cfg(not(target_arch = "wasm32"))]
// pub mod test_gensec_spine;

pub mod test_h_beam_drns_drne;
pub mod test_multi_segment_path;
pub mod test_scylinder_shear;
pub mod test_sweep_orientation;

#[cfg(not(target_arch = "wasm32"))]
pub mod test_svg_standalone;

#[cfg(not(target_arch = "wasm32"))]
pub mod test_arc_demo;

#[cfg(not(target_arch = "wasm32"))]
pub mod test_clean_svg;
