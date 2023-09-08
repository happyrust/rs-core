use crate::parsed_data::geo_params_data::CateGeoParam;
use crate::pdms_types::RefU64;
use crate::prim_geo::ctorus::SCTorus;
use crate::prim_geo::cylinder::SCylinder;
use crate::prim_geo::dish::Dish;
use crate::prim_geo::extrusion::Extrusion;
use crate::prim_geo::lpyramid::LPyramid;
use crate::prim_geo::revolution::Revolution;
use crate::prim_geo::rtorus::SRTorus;
use crate::prim_geo::sbox::SBox;
use crate::prim_geo::snout::LSnout;
use crate::prim_geo::sphere::Sphere;
use crate::shape::pdms_shape::BrepShapeTrait;
use bevy_math::prelude::*;
use bevy_transform::prelude::Transform;
use std::f32::EPSILON;

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
    //是否要和design发生负实体运算
    pub is_ngmr: bool,
}

///转换成brep shape
pub fn convert_to_brep_shapes(geom: &CateGeoParam) -> Option<CateBrepShape> {
    match geom {
        CateGeoParam::Pyramid(d) => {
            let pa = d.pa.as_ref()?;
            let pb = d.pb.as_ref()?;
            let pc = d.pc.as_ref()?;
            let mut pts = Vec::default();
            pts.push(pa.number);
            pts.push(pb.number);
            pts.push(pc.number);

            let z_axis = pa.dir.normalize_or_zero();
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
            let translation = z_axis * (d.dist_to_btm + d.dist_to_top) / 2.0 + pa.pt;
            let rotation = Quat::from_rotation_arc(Vec3::Z, z_axis);
            let brep_shape: Box<dyn BrepShapeTrait> = Box::new(pyramid);
            return Some(CateBrepShape {
                refno: d.refno,
                brep_shape,
                transform: Transform {
                    translation,
                    rotation,
                    ..Default::default()
                },
                visible: d.tube_flag,
                is_tubi: false,
                shape_err: None,
                pts,
                is_ngmr: false,
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
                    is_ngmr: false,
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
                    is_ngmr: false,
                });
            }
        }
        CateGeoParam::Box(d) => {
            let brep_shape: Box<dyn BrepShapeTrait> = Box::new(SBox {
                size: d.size,
                ..Default::default()
            });
            let transform = Transform {
                translation: d.offset,
                ..Default::default()
            };
            return Some(CateBrepShape {
                refno: d.refno,
                brep_shape,
                transform,
                visible: d.tube_flag,
                is_tubi: false,
                shape_err: None,
                pts: Default::default(),
                is_ngmr: false,
            });
        }
        CateGeoParam::Dish(d) => {
            let axis = d.axis.as_ref()?;
            let mut pts = Vec::default();
            pts.push(axis.number);
            let mut dir = axis.dir.normalize_or_zero();
            if dir.length() == 0.0 {
                return None;
            }
            let translation = dir * (d.dist_to_btm as f32) + axis.pt;
            let mut height = d.height;
            if d.height < 0.0 {
                height = -d.height;
                dir = -dir;
            }

            let transform = Transform {
                rotation: Quat::from_rotation_arc(Vec3::Z, dir),
                translation,
                ..Default::default()
            };
            let pdia = d.diameter as f32;
            let prad = d.radius as f32;
            let brep_shape: Box<dyn BrepShapeTrait> = Box::new(Dish {
                pdis: 0.0,
                pheig: height,
                pdia,
                prad,
                ..Default::default()
            });
            dbg!(&brep_shape);
            return Some(CateBrepShape {
                refno: d.refno,
                brep_shape,
                transform,
                visible: d.tube_flag,
                is_tubi: false,
                shape_err: None,
                pts,
                is_ngmr: false,
            });
        }
        CateGeoParam::Snout(d) | CateGeoParam::Cone(d) => {
            let z = d.pa.as_ref()?;
            // let x = d.pb.as_ref().unwrap_or(default);
            let mut x_dir = Vec3::X;
            let mut x_pt = Vec3::ZERO;
            let mut pts = Vec::default();
            pts.push(z.number);
            let mut is_cone = d.btm_diameter == 0.0;
            if let Some(pb) = d.pb.as_ref() {
                x_dir = pb.dir;
                pts.push(pb.number);
            }else{
                // dbg!(d);
            }
            
            let mut btm_on_top = false;
            let z_axis = z.dir;
            if z_axis.length() == 0.0 {
                return None;
            }
            let origin = z.pt;
            let x_axis = x_dir;
            let translation = origin + z_axis * (d.dist_to_btm as f32 + d.dist_to_top as f32) / 2.0;
            let mut height = (d.dist_to_top - d.dist_to_btm) as f32;
            let poff = d.offset as f32;

            let mut ptdm = d.top_diameter as f32;
            let mut pbdm = d.btm_diameter as f32;

            //统一使用旋转来实现
            if height < 0.0 {
                btm_on_top = true;
                height = -height;
                // z_axis = -z_axis;
                ptdm = d.btm_diameter as f32;
                pbdm = d.top_diameter as f32;
            }

            let y_axis = z_axis.cross(x_axis).normalize_or_zero();
            if y_axis.length() == 0.0 {
                return None;
            }

            let rotation = Quat::from_mat3(&Mat3::from_cols(x_axis, y_axis, z_axis));
            let transform = Transform {
                rotation,
                translation,
                ..Default::default()
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
                is_ngmr: false,
            });
        }
        CateGeoParam::SCylinder(d) => {
            let axis = d.axis.as_ref()?;
            let mut pts = Vec::default();
            pts.push(axis.number);
            let mut dir = axis.dir.normalize_or_zero();
            if dir.length() == 0.0 {
                return None;
            }
            let translation =  (dir * d.dist_to_btm + axis.pt);
            let mut phei = d.height as f32;
            //如果height是负数，相当于要额外旋转一下
            if phei < 0.0 {
                phei = -phei;
                dir = -dir;
            }
            let pdia = d.diameter as f32;
            let rotation = Quat::from_rotation_arc(Vec3::Z, dir);
            let transform = Transform {
                rotation,
                translation,
                ..Default::default()
            };
            // 是以中心为原点，所以需要移动到中心位置
            let brep_shape: Box<dyn BrepShapeTrait> = Box::new(SCylinder {
                phei,
                pdia,
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
                is_ngmr: false,
            });
        }
        CateGeoParam::LCylinder(d) => {
            let axis = d.axis.as_ref()?;
            let mut pts = Vec::default();
            pts.push(axis.number);
            let mut dir = axis.dir.normalize_or_zero();
            if dir.length() == 0.0 {
                return None;
            }
            let mut phei = (d.dist_to_top - d.dist_to_btm) as f32;
            let mut dis = d.dist_to_btm;
            let translation =  (dir * dis + axis.pt);
            //如果height是负数，相当于要额外旋转一下
            if phei < 0.0 {
                phei = -phei;
                dir = -dir;
            }
            let pdia = d.diameter as f32;
            let rotation = Quat::from_rotation_arc(Vec3::Z, dir);
            let transform = Transform {
                rotation,
                translation,
                ..Default::default()
            };
            // 是以中心为原点，所以需要移动到中心位置
            let brep_shape: Box<dyn BrepShapeTrait> = Box::new(SCylinder {
                phei,
                pdia,
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
                is_ngmr: false,
            });
        }

        CateGeoParam::SlopeBottomCylinder(d) => {
            let axis = d.axis.as_ref()?;
            let mut pts = Vec::default();
            pts.push(axis.number);
            let z_dir = axis.dir.normalize_or_zero();
            if z_dir.length() == 0.0 {
                return None;
            }
            let phei = d.height as f32;
            let pdia = d.diameter as f32;

            let x_dir = Vec3::Y.cross(z_dir).normalize_or_zero();
            // dbg!(x_dir);
            let mat3 = if x_dir.x > 0.0 {
                Mat3::from_cols(x_dir, Vec3::Y, z_dir)
            } else {
                Mat3::from_cols(-x_dir, -Vec3::Y, z_dir)
            };
            let angle_flag = -1.0;
            let rotation = Quat::from_mat3(&mat3);
            let translation = z_dir * (d.dist_to_btm as f32) + axis.pt;
            let transform = Transform {
                rotation,
                translation,
                ..Default::default()
            };
            // 是以中心为原点，所以需要移动到中心位置
            let brep_shape: Box<dyn BrepShapeTrait> = Box::new(SCylinder {
                phei,
                pdia,
                btm_shear_angles: [d.alt_x_shear * angle_flag, d.alt_y_shear * angle_flag],
                top_shear_angles: [d.x_shear * angle_flag, d.y_shear * angle_flag],
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
                is_ngmr: false,
            });
        }

        CateGeoParam::Sphere(d) => {
            // dbg!(d);
            let brep_shape: Box<dyn BrepShapeTrait> = Box::new(Sphere {
                radius: d.diameter as f32 / 2.0,
                ..Default::default()
            });
            let axis = d.axis.as_ref()?;
            let mut pts = Vec::default();
            pts.push(axis.number);
            let transform = Transform {
                translation: axis.pt,
                ..Default::default()
            };
            return Some(CateBrepShape {
                refno: d.refno,
                brep_shape,
                transform,
                visible: d.tube_flag,
                is_tubi: false,
                shape_err: None,
                pts,
                is_ngmr: false,
            });
        }

        CateGeoParam::Revolution(d) => {
            let pa = d.pa.as_ref()?;
            let pb = d.pb.as_ref()?;
            let mut pts = Vec::default();
            pts.push(pa.number);
            pts.push(pb.number);
            let paax_dir = pa.dir;
            let pbax_dir = pb.dir;
            let extrude_dir = paax_dir
                .normalize_or_zero()
                .cross(pbax_dir.normalize_or_zero())
                .normalize_or_zero();
            if extrude_dir.length() == 0.0 {
                return None;
            }
            let mat3 = Mat3::from_cols(paax_dir, pbax_dir, extrude_dir);
            let rotation = Quat::from_mat3(&mat3);
            let xyz_pt = Vec3::new(d.x, d.y, d.z);
            if d.verts.len() <= 2 {
                return None;
            }
            let brep_shape: Box<dyn BrepShapeTrait> = Box::new(Revolution {
                verts: d.verts.clone(),
                fradius_vec: d.frads.clone(),
                angle: d.angle,
                ..Default::default()
            });

            //rotation *
            let translation = pa.pt + xyz_pt;
            let transform = Transform {
                rotation,
                translation,
                ..Default::default()
            };
            return Some(CateBrepShape {
                refno: d.refno,
                brep_shape,
                transform,
                visible: d.tube_flag,
                is_tubi: false,
                shape_err: None,
                pts,
                is_ngmr: false,
            });
        }

        CateGeoParam::Extrusion(d) => {
            let pa = d.pa.as_ref()?;
            let pb = d.pb.as_ref()?;
            let mut pts = Vec::default();
            pts.push(pa.number);
            pts.push(pb.number);
            let mut paax_dir = pa.dir;
            let mut pbax_dir = pb.dir;

            let mut verts = vec![];
            if d.verts.len() > 2 {
                let prev = d.verts[0].truncate();
                verts.push(prev.extend(0.0));
                for vert in &d.verts[1..] {
                    let p = vert.truncate();
                    if p.distance(prev) > EPSILON {
                        verts.push(p.extend(0.0));
                    }
                }
            } else {
                return None;
            }

            let brep_shape: Box<dyn BrepShapeTrait> = Box::new(Extrusion {
                verts,
                fradius_vec: d.frads.clone(),
                height: d.height,
                ..Default::default()
            });
            let extrude_dir = paax_dir
                .normalize_or_zero()
                .cross(pbax_dir.normalize_or_zero())
                .normalize_or_zero();
            if extrude_dir.length() == 0.0 {
                return None;
            }
            let pbax_dir = extrude_dir
                .cross(paax_dir.normalize_or_zero())
                .normalize_or_zero();
            let rotation = Quat::from_mat3(&Mat3::from_cols(paax_dir, pbax_dir, extrude_dir));
            let translation = rotation * Vec3::new(d.x, d.y, d.z) + (pa.pt);
            let transform = Transform {
                rotation,
                translation,
                ..Default::default()
            };
            return Some(CateBrepShape {
                refno: d.refno,
                brep_shape,
                transform,
                visible: d.tube_flag,
                is_tubi: false,
                shape_err: None,
                pts,
                is_ngmr: false,
            });
        }
        _ => {}
    }

    return None;
}
