pub mod helper;
pub mod sbox;
pub mod sphere;
pub mod cylinder;
pub mod snout;
pub mod dish;
pub mod ctorus;
pub mod extrusion;
pub mod revolution;
pub mod pyramid;
pub mod lpyramid;
pub mod rtorus;
pub mod facet;
pub mod sweep_solid;
pub mod tubing;
pub mod polyhedron;

pub mod category;
pub mod spine;

pub mod wire;

pub mod line;

pub const CUBE_GEO_HASH: u64 = 1u64;
pub const CYLINDER_GEO_HASH: u64 = 2u64;
pub const TUBI_GEO_HASH: u64 = 2u64;
pub const BOXI_GEO_HASH: u64 = 1u64;
pub const SPHERE_GEO_HASH: u64 = 3u64;

