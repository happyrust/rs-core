use crate::prim_geo::ctorus::CTorus;
use crate::prim_geo::cylinder::SCylinder;
use crate::prim_geo::dish::Dish;
use crate::prim_geo::pyramid::Pyramid;
use crate::prim_geo::rtorus::RTorus;
use crate::prim_geo::sbox::SBox;
use crate::prim_geo::snout::LSnout;
use crate::prim_geo::sphere::Sphere;
use crate::shape::pdms_shape::BrepShapeTrait;
use crate::types::attmap::AttrMap;
use crate::types::named_attmap::NamedAttrMap;

pub trait AttrMapCsgExt {
    fn create_csg_shape(&self, limit_size: Option<f32>) -> Option<Box<dyn BrepShapeTrait>>;
}

macro_rules! impl_csg_dispatch {
    ($self:expr, $get_type:ident, $limit_size:expr) => {{
        let type_noun = $self.$get_type();
        let mut r: Option<Box<dyn BrepShapeTrait>> = match type_noun {
            "BOX" | "NBOX" => Some(Box::new(SBox::from($self))),
            "CYLI" | "SLCY" | "NCYL" => Some(Box::new(SCylinder::from($self))),
            "SPHE" => Some(Box::new(Sphere::from($self))),
            "CONE" | "NCON" | "SNOU" | "NSNO" => Some(Box::new(LSnout::from($self))),
            "DISH" | "NDIS" => Some(Box::new(Dish::from($self))),
            "CTOR" | "NCTO" => Some(Box::new(CTorus::from($self))),
            "RTOR" | "NRTO" => Some(Box::new(RTorus::from($self))),
            "PYRA" | "NPYR" => Some(Box::new(Pyramid::from($self))),
            _ => None,
        };
        if let (Some(shape), Some(limit)) = (r.as_mut(), $limit_size) {
            shape.apply_limit_by_size(limit);
        }
        r
    }};
}

impl AttrMapCsgExt for AttrMap {
    fn create_csg_shape(&self, limit_size: Option<f32>) -> Option<Box<dyn BrepShapeTrait>> {
        impl_csg_dispatch!(self, get_type, limit_size)
    }
}

impl AttrMapCsgExt for NamedAttrMap {
    fn create_csg_shape(&self, limit_size: Option<f32>) -> Option<Box<dyn BrepShapeTrait>> {
        impl_csg_dispatch!(self, get_type_str, limit_size)
    }
}
