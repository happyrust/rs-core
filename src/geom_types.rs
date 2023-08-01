use bevy_transform::prelude::Transform;
use glam::{Quat, Vec3};
use parry3d::bounding_volume::Aabb;
use serde::{Serialize, Deserialize};
use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::pdms_types::{EleInstGeo, RefU64};
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use std::borrow::BorrowMut;
#[cfg(feature = "opencascade_rs")]
use opencascade::primitives::*;
use crate::parsed_data::geo_params_data::PdmsGeoParam::PrimSCylinder;
use crate::pdms_types::GeoBasicType;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct RvmGeoInfo {
    pub _key: String,
    pub aabb: Option<Aabb>,
    pub data: Vec<EleInstGeo>,
    // 相对世界坐标系下的变换矩阵 rot, translation, scale
    pub world_transform: Transform,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Default,Clone)]
pub struct RvmGeoInfos {
    #[serde_as(as = "DisplayFromStr")]
    pub refno: RefU64,
    pub att_type: String,
    pub world_transform: Transform,
    pub rvm_inst_geo: Vec<RvmInstGeo>,
}


impl RvmGeoInfos {
    #[cfg(feature = "opencascade_rs")]
    pub fn gen_occ_shape(&self) -> Option<Shape> {
        let mut pos_shapes = self.rvm_inst_geo.iter()
            .filter(|x| x.geo_type == GeoBasicType::Pos)
            .map(|x| x.gen_occ_shape())
            .flatten()
            .collect::<Vec<_>>();
        if pos_shapes.is_empty() { return None; }
        let mut final_shape = if pos_shapes.len() == 1 { pos_shapes.pop().unwrap() } else {
            let mut first_shape = pos_shapes.pop().unwrap();
            for s in pos_shapes {
                first_shape = first_shape.union_shape(&s).0;
            }
            first_shape
        };

        //执行相减运算
        self.rvm_inst_geo.iter()
            .filter(|x| x.geo_type == GeoBasicType::Neg ||x.geo_type == GeoBasicType::CateNeg)
            .for_each(|x|{
                if let Some(s) = x.gen_occ_shape() {
                    final_shape = final_shape.subtract_shape(&s).0;
                }
            });

        final_shape.transform_by_mat(&self.world_transform.compute_matrix().as_dmat4());
        Some(final_shape)
    }

    #[cfg(feature = "opencascade_rs")]
    pub fn gen_ngmr_occ_shape(&self) -> Option<Shape> {
        let mut ngmr_shapes = self.rvm_inst_geo.iter()
            .filter(|x| x.geo_type == GeoBasicType::CateCrossNeg)
            .map(|x| x.gen_occ_shape())
            .flatten()
            .collect::<Vec<_>>();
        if ngmr_shapes.is_empty() {   return None;}

        let mut final_shape = Compound::from_shapes(&ngmr_shapes);
        final_shape.transform_by_mat(&self.world_transform.compute_matrix().as_dmat4());
        Some(final_shape)
    }


}


#[serde_as]
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct RvmTubiGeoInfos {
    #[serde_as(as = "DisplayFromStr")]
    pub refno: RefU64,
    pub att_type: String,
    pub aabb: Option<Aabb>,
    pub world_transform: Transform,
    pub rvm_inst_geo: Vec<RvmInstGeo>,
}

impl RvmTubiGeoInfos {
    pub fn into_rvmgeoinfos(self) -> RvmGeoInfos {
        let mut geos = self.rvm_inst_geo;
        for mut geo in geos.iter_mut() {
            match geo.geo_param.borrow_mut() {
                PrimSCylinder(data) => {
                    data.phei = self.world_transform.scale.z;
                    data.pdia = self.world_transform.scale.x;
                }
                _ => { continue; }
            }
            geo.aabb = self.aabb;
        }
        RvmGeoInfos {
            refno: self.refno,
            att_type: self.att_type,
            world_transform: self.world_transform,
            rvm_inst_geo: geos,
        }
    }
}

/// rvm 需要的 元件 geo 数据
#[derive(Serialize, Deserialize, Debug, Default,Clone)]
pub struct RvmInstGeo {
    pub geo_param: PdmsGeoParam,
    pub geo_hash: String,
    pub aabb: Option<Aabb>,
    //相对于自身的坐标系变换
    pub transform: Transform,
    pub visible: bool,
    pub is_tubi: bool,
    pub geo_type: GeoBasicType,
}

impl RvmInstGeo {
    #[cfg(feature = "opencascade_rs")]
    pub fn gen_occ_shape(&self) -> Option<Shape> {
        let mut shape: Option<Shape> = self.geo_param.gen_occ_shape();
        //scale 不能要，已经包含在OCC的真实参数里
        let mut new_transform = self.transform;
        new_transform.scale = Vec3::ONE;
        if let Some(mut s) = shape.as_mut() {
            s.transform_by_mat(&new_transform.compute_matrix().as_dmat4());
        }
        shape
    }
}

