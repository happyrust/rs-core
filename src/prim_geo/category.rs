use crate::debug_model_debug;
use crate::parsed_data::geo_params_data::CateGeoParam;
use crate::prim_geo::LCylinder;
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
use crate::types::*;
use bevy_math::prelude::*;
use bevy_transform::prelude::Transform;
use std::f32::consts::FRAC_PI_2;

#[derive(Debug, Clone)]
pub enum ShapeErr {
    //tubi的方向不一致
    TubiDirErr,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct CateCsgShape {
    pub refno: RefnoEnum,
    pub csg_shape: Box<dyn BrepShapeTrait>,
    pub transform: Transform,
    pub visible: bool,
    pub is_tubi: bool,
    pub shape_err: Option<ShapeErr>,
    //点集信息
    pub pts: Vec<i32>,
    //是否要和design发生负实体运算
    pub is_ngmr: bool,
}

/// 将几何参数（CateGeoParam）转换为CSG形状（CateCsgShape）
///
/// 该函数根据输入的几何参数类型（如金字塔、圆环、盒子等）
/// 创建相应的CSG形状对象，并计算其变换矩阵（位置、旋转）
/// 返回Some(CateCsgShape)如果转换成功，否则返回None
///
/// # 参数
/// * `geom` - 几何参数枚举，包含各种PDMS几何体信息
///
/// # 返回值
/// - `Option<CateCsgShape>` - 转换后的CSG形状，如果失败则为None
pub fn convert_to_csg_shapes(geom: &CateGeoParam) -> Option<CateCsgShape> {
    match geom {
        CateGeoParam::Pyramid(d) => {
            // 金字塔几何体的转换逻辑
            // dbg!(d);
            let pa = d.pa.as_ref()?;
            let pb = d.pb.as_ref()?;
            let pc = d.pc.as_ref()?;
            let mut pts = Vec::default();
            pts.push(pa.number);
            pts.push(pb.number);
            pts.push(pc.number);

            let paax_dir = pa
                .dir
                .as_ref()
                .map(|d| d.0.normalize_or_zero())
                .unwrap_or(pa.dir_flag * Vec3::Z);
            let pbax_dir = pb
                .dir
                .as_ref()
                .map(|d| d.0.normalize_or_zero())
                .unwrap_or(pb.dir_flag * Vec3::Y);

            let pcax_dir = pc
                .dir
                .as_ref()
                .map(|d| d.0.normalize_or_zero())
                .unwrap_or(pc.dir_flag * Vec3::X);
            let z_axis = paax_dir;
            let mut x_axis = pbax_dir;
            let mut y_axis = pcax_dir;
            let mut rotation = Quat::IDENTITY;
            let tmp_axis = z_axis.cross(Vec3::Z).normalize_or_zero();
            // 有发生旋转，如果没有旋转，直接使用默认坐标系
            if tmp_axis.is_normalized() {
                let mut ref_axis = z_axis.cross(x_axis).normalize_or_zero();
                //如果求不出来y，就要按 z_axis 和 x_axis 结合，需要变通的去求方位
                if !ref_axis.is_normalized() {
                    x_axis = tmp_axis;
                    y_axis = z_axis.cross(x_axis).normalize_or_zero();
                    if !x_axis.is_normalized() {
                        println!("Pyramid 求方位失败。{:?}", (x_axis, y_axis, z_axis));
                        return None;
                    }
                    // dbg!((x_axis, y_axis, z_axis));
                } else {
                    y_axis = ref_axis;
                    x_axis = y_axis.cross(z_axis).normalize_or_zero();
                    // dbg!((x_axis, y_axis, z_axis));
                }
                rotation = Quat::from_mat3(&Mat3::from_cols(x_axis, y_axis, z_axis));
            }
            // dbg!((x_axis, y_axis, z_axis));
            //需要转换成CTorus
            let pyramid = LPyramid {
                pbax_pt: pb.pt.0,
                pbax_dir,
                pcax_pt: pc.pt.0,
                pcax_dir,
                paax_pt: pa.pt.0,
                paax_dir,

                pbtp: d.x_top,
                pctp: d.y_top,
                pbbt: d.x_bottom,
                pcbt: d.y_bottom,
                ptdi: d.dist_to_top,
                pbdi: d.dist_to_btm,
                pbof: d.x_offset,
                pcof: d.y_offset,
            };
            // dbg!(&pyramid);
            //需要偏移到 btm
            let translation = z_axis * (d.dist_to_btm + d.dist_to_top) / 2.0 + pa.pt.0;
            let csg_shape: Box<dyn BrepShapeTrait> = Box::new(pyramid);
            return Some(CateCsgShape {
                refno: d.refno,
                csg_shape,
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
            // 圆环（Torus）几何体的转换逻辑
            let pa = d.pa.as_ref()?;
            let pb = d.pb.as_ref()?;
            let mut pts = Vec::default();
            pts.push(pa.number);
            pts.push(pb.number);
            let paax_dir = pa
                .dir
                .as_ref()
                .map(|d| d.0.normalize_or_zero())
                .unwrap_or(pa.dir_flag * Vec3::X);
            let pbax_dir = pb
                .dir
                .as_ref()
                .map(|d| d.0.normalize_or_zero())
                .unwrap_or(pb.dir_flag * Vec3::Y);
            let sc_torus = SCTorus {
                paax_pt: pa.pt.0,
                paax_dir,
                pbax_pt: pb.pt.0,
                pbax_dir,
                pdia: d.diameter as f32,
            };
            // dbg!(d);
            if let Some((torus, transform)) = sc_torus.convert_to_ctorus() {
                let csg_shape: Box<dyn BrepShapeTrait> = Box::new(torus);
                return Some(CateCsgShape {
                    refno: d.refno,
                    csg_shape,
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
            // 矩形圆环（RectTorus）几何体的转换逻辑
            let pa = d.pa.as_ref()?;
            let pb = d.pb.as_ref()?;
            let mut pts = Vec::default();
            pts.push(pa.number);
            pts.push(pb.number);
            let paax_dir = pa
                .dir
                .as_ref()
                .map(|d| d.0.normalize_or_zero())
                .unwrap_or(pa.dir_flag * Vec3::X);
            let pbax_dir = pb
                .dir
                .as_ref()
                .map(|d| d.0.normalize_or_zero())
                .unwrap_or(pb.dir_flag * Vec3::Y);
            let sr_torus = SRTorus {
                paax_expr: "PAAX".to_string(),
                paax_pt: pa.pt.0,
                paax_dir,
                pbax_expr: "PBAX".to_string(),
                pbax_pt: pb.pt.0,
                pbax_dir,
                pheig: d.height as f32,
                pdia: d.diameter as f32,
            };
            if let Some((torus, transform)) = sr_torus.convert_to_rtorus() {
                let csg_shape: Box<dyn BrepShapeTrait> = Box::new(torus);
                return Some(CateCsgShape {
                    refno: d.refno,
                    csg_shape,
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
            // 盒子（Box）几何体的转换逻辑
            let csg_shape: Box<dyn BrepShapeTrait> = Box::new(SBox {
                size: d.size,
                ..Default::default()
            });
            let transform = Transform {
                translation: d.offset,
                ..Default::default()
            };
            return Some(CateCsgShape {
                refno: d.refno,
                csg_shape,
                transform,
                visible: d.tube_flag,
                is_tubi: false,
                shape_err: None,
                pts: Default::default(),
                is_ngmr: false,
            });
        }
        CateGeoParam::Dish(d) => {
            // 碟形（Dish）几何体的转换逻辑
            let axis = d.axis.as_ref()?;
            let mut pts = Vec::default();
            pts.push(axis.number);
            let mut axis_dir = axis
                .dir
                .as_ref()
                .map(|d| d.0.normalize_or_zero())
                .unwrap_or(Vec3::X);
            if axis_dir.length() == 0.0 {
                return None;
            }
            axis_dir = axis_dir
                .is_normalized()
                .then(|| axis_dir)
                .unwrap_or(axis.dir_flag * Vec3::X);

            let axis_pt = axis.pt.0;
            let bottom = axis_pt + axis_dir * (d.dist_to_btm as f32);
            let top = bottom + axis_dir * (d.height as f32);
            let axis_vec = top - bottom;
            let height = axis_vec.length();
            if height <= f32::EPSILON {
                return None;
            }

            let dir = axis_vec / height;
            let transform = Transform {
                rotation: Quat::from_rotation_arc(Vec3::Z, dir),
                translation: bottom,
                ..Default::default()
            };
            let pdia = d.diameter as f32;
            let prad = d.radius as f32;
            let csg_shape: Box<dyn BrepShapeTrait> = Box::new(Dish {
                pdis: 0.0,
                pheig: height,
                pdia,
                prad,
                ..Default::default()
            });
            // dbg!(&csg_shape);
            return Some(CateCsgShape {
                refno: d.refno,
                csg_shape,
                transform,
                visible: d.tube_flag,
                is_tubi: false,
                shape_err: None,
                pts,
                is_ngmr: false,
            });
        }
        CateGeoParam::Snout(d) | CateGeoParam::Cone(d) => {
            // 锥形（Snout/Cone）几何体的转换逻辑
            let pa = d.pa.as_ref()?;
            let mut x_dir = Vec3::Y;
            let mut pts = Vec::default();
            pts.push(pa.number);
            if let Some(pb) = d.pb.as_ref() {
                x_dir = pb
                    .dir
                    .as_ref()
                    .map(|d| d.0.normalize_or_zero())
                    .unwrap_or(pb.dir_flag * Vec3::Y);
                pts.push(pb.number);
            }

            let mut btm_on_top = false;
            let z_dir = pa
                .dir
                .as_ref()
                .map(|d| d.0.normalize_or_zero())
                .unwrap_or(pa.dir_flag * Vec3::X);

            let origin = pa.pt.0;
            let x_axis = x_dir;
            let translation = origin + z_dir * (d.dist_to_btm as f32 + d.dist_to_top as f32) / 2.0;
            let mut height = (d.dist_to_top - d.dist_to_btm) as f32;
            let poff = d.offset as f32;

            let mut ptdm = d.top_diameter as f32;
            let mut pbdm = d.btm_diameter as f32;

            //统一使用旋转来实现
            if height < 0.0 {
                btm_on_top = true;
                height = -height;
                ptdm = d.btm_diameter as f32;
                pbdm = d.top_diameter as f32;
            }

            let y_axis = z_dir.cross(x_axis).normalize_or_zero();
            // if !is_cone && y_axis.length() == 0.0 {
            //     return None;
            // }
            let rotation = if y_axis.length() == 0.0 {
                Quat::from_rotation_arc(Vec3::Z, z_dir)
            } else {
                Quat::from_mat3(&Mat3::from_cols(x_axis, y_axis, z_dir))
            };
            let transform = Transform {
                rotation,
                translation,
                ..Default::default()
            };
            let csg_shape: Box<dyn BrepShapeTrait> = Box::new(LSnout {
                ptdi: height / 2.0,
                pbdi: -height / 2.0,
                ptdm,
                pbdm,
                poff,
                btm_on_top,
                ..Default::default()
            });
            return Some(CateCsgShape {
                refno: d.refno,
                csg_shape,
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

            // 轴向方向，优先使用 axis.dir，没有的话使用 dir_flag
            let axis_dir = axis
                .dir
                .as_ref()
                .map(|d| d.0.normalize_or_zero())
                .unwrap_or(axis.dir_flag * Vec3::Y);

            let axis_pt = axis.pt.0;
            let dist_to_btm = d.dist_to_btm;
            let height_raw = d.height as f32;

            // 根据原始参数计算底部和顶部位置
            let bottom = axis_pt + axis_dir * dist_to_btm;
            let top = bottom + axis_dir * height_raw;
            let axis_vec = top - bottom;
            let phei = axis_vec.length();
            if phei <= f32::EPSILON {
                return None;
            }

            let dir = axis_vec / phei;

            let pdia = d.diameter as f32;
            let rotation = Quat::from_rotation_arc(Vec3::Z, dir);
            let translation = if d.centre_line_flag {
                bottom + dir * (phei * 0.5)
            } else {
                bottom
            };

            let scyl = SCylinder {
                phei,
                pdia,
                center_in_mid: d.centre_line_flag,
                ..Default::default()
            };
            let transform = Transform {
                rotation,
                translation,
                ..Default::default()
            };
            let csg_shape: Box<dyn BrepShapeTrait> = Box::new(scyl);
            return Some(CateCsgShape {
                refno: d.refno,
                csg_shape,
                transform,
                visible: d.tube_flag,
                is_tubi: false,
                shape_err: None,
                pts,
                is_ngmr: false,
            });
        }
        CateGeoParam::LCylinder(d) => {
            // 长圆柱（LCylinder）几何体的转换逻辑
            let axis = d.axis.as_ref()?;
            let mut pts = Vec::default();
            pts.push(axis.number);

            let axis_dir = axis
                .dir
                .as_ref()
                .map(|d| d.0.normalize_or_zero())
                .unwrap_or(axis.dir_flag * Vec3::Y);

            let axis_pt = axis.pt.0;
            let mut bottom_dist = d.dist_to_btm;
            let mut top_dist = d.dist_to_top;
            if top_dist < bottom_dist {
                std::mem::swap(&mut bottom_dist, &mut top_dist);
            }

            let bottom = axis_pt + axis_dir * bottom_dist;
            let top = axis_pt + axis_dir * top_dist;
            let axis_vec = top - bottom;
            let height = axis_vec.length();
            if height <= f32::EPSILON {
                return None;
            }

            let dir = axis_vec / height;
            let pdia = d.diameter as f32;
            let rotation = Quat::from_rotation_arc(Vec3::Z, dir);
            let translation = if d.centre_line_flag {
                bottom + dir * (height * 0.5)
            } else {
                bottom
            };

            let csg_shape: Box<dyn BrepShapeTrait> = Box::new(LCylinder {
                pbdi: bottom_dist,
                ptdi: top_dist,
                pdia,
                centre_line_flag: d.centre_line_flag,
                ..Default::default()
            });
            let transform = Transform {
                rotation,
                translation,
                ..Default::default()
            };

            return Some(CateCsgShape {
                refno: d.refno,
                csg_shape,
                transform,
                visible: d.tube_flag,
                is_tubi: false,
                shape_err: None,
                pts,
                is_ngmr: false,
            });
        }

        CateGeoParam::SlopeBottomCylinder(d) => {
            // 斜底圆柱（SlopeBottomCylinder）几何体的转换逻辑
            let axis = d.axis.as_ref()?;
            let mut pts = Vec::default();
            pts.push(axis.number);
            let z_axis = axis
                .dir
                .as_ref()
                .map(|d| d.0.normalize_or_zero())
                .unwrap_or(axis.dir_flag * Vec3::Y);
            // dbg!(d.refno);
            if z_axis.length() == 0.0 {
                return None;
            }
            // dbg!(z_axis);
            let phei = d.height as f32;
            let pdia = d.diameter as f32;
            let ref_axis = axis.ref_dir.as_ref().map(|r| r.0).unwrap_or_default();
            //检查有没有参考轴，没有的话使用底部的， 不能使用这个from_rotation_arc
            let rotation = if ref_axis.length() == 0.0 {
                //ref_axis初始轴为X轴，先绕着y轴旋转x_shear, 再绕着x轴旋转 y_shear
                let rot1 = Quat::from_rotation_arc(Vec3::Z, z_axis);
                let mut rot2 = Quat::IDENTITY;
                if d.y_shear.abs() > d.x_shear.abs() {
                    //todo 旋转到长轴即可
                    let t = if z_axis.z > 0.01 {
                        -1.0
                    } else if z_axis.z < -0.01 {
                        1.0
                    } else {
                        if z_axis.x > 0.01 { -1.0 } else { 1.0 }
                    };
                    // dbg!(t);
                    rot2 = Quat::from_axis_angle(z_axis, t * FRAC_PI_2);
                }
                rot2 * rot1
            } else {
                let y_axis = ref_axis;
                let x_axis = y_axis.cross(z_axis).normalize_or_zero();
                if !x_axis.is_normalized() {
                    return None;
                }
                Quat::from_mat3(&Mat3::from_cols(x_axis, y_axis, z_axis))
            };
            let translation = z_axis * (d.dist_to_btm as f32) + axis.pt.0;
            let transform = Transform {
                rotation,
                translation,
                ..Default::default()
            };
            // 是以中心为原点，所以需要移动到中心位置
            let csg_shape: Box<dyn BrepShapeTrait> = Box::new(SCylinder {
                paxi_dir: z_axis,
                phei,
                pdia,
                // ✅ 修正：底部剪切角使用 alt_x_shear, alt_y_shear（PXBS, PYBS）
                btm_shear_angles: [d.alt_x_shear, d.alt_y_shear],
                // ✅ 修正：顶部剪切角使用 x_shear, y_shear（PXTS, PYTS）
                top_shear_angles: [d.x_shear, d.y_shear],
                ..Default::default()
            });
            return Some(CateCsgShape {
                refno: d.refno,
                csg_shape,
                transform,
                visible: d.tube_flag,
                is_tubi: false,
                shape_err: None,
                pts,
                is_ngmr: false,
            });
        }

        CateGeoParam::Sphere(d) => {
            // 球体（Sphere）几何体的转换逻辑
            // dbg!(d);
            let csg_shape: Box<dyn BrepShapeTrait> = Box::new(Sphere {
                radius: d.diameter as f32 / 2.0,
                ..Default::default()
            });
            let axis = d.axis.as_ref()?;
            let mut pts = Vec::default();
            pts.push(axis.number);
            let transform = Transform {
                translation: axis.pt.0,
                ..Default::default()
            };
            return Some(CateCsgShape {
                refno: d.refno,
                csg_shape,
                transform,
                visible: d.tube_flag,
                is_tubi: false,
                shape_err: None,
                pts,
                is_ngmr: false,
            });
        }

        CateGeoParam::Revolution(d) => {
            // 旋转体（Revolution）几何体的转换逻辑
            let pa = d.pa.as_ref()?;
            let pb = d.pb.as_ref()?;
            let mut pts = Vec::default();
            pts.push(pa.number);
            pts.push(pb.number);
            let paax_dir = pa
                .dir
                .as_ref()
                .map(|d| d.0.normalize_or_zero())
                .unwrap_or(pa.dir_flag * Vec3::X);
            let pbax_dir = pb
                .dir
                .as_ref()
                .map(|d| d.0.normalize_or_zero())
                .unwrap_or(pb.dir_flag * Vec3::Y);
            let z_dir = paax_dir
                .normalize_or_zero()
                .cross(pbax_dir.normalize_or_zero())
                .normalize_or_zero();
            if !z_dir.is_normalized() {
                return None;
            }
            //需要重新计算pbax dir，paax dir是一个主要的方向，这个不能变
            let pbax_dir = z_dir.cross(paax_dir).normalize_or_zero();
            let mat3 = Mat3::from_cols(paax_dir, pbax_dir, z_dir);
            let rotation = Quat::from_mat3(&mat3);
            let xyz_pt = Vec3::new(d.x, d.y, d.z);
            if d.verts.len() <= 2 {
                return None;
            }
            let csg_shape: Box<dyn BrepShapeTrait> = Box::new(Revolution {
                verts: vec![d.verts.clone()],
                angle: d.angle,
                ..Default::default()
            });

            let translation = pa.pt.0 + xyz_pt;
            let transform = Transform {
                rotation,
                translation: translation.into(),
                ..Default::default()
            };
            return Some(CateCsgShape {
                refno: d.refno,
                csg_shape,
                transform,
                visible: d.tube_flag,
                is_tubi: false,
                shape_err: None,
                pts,
                is_ngmr: false,
            });
        }

        CateGeoParam::Extrusion(d) => {
            // 挤出体（Extrusion）几何体的转换逻辑
            let pa = d.pa.as_ref()?;
            let pb = d.pb.as_ref()?;
            let mut pts = Vec::default();
            pts.push(pa.number);
            pts.push(pb.number);
            //如果有一个轴为0
            let paax_dir = pa
                .dir
                .as_ref()
                .map(|d| d.0.normalize_or_zero())
                .unwrap_or(pa.dir_flag * Vec3::X);
            let pbax_dir = pb
                .dir
                .as_ref()
                .map(|d| d.0.normalize_or_zero())
                .unwrap_or(pb.dir_flag * Vec3::Y);
            let mut z_dir = paax_dir.cross(pbax_dir).normalize_or_zero();
            if !z_dir.is_normalized() {
                return None;
            }
            let pbax_dir = z_dir.cross(paax_dir).normalize_or_zero();
            let csg_shape: Box<dyn BrepShapeTrait> = Box::new(Extrusion {
                verts: vec![d.verts.clone()],
                height: d.height,
                ..Default::default()
            });
            let rotation = Quat::from_mat3(&Mat3::from_cols(paax_dir, pbax_dir, z_dir));
            let translation = rotation * Vec3::new(d.x, d.y, d.z) + pa.pt.0;
            let transform = Transform {
                rotation,
                translation,
                ..Default::default()
            };
            return Some(CateCsgShape {
                refno: d.refno,
                csg_shape,
                transform,
                visible: d.tube_flag,
                is_tubi: false,
                shape_err: None,
                pts,
                is_ngmr: false,
            });
        }
        _ => {
            // 未处理的几何类型，返回None
        }
    }

    return None;
}
