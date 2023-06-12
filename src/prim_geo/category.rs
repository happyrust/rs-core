use std::default::default;
use std::f32::consts::PI;
use std::f32::EPSILON;
use std::ops::Range;

use bevy::app::RunMode::Loop;
use bevy::math::*;
use bevy::prelude::Transform;
use id_tree::NodeId;
use smallvec::SmallVec;

use crate::parsed_data::geo_params_data::{CateGeoParam, PdmsGeoParam};
use crate::pdms_types::RefU64;
use crate::prim_geo::ctorus::{CTorus, SCTorus};
use crate::prim_geo::cylinder::SCylinder;
use crate::prim_geo::dish::Dish;
use crate::prim_geo::extrusion::Extrusion;
use crate::prim_geo::lpyramid::LPyramid;
use crate::prim_geo::pyramid::Pyramid;
use crate::prim_geo::revolution::Revolution;
use crate::prim_geo::rtorus::{RTorus, SRTorus};
use crate::prim_geo::sbox::SBox;
use crate::prim_geo::snout::LSnout;
use crate::prim_geo::sphere::Sphere;
use crate::prim_geo::tubing::PdmsTubing;
use crate::shape::pdms_shape::BrepShapeTrait;

#[derive(Debug, Clone)]
pub enum ShapeErr {
    //tubi的方向不一致
    TubiDirErr,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct CateBrepShape {
    pub refno: RefU64,
    pub brep_shape: Box<dyn BrepShapeTrait>,
    pub transform: Transform,
    pub visible: bool,
    pub is_tubi: bool,
    pub shape_err: Option<ShapeErr>,
    //点集信息
    pub pts: Vec<i32>,
}

///转换成brep shape
pub fn convert_to_brep_shapes(geom: &CateGeoParam) -> Option<CateBrepShape> {
    match geom {
        CateGeoParam::Pyramid(d) => {   //now dont resue pyramid
            let pa = d.pa.as_ref()?;
            let pb = d.pb.as_ref()?;
            let pc = d.pc.as_ref()?;
            let mut pts = Vec::default();
            pts.push(pa.number);
            pts.push(pb.number);
            pts.push(pc.number);

            let z_axis = Vec3::new(pa.dir[0], pa.dir[1], pa.dir[2]).normalize_or_zero();
            //需要转换成CTorus
            let pyramid = LPyramid {
                pbax_pt: (pb.pt),
                pbax_dir: (pb.dir).normalize_or_zero(),
                pcax_pt: (pc.pt),
                pcax_dir: (pc.dir).normalize_or_zero(),
                paax_pt: (pa.pt),
                paax_dir: (pa.dir).normalize_or_zero(),

                pbtp: d.x_top,
                pctp: d.y_top,
                pbbt: d.x_bottom,
                pcbt: d.y_bottom,
                ptdi: d.dist_to_top,
                pbdi: d.dist_to_btm,
                pbof: d.x_offset,
                pcof: d.y_offset,
            };
            ///需要偏移到 btm
            let translation = z_axis * (d.dist_to_btm + d.dist_to_top ) / 2.0;
            let brep_shape: Box<dyn BrepShapeTrait> = Box::new(pyramid);
            return Some(CateBrepShape {
                refno: d.refno,
                brep_shape,
                transform: Transform {
                    translation,
                    ..default()
                },
                visible: d.tube_flag,
                is_tubi: false,
                shape_err: None,
                pts,

            });
        }
        CateGeoParam::Torus(d) => {
            let pa = d.pa.as_ref()?;
            let pb = d.pb.as_ref()?;
            let mut pts = Vec::default();
            pts.push(pa.number);
            pts.push(pb.number);
            let sc_torus = SCTorus {
                paax_pt: (pa.pt),
                paax_dir: (pa.dir).normalize_or_zero(),
                pbax_pt: (pb.pt),
                pbax_dir: (pb.dir).normalize_or_zero(),
                pdia: d.diameter as f32,
            };
            // dbg!(d);
            if let Some((torus, transform)) = sc_torus.convert_to_ctorus() {
                let brep_shape: Box<dyn BrepShapeTrait> = Box::new(torus);
                return Some(CateBrepShape {
                    refno: d.refno,
                    brep_shape,
                    transform,
                    visible: d.tube_flag,
                    is_tubi: false,
                    shape_err: None,
                    pts,

                });
            }
        }
        CateGeoParam::RectTorus(d) => {
            let pa = d.pa.as_ref()?;
            let pb = d.pb.as_ref()?;
            let mut pts = Vec::default();
            pts.push(pa.number);
            pts.push(pb.number);
            let sr_torus = SRTorus {
                paax_expr: "PAAX".to_string(),
                paax_pt: pa.pt,
                paax_dir: pa.dir,
                pbax_expr: "PBAX".to_string(),
                pbax_pt: pb.pt,
                pbax_dir: pb.dir,
                pheig: d.height as f32,
                pdia: d.diameter as f32,
            };
            if let Some((torus, transform)) = sr_torus.convert_to_rtorus() {
                let brep_shape: Box<dyn BrepShapeTrait> = Box::new(torus);
                return Some(CateBrepShape {
                    refno: d.refno,
                    brep_shape,
                    transform,
                    visible: d.tube_flag,
                    is_tubi: false,
                    shape_err: None,
                    pts,

                });
            }
        }
        CateGeoParam::Box(d) => {
            let brep_shape: Box<dyn BrepShapeTrait> = Box::new(SBox {
                size: d.size,
                ..default()
            });
            let transform = Transform {
                translation: d.offset,
                ..default()
            };
            return Some(CateBrepShape {
                refno: d.refno,
                brep_shape,
                transform,
                visible: d.tube_flag,
                is_tubi: false,
                shape_err: None,
                pts: Default::default(),

            });
        }
        CateGeoParam::Dish(d) => {
            let axis = d.axis.as_ref()?;
            let mut pts = Vec::default();
            pts.push(axis.number);
            let dir = axis.dir.normalize_or_zero();
            if dir.length() == 0.0 { return None; }
            let translation = dir * (d.dist_to_btm as f32) + Vec3::new(axis.pt[0] as f32, axis.pt[1] as f32, axis.pt[2] as f32);
            let transform = Transform {
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
                refno: d.refno,
                brep_shape,
                transform,
                visible: d.tube_flag,
                is_tubi: false,
                shape_err: None,
                pts,

            });
        }
        CateGeoParam::Snout(d) => {
            // 统计复用个数
            let z = d.pa.as_ref()?;
            let x = d.pb.as_ref()?;
            let mut pts = Vec::default();
            pts.push(z.number);
            pts.push(x.number);

            let mut btm_on_top = false;
            let mut z_axis = z.dir;
            if z_axis.length() == 0.0 { return None; }
            let origin = z.pt;
            let x_axis = x.dir;
            let translation = origin + z_axis * (d.dist_to_btm as f32 + d.dist_to_top as f32) / 2.0;

            let mut height = (d.dist_to_top - d.dist_to_btm) as f32;
            let mut poff = d.offset as f32;

            let mut ptdm = d.top_diameter as f32;
            let mut pbdm = d.btm_diameter as f32;

            if height < 0.0 {
                btm_on_top = true;
                height = -height;
                ptdm = d.btm_diameter as f32;
                pbdm = d.top_diameter as f32;
            }

            let y_axis = z_axis.cross(x_axis).normalize_or_zero();
            if y_axis.length() == 0.0 {  return None; }


            let rotation = Quat::from_mat3(&Mat3::from_cols(
                x_axis, y_axis, z_axis,
            ));

            let transform = Transform {
                rotation,
                translation,
                ..default()
            };
            let brep_shape: Box<dyn BrepShapeTrait> = Box::new(LSnout {
                ptdi: height / 2.0,
                pbdi: -height / 2.0,
                ptdm,
                pbdm,
                poff,
                btm_on_top,
                ..Default::default()
            });
            return Some(CateBrepShape {
                refno: d.refno,
                brep_shape,
                transform,
                visible: d.tube_flag,
                is_tubi: false,
                shape_err: None,
                pts,

            });
        }
        CateGeoParam::SCylinder(d) => {
            let axis = d.axis.as_ref()?;
            let mut pts = Vec::default();
            pts.push(axis.number);
            let mut dir = axis.dir.normalize_or_zero();
            if dir.length() == 0.0 {  return None; }
            let mut phei = d.height as f32;
            let translation = dir * d.dist_to_btm + axis.pt;
            // dbg!(&translation);
            if phei < 0.0 {
                phei = -phei;
                dir = -dir;
            }
            let pdia = d.diameter as f32;
            let rotation = Quat::from_rotation_arc(Vec3::Z, dir);
            let transform = Transform {
                rotation,
                translation,
                ..default()
            };
            // 是以中心为原点，所以需要移动到中心位置
            let brep_shape: Box<dyn BrepShapeTrait> = Box::new(SCylinder {
                phei,
                pdia,
                ..default()
            });
            return Some(CateBrepShape {
                refno: d.refno,
                brep_shape,
                transform,
                visible: d.tube_flag,
                is_tubi: false,
                shape_err: None,
                pts,

            });
        }
        CateGeoParam::LCylinder(d) => {
            let axis = d.axis.as_ref()?;
            let mut pts = Vec::default();
            pts.push(axis.number);
            let mut dir = axis.dir.normalize_or_zero();
            if dir.length() == 0.0 {  return None; }
            let mut phei = (d.dist_to_top - d.dist_to_btm) as f32;
            let translation = dir * (d.dist_to_btm) + axis.pt;
            if phei < 0.0 {
                phei = -phei;
                dir = -dir;
            }
            let pdia = d.diameter as f32;
            let rotation = Quat::from_rotation_arc(Vec3::Z, dir);
            let transform = Transform {
                rotation,
                translation,
                ..default()
            };
            // 是以中心为原点，所以需要移动到中心位置
            let brep_shape: Box<dyn BrepShapeTrait> = Box::new(SCylinder {
                phei,
                pdia,
                ..default()
            });
            return Some(CateBrepShape {
                refno: d.refno,
                brep_shape,
                transform,
                visible: d.tube_flag,
                is_tubi: false,
                shape_err: None,
                pts,

            });
        }

        CateGeoParam::SlopeBottomCylinder(d) => {
            let axis = d.axis.as_ref()?;
            let mut pts = Vec::default();
            pts.push(axis.number);
            let z_dir = axis.dir.normalize_or_zero();
            if z_dir.length() == 0.0 {  return None; }
            let phei = d.height as f32;
            let pdia = d.diameter as f32;

            let mut x_dir = Vec3::Y.cross(z_dir).normalize_or_zero();
            // dbg!(x_dir);
            let mat3 = if x_dir.x > 0.0 {
                Mat3::from_cols(
                    x_dir,
                    Vec3::Y,
                    z_dir,
                )
            } else {
                Mat3::from_cols(
                    -x_dir,
                    -Vec3::Y,
                    z_dir,
                )
            };
            let angle_flag = -1.0;
            let rotation = Quat::from_mat3(&mat3);
            let translation = z_dir * (d.dist_to_btm as f32) +
                Vec3::new(axis.pt[0] as f32, axis.pt[1] as f32, axis.pt[2] as f32);
            let transform = Transform {
                rotation,
                translation,
                ..default()
            };
            // 是以中心为原点，所以需要移动到中心位置
            let brep_shape: Box<dyn BrepShapeTrait> = Box::new(SCylinder {
                phei,
                pdia,
                btm_shear_angles: [d.alt_x_shear * angle_flag, d.alt_y_shear * angle_flag],
                top_shear_angles: [d.x_shear * angle_flag, d.y_shear * angle_flag],
                ..default()
            });
            return Some(CateBrepShape {
                refno: d.refno,
                brep_shape,
                transform,
                visible: d.tube_flag,
                is_tubi: false,
                shape_err: None,
                pts,

            });
        }

        CateGeoParam::Sphere(d) => {
            // dbg!(d);
            let brep_shape: Box<dyn BrepShapeTrait> = Box::new(Sphere {
                radius: d.diameter as f32 / 2.0,
                ..default()
            });
            let axis = d.axis.as_ref()?;
            let mut pts = Vec::default();
            pts.push(axis.number);
            let transform = Transform {
                translation: axis.pt,
                ..default()
            };
            return Some(CateBrepShape {
                refno: d.refno,
                brep_shape,
                transform,
                visible: d.tube_flag,
                is_tubi: false,
                shape_err: None,
                pts,

            });
        }

        CateGeoParam::Revolution(d) => {
            let pa = d.pa.as_ref()?;
            let pb = d.pb.as_ref()?;
            let mut pts = Vec::default();
            pts.push(pa.number);
            pts.push(pb.number);
            let paax_dir = (pa.dir);
            let pbax_dir = (pb.dir);
            let extrude_dir = paax_dir.normalize_or_zero()
                .cross(pbax_dir.normalize_or_zero()).normalize_or_zero();
            if extrude_dir.length() == 0.0 { return None; }
            let mat3 = Mat3::from_cols(
                paax_dir,
                pbax_dir,
                extrude_dir,
            );
            let rotation = Quat::from_mat3(&mat3);
            let xyz_pt = Vec3::new(d.x, d.y, d.z);
            let origin_pt = (pa.pt);
            if d.verts.len() <= 2 {
                return None;
            }

            let brep_shape: Box<dyn BrepShapeTrait> = Box::new(Revolution {
                verts: d.verts.clone(),
                fradius_vec: d.frads.clone(),
                angle: d.angle,
                ..default()
            });

            let translation = origin_pt + xyz_pt;
            let transform = Transform {
                rotation,
                translation,
                ..default()
            };
            return Some(CateBrepShape {
                refno: d.refno,
                brep_shape,
                transform,
                visible: d.tube_flag,
                is_tubi: false,
                shape_err: None,
                pts,

            });
        }

        CateGeoParam::Extrusion(d) => {
            let pa = d.pa.as_ref()?;
            let pb = d.pb.as_ref()?;
            let mut pts = Vec::default();
            pts.push(pa.number);
            pts.push(pb.number);
            let paax_dir = (pa.dir);
            let mut pbax_dir = (pb.dir);

            let mut verts = vec![];
            if d.verts.len() > 2 {
                let mut prev = Vec3::new(d.verts[0][0], d.verts[0][1], 0.0);
                verts.push(prev);
                for vert in &d.verts[1..] {
                    let p = Vec3::new(vert[0], vert[1], 0.0);
                    if p.distance(prev) > EPSILON {
                        verts.push(p);
                    }
                }
            } else {
                return None;
            }

            let brep_shape: Box<dyn BrepShapeTrait> = Box::new(Extrusion {
                verts,
                fradius_vec: d.frads.clone(),
                height: d.height,
                ..default()
            });
            let extrude_dir = paax_dir.normalize_or_zero()
                .cross(pbax_dir.normalize_or_zero()).normalize_or_zero();
            if extrude_dir.length() == 0.0 { return None; }
            let pbax_dir = extrude_dir.cross(paax_dir.normalize_or_zero()).normalize_or_zero();
            let rotation = Quat::from_mat3(&Mat3::from_cols(
                paax_dir,
                pbax_dir,
                extrude_dir,
            ));
            let translation = rotation * Vec3::new(d.x, d.y, d.z) + (pa.pt);
            let transform = Transform {
                rotation,
                translation,
                ..default()
            };
            return Some(CateBrepShape {
                refno: d.refno,
                brep_shape,
                transform,
                visible: d.tube_flag,
                is_tubi: false,
                shape_err: None,
                pts,

            });
        }
        _ => {}
    }

    return None;
}