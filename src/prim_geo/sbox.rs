use std::f32::EPSILON;
use bevy::prelude::*;
use truck_base::cgmath64::Vector3;
use truck_meshalgo::prelude::{MeshableShape, MeshedShape};
use truck_modeling::{builder, Shell, Solid};
use truck_polymesh::stl::IntoSTLIterator;
// use bevy_inspector_egui::Inspectable;
use bevy::reflect::Reflect;
use bevy::ecs::reflect::ReflectComponent;
use serde::{Serialize,Deserialize};
use crate::pdms_types::AttrMap;
use crate::prim_geo::helper::quad_indices;
use crate::shape::pdms_shape::{BrepMathTrait, BrepShapeTrait, PdmsMesh, VerifiedShape};

#[derive(Component, Debug, /*Inspectable, Reflect,*/ Clone, Serialize, Deserialize)]
// #[reflect(Component)]
pub struct SBox {
    pub center: Vec3,
    pub size: Vec3,
}

impl Default for SBox {
    fn default() -> Self {
        SBox {
            center: Default::default(),
            size: Vec3::new(1.0, 1.0, 1.0),
        }
    }
}

impl VerifiedShape for SBox {
    #[inline]
    fn check_valid(&self) -> bool {
       self.size.x > f32::EPSILON &&  self.size.y > f32::EPSILON && self.size.z > f32::EPSILON
    }
}

impl BrepShapeTrait for SBox {
    fn gen_brep_shell(& self) -> Option<Shell> {
        if !self.check_valid() { return None; }
        let v = builder::vertex((self.center - self.size / 2.0).point3());
        let e = builder::tsweep(&v, Vector3::unit_x() * self.size.x as f64);
        let f = builder::tsweep(&e, Vector3::unit_y() * self.size.y as f64);
        let mut s = builder::tsweep(&f, Vector3::unit_z() * self.size.z as f64).into_boundaries();
        s.pop()
    }

    fn hash_mesh_params(&self) -> u64{
        1u64            //代表BOX
    }

    fn gen_unit_shape(&self) -> PdmsMesh{
        SBox::default().gen_mesh(None)
    }

    #[inline]
    fn get_scaled_vec3(&self) -> Vec3 {
        self.size
    }
}


impl From<&AttrMap> for SBox {
    fn from(m: &AttrMap) -> Self {
        SBox {
            center: Default::default(),
            size: Vec3::new(m.get_f32("XLEN").unwrap_or_default(),
                            m.get_f32("YLEN").unwrap_or_default(),
                            m.get_f32("ZLEN").unwrap_or_default(),),
        }
    }
}

impl From<AttrMap> for SBox {
    fn from(m: AttrMap) -> Self {
        (&m).into()
    }
}



