use crate::geometry::csg::{unit_box_mesh, unit_cylinder_mesh, unit_sphere_mesh};
use crate::mesh_precision::LodMeshSettings;
use crate::shape::pdms_shape::PlantMesh;
use derive_more::{Deref, DerefMut};
use glam::{DMat4, Vec3};
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::Arc;

lazy_static! {
    pub static ref BOX_SHAPE: CsgSharedMesh = CsgSharedMesh::new(unit_box_mesh());
    pub static ref SPHERE_SHAPE: CsgSharedMesh = CsgSharedMesh::new(unit_sphere_mesh());
    pub static ref CYLINDER_SHAPE: CsgSharedMesh =
        CsgSharedMesh::new(unit_cylinder_mesh(&LodMeshSettings::default(), false));
    pub static ref BASIC_PRIM_SHAPE_MAP: HashMap<u64, CsgSharedMesh> = {
        let mut s = HashMap::new();
        s.insert(BOXI_GEO_HASH, BOX_SHAPE.clone());
        s.insert(CYLINDER_GEO_HASH, CYLINDER_SHAPE.clone());
        s.insert(SPHERE_GEO_HASH, SPHERE_SHAPE.clone());
        s
    };
}

#[derive(Clone, Deref, DerefMut)]
pub struct CsgSharedMesh(pub Arc<PlantMesh>);

impl CsgSharedMesh {
    #[inline]
    pub fn new(mesh: PlantMesh) -> Self {
        CsgSharedMesh(Arc::new(mesh))
    }

    #[inline]
    pub fn transformed(&self, m: &DMat4) -> anyhow::Result<Self> {
        let transformed_mesh = self.0.transform_by(m);
        Ok(CsgSharedMesh::new(transformed_mesh))
    }
}

impl AsRef<PlantMesh> for CsgSharedMesh {
    fn as_ref(&self) -> &PlantMesh {
        &self.0
    }
}

impl AsMut<PlantMesh> for CsgSharedMesh {
    fn as_mut(&mut self) -> &mut PlantMesh {
        Arc::get_mut(&mut self.0).unwrap()
    }
}

impl From<PlantMesh> for CsgSharedMesh {
    fn from(m: PlantMesh) -> Self {
        CsgSharedMesh::new(m)
    }
}

#[cfg(feature = "occ")]
#[derive(Clone, Deref, DerefMut)]
pub struct OccSharedShape(pub Arc<Shape>);

#[cfg(feature = "occ")]
impl OccSharedShape {
    #[inline]
    pub fn new(shape: Shape) -> Self {
        OccSharedShape(Arc::new(shape))
    }

    #[inline]
    pub fn transformed(&self, m: &DMat4) -> anyhow::Result<Self> {
        let s = self.0.transformed_by_gmat(m)?;
        Ok(OccSharedShape::new(s))
    }
}

#[cfg(feature = "occ")]
impl AsRef<Shape> for OccSharedShape {
    fn as_ref(&self) -> &Shape {
        &self.0
    }
}

#[cfg(feature = "occ")]
impl AsMut<Shape> for OccSharedShape {
    fn as_mut(&mut self) -> &mut Shape {
        Arc::get_mut(&mut self.0).unwrap()
    }
}

#[cfg(feature = "occ")]
impl From<Shape> for OccSharedShape {
    fn from(s: Shape) -> Self {
        OccSharedShape::new(s)
    }
}

pub const BOX_GEO_HASH: u64 = 1u64;
pub const CYLINDER_GEO_HASH: u64 = 2u64;
pub const TUBI_GEO_HASH: u64 = 2u64;
pub const BOXI_GEO_HASH: u64 = 1u64;
pub const SPHERE_GEO_HASH: u64 = 3u64;
