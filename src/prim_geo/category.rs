use bevy::math::TransformSRT;
use id_tree::NodeId;
use bevy::math::*;
use crate::prim_geo::ctorus::{CTorus, SCTorus};
use crate::prim_geo::cylinder::SCylinder;
use crate::prim_geo::dish::Dish;
use crate::prim_geo::extrusion::Extrusion;
use crate::prim_geo::pyramid::Pyramid;
use crate::prim_geo::revolution::Revolution;
use crate::prim_geo::rtorus::{RTorus, SRTorus};
use crate::prim_geo::sbox::SBox;
use crate::prim_geo::snout::LSnout;
use crate::prim_geo::sphere::Sphere;
use std::f32::consts::PI;
use std::ops::Range;
use std::default::default;
use std::f32::EPSILON;
use smallvec::SmallVec;
use crate::parsed_data::geo_params_data::CateGeoParam;
use crate::pdms_types::RefU64;
use crate::prim_geo::lpyramid::LPyramid;
use crate::shape::pdms_shape::BrepShapeTrait;

#[derive(Debug)]
pub struct CateBrepShape {
    pub refno: RefU64,
    pub brep_shape: Box<dyn BrepShapeTrait>,
    pub transform: TransformSRT,
    pub visible: bool,
    pub is_tubi: bool,
    pub pts: SmallVec<[i32; 3]>,  //点集信息
    // pub level: Range<u32>,
}


