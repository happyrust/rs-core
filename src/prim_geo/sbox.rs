use std::f32::EPSILON;
use bevy::prelude::*;
use truck_base::cgmath64::Vector3;
use truck_meshalgo::prelude::{MeshableShape, MeshedShape};
use truck_modeling::{builder, Shell, Solid};
use bevy::reflect::Reflect;
use bevy::ecs::reflect::ReflectComponent;
use serde::{Serialize, Deserialize};
use crate::consts::BOX_HASH;
use crate::parsed_data::CateBoxParam;
use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::pdms_types::AttrMap;
use crate::prim_geo::CUBE_GEO_HASH;
#[cfg(feature = "opencascade")]
use opencascade::OCCShape;
use crate::shape::pdms_shape::{BrepMathTrait,  PlantMesh, VerifiedShape};

#[derive(Component, Debug, Clone, Serialize, Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, )]
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
        self.size.x > f32::EPSILON && self.size.y > f32::EPSILON && self.size.z > f32::EPSILON
    }
}

impl From<&AttrMap> for SBox {
    fn from(m: &AttrMap) -> Self {
        SBox {
            center: Default::default(),
            size: Vec3::new(m.get_f32("XLEN").unwrap_or_default(),
                            m.get_f32("YLEN").unwrap_or_default(),
                            m.get_f32("ZLEN").unwrap_or_default(), ),
        }
    }
}