pub fn convert_to_brep_shapes(geom: &CateGeoParam) -> Option<CateBrepShape> {
    match geom {
        CateGeoParam::Pyramid(d) => {   //now dont resue pyramid
            let pa = d.pa.as_ref().unwrap();
            let pb = d.pb.as_ref().unwrap();
            let pc = d.pc.as_ref().unwrap();
            let mut pts =  SmallVec::default();
            pts.push(pa.number);
            pts.push(pb.number);
            pts.push(pc.number);

            let z_axis = Vec3::new(pa.dir[0], pa.dir[1], pa.dir[2]).normalize();
            //需要转换成CTorus
            let pyramid = LPyramid {
                pbax_pt: Vec3::from(pb.pt),
                pbax_dir: Vec3::from(pb.dir).normalize(),
                pcax_pt: Vec3::from(pc.pt),
                pcax_dir: Vec3::from(pc.dir).normalize(),
                paax_pt: Vec3::from(pa.pt),
                paax_dir: Vec3::from(pa.dir).normalize(),

                pbtp: d.x_top,
                pctp: d.y_top,
                pbbt: d.x_bottom,
                pcbt: d.y_bottom,
                ptdi: d.dist_to_top,
                pbdi: d.dist_to_btm,
                pbof: d.x_offset,
                pcof: d.y_offset,
            };
            let translation = z_axis * (d.dist_to_btm) as f32;
            let brep_shape: Box<dyn BrepShapeTrait> = Box::new(pyramid);
            return Some(CateBrepShape {
                refno: Default::default(),
                brep_shape,
                transform: TransformSRT {
                    translation,
                    ..default()
                },
                visible: d.tube_flag,
                is_tubi: false,
                pts,
            });
        }
        CateGeoParam::Torus(d) => {
            let pa = d.pa.as_ref().unwrap();
            let pb = d.pb.as_ref().unwrap();
            let mut pts =  SmallVec::default();
            pts.push(pa.number);
            pts.push(pb.number);
            let sc_torus = SCTorus {
                paax_pt: Vec3::from(pa.pt),
                paax_dir: Vec3::from(pa.dir).normalize(),
                pbax_pt: Vec3::from(pb.pt),
                pbax_dir: Vec3::from(pb.dir).normalize(),
                pdia: d.diameter as f32,
            };
            if let Some((torus, transform)) = sc_torus.convert_to_ctorus() {
                let brep_shape: Box<dyn BrepShapeTrait> = Box::new(torus);
                return Some(CateBrepShape {
                    refno: Default::default(),
                    brep_shape,
                    transform,
                    visible: d.tube_flag,
                    is_tubi: false,
                    pts,
                });
            }
        }
        CateGeoParam::RectTorus(d) => {
            let pa = d.pa.as_ref().unwrap();
            let pb = d.pb.as_ref().unwrap();
            let mut pts =  SmallVec::default();
            pts.push(pa.number);
            pts.push(pb.number);
            let sr_torus = SRTorus {
                paax_expr: "PAAX".to_string(),
                paax_pt: Vec3::new(pa.pt[0] as f32, pa.pt[1] as f32, pa.pt[2] as f32),
                paax_dir: Vec3::new(pa.dir[0] as f32, pa.dir[1] as f32, pa.dir[2] as f32),
                pbax_expr: "PBAX".to_string(),
                pbax_pt: Vec3::new(pb.pt[0] as f32, pb.pt[1] as f32, pb.pt[2] as f32),
                pbax_dir: Vec3::new(pb.dir[0] as f32, pb.dir[1] as f32, pb.dir[2] as f32),
                pheig: d.height as f32,
                pdia: d.diameter as f32,
            };
            if let Some((torus, transform)) = sr_torus.convert_to_rtorus() {
                let brep_shape: Box<dyn BrepShapeTrait> = Box::new(torus);
                return Some(CateBrepShape {
                    refno: Default::default(),
                    brep_shape,
                    transform,
                    visible: d.tube_flag,
                    is_tubi: false,
                    pts,
                });
            }
        }
        CateGeoParam::Box(d) => {
            let brep_shape: Box<dyn BrepShapeTrait> = Box::new(SBox {
                size: Vec3::new(d.size[0] as f32, d.size[1] as f32, d.size[2] as f32),
                ..default()
            });
            let transform = TransformSRT {
                translation: Vec3::new(d.offset[0] as f32, d.offset[1] as f32, d.offset[2] as f32),
                ..default()
            };
            return Some(CateBrepShape {
                refno: Default::default(),
                brep_shape,
                transform,
                visible: d.tube_flag,
                is_tubi: false,
                pts: Default::default()
            });
        }
        CateGeoParam::Dish(d) => {
            let axis = d.axis.as_ref().unwrap();
            let mut pts =  SmallVec::default();
            pts.push(axis.number);
            let dir = Vec3::new(axis.dir[0] as f32, axis.dir[1] as f32, axis.dir[2] as f32);
            let translation = dir * (d.dist_to_btm as f32) + Vec3::new(axis.pt[0] as f32, axis.pt[1] as f32, axis.pt[2] as f32);
            let transform = TransformSRT {
                rotation: Quat::from_rotation_arc(Vec3::Z, dir),
                translation,
                ..default()
            };
            let pheig = d.height as f32;
            let pdia = d.diameter as f32;
            let brep_shape: Box<dyn BrepShapeTrait> = Box::new(Dish {
                pdis: 0.0,
                pheig,
                pdia,
                ..default()
            });
            return Some(CateBrepShape {
                refno: Default::default(),
                brep_shape,
                transform,
                visible: d.tube_flag,
                is_tubi: false,
                pts,
            });
        }
        CateGeoParam::Snout(d) => {
            // 统计复用个数
            let z = d.pa.as_ref().unwrap();
            let x = d.pb.as_ref().unwrap();
            let mut pts =  SmallVec::default();
            pts.push(z.number);
            pts.push(x.number);

            let z_axis = Vec3::new(z.dir[0] as f32, z.dir[1] as f32, z.dir[2] as f32).normalize();
            let x_axis = Vec3::new(x.dir[0] as f32, x.dir[1] as f32, x.dir[2] as f32).normalize();

            let y_axis = z_axis.cross(x_axis).normalize();
            let origin = Vec3::new(z.pt[0] as f32, z.pt[1] as f32, z.pt[2] as f32);
            let height = (d.dist_to_top - d.dist_to_btm) as f32;
            let translation = origin + z_axis * (d.dist_to_btm as f32 + d.dist_to_top as f32) / 2.0;
            let local_rot = if height < 0.0 {
                Quat::from_rotation_x(PI)
            } else {
                Quat::IDENTITY
            };
            let transform = glam::TransformSRT {
                rotation: local_rot * bevy::prelude::Quat::from_mat3(&bevy::prelude::Mat3::from_cols(
                    x_axis, y_axis, z_axis,
                )),
                translation,
                ..default()
            };
            let brep_shape: Box<dyn BrepShapeTrait> = Box::new(LSnout {
                ptdi: height.abs() / 2.0,
                pbdi: -height.abs() / 2.0,   //为了能够实现复用
                ptdm: d.top_diameter as f32,
                pbdm: d.btm_diameter as f32,
                poff: d.offset as f32,
                ..Default::default()
            });
            return Some(CateBrepShape {
                refno: Default::default(),
                brep_shape,
                transform,
                visible: d.tube_flag,
                is_tubi: false,
                pts
            });
        }
        CateGeoParam::SCylinder(d) => {
            let axis = d.axis.as_ref().unwrap();
            let mut pts =  SmallVec::default();
            pts.push(axis.number);
            let dir = Vec3::new(axis.dir[0] as f32, axis.dir[1] as f32, axis.dir[2] as f32);
            let phei = d.height as f32;
            let pdia = d.diameter as f32;
            let rotation = Quat::from_rotation_arc(Vec3::Z, dir);
            let translation = dir * (d.dist_to_btm as f32 + phei / 2.0) +
                Vec3::new(axis.pt[0] as f32, axis.pt[1] as f32, axis.pt[2] as f32);
            let transform = TransformSRT {
                rotation,
                translation,
                ..default()
            };
            // 是以中心为原点，所以需要移动到中心位置
            let brep_shape: Box<dyn BrepShapeTrait> = Box::new(SCylinder {
                phei,
                pdia,
                pdis: 0.0,  //-phei / 2.0
                ..default()
            });
            return Some(CateBrepShape {
                refno: Default::default(),
                brep_shape,
                transform,
                visible: d.tube_flag,
                is_tubi: false,
                pts
            });
        }
        CateGeoParam::LCylinder(d) => {
            let axis = d.axis.as_ref().unwrap();
            let mut pts =  SmallVec::default();
            pts.push(axis.number);
            let dir = Vec3::new(axis.dir[0] as f32, axis.dir[1] as f32, axis.dir[2] as f32);
            let phei = (d.dist_to_top - d.dist_to_btm) as f32;
            let pdia = d.diameter as f32;
            let rotation = Quat::from_rotation_arc(Vec3::Z, dir);
            let translation = dir * (d.dist_to_btm as f32 + phei / 2.0 as f32) + Vec3::new(axis.pt[0] as f32, axis.pt[1] as f32, axis.pt[2] as f32);
            let transform = TransformSRT {
                rotation,
                translation,
                ..default()
            };
            // 是以中心为原点，所以需要移动到中心位置
            let brep_shape: Box<dyn BrepShapeTrait> = Box::new(SCylinder {
                phei,
                pdia,
                pdis: 0.0,
                ..default()
            });
            return Some(CateBrepShape {
                refno: Default::default(),
                brep_shape,
                transform,
                visible: d.tube_flag,
                is_tubi: false,
                pts
            });
        }
        CateGeoParam::Sphere(d) => {
            let brep_shape: Box<dyn BrepShapeTrait> = Box::new(Sphere {
                radius: d.diameter as f32 / 2.0,
                ..default()
            });
            let axis = d.axis.as_ref().unwrap();
            let mut pts =  SmallVec::default();
            pts.push(axis.number);
            let transform = TransformSRT {
                translation: Vec3::new(axis.pt[0] as f32, axis.pt[1] as f32, axis.pt[2] as f32),
                ..default()
            };
            return Some(CateBrepShape {
                refno: Default::default(),
                brep_shape,
                transform,
                visible: d.tube_flag,
                is_tubi: false,
                pts
            });
        }
        CateGeoParam::Extrusion(d) => {
            let pa = d.pa.as_ref().unwrap();
            let pb = d.pb.as_ref().unwrap();
            let mut pts =  SmallVec::default();
            pts.push(pa.number);
            pts.push(pb.number);
            let paax_dir = Vec3::from(pa.dir);
            let pbax_dir = Vec3::from(pb.dir);

            // dbg!(&d.verts);

            let mut verts = vec![];
            if d.verts.len() > 2 {
                let mut prev = Vec3::new(d.verts[0][0], d.verts[0][1], 0.0);
                verts.push(prev);
                for vert in &d.verts[1..] {
                    let p = Vec3::new(vert[0], vert[1], 0.0);
                    if p.distance(prev) > EPSILON{
                        verts.push(p);
                    }
                }
                // if verts.last().unwrap().distance(*verts.first().unwrap()) < EPSILON {
                //     verts.remove(verts.len() - 1);
                // }
            }else{
                return None;
            }

            // dbg!(&d);
            let brep_shape: Box<dyn BrepShapeTrait> = Box::new(Extrusion {
                paax_pt: Vec3::from(pa.pt),
                paax_dir,
                pbax_pt: Vec3::from(pb.pt),
                pbax_dir,
                verts,
                fradius_vec: d.prads.clone(),
                height: d.height,
                ..default()
            });
            // dbg!(&brep_shape);
            let extrude_dir = paax_dir.normalize()
                .cross(pbax_dir.normalize()).normalize();
            let rotation = Quat::from_mat3(&Mat3::from_cols(
                paax_dir,
                pbax_dir,
                extrude_dir,
            ));
            let translation = rotation * Vec3::new(d.x, d.y, -d.z);
            let transform = TransformSRT {
                rotation,
                translation,
                ..default()
            };
            return Some(CateBrepShape {
                refno: Default::default(),
                brep_shape,
                transform,
                visible: d.tube_flag,
                is_tubi: false,
                pts
            });
        }
        _ => {}
    }

    return None;
}