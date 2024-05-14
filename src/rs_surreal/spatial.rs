use crate::room::room::GLOBAL_AABB_TREE;
use crate::tool::math_tool;
use crate::tool::math_tool::{dquat_to_pdms_ori_xyz_str, to_pdms_vec_str};
use crate::{
    accel_tree::acceleration_tree::QueryRay,
    consts::HAS_PLIN_TYPES,
    pdms_data::{PlinParam, PlinParamData},
    prim_geo::spine::{Spine3D, SpineCurveType, SweepPath3D},
    shape::pdms_shape::LEN_TOL,
    tool::{
        direction_parse::parse_expr_to_dir,
        math_tool::{quat_to_pdms_ori_str, quat_to_pdms_ori_xyz_str},
    },
    NamedAttrMap, RefU64, SUL_DB,
};
use anyhow::anyhow;
use approx::abs_diff_eq;
use async_recursion::async_recursion;
use bevy_transform::prelude::*;
use cached::proc_macro::cached;
use futures::future::{BoxFuture, FutureExt};
use glam::{DMat3, DMat4, DQuat, DVec3, Mat3, Mat4, Quat, Vec3};
use parry3d::bounding_volume::Aabb;
use parry3d::query::Ray;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use std::{collections::HashSet, f32::consts::E, time::Instant};

pub fn cal_ori_by_z_axis_ref_x(v: DVec3, neg: bool) -> DQuat {
    let mut ref_dir = if v.normalize().dot(DVec3::Z).abs() > 0.999 {
        DVec3::Y
    } else {
        DVec3::Z
    };
    if neg {
        ref_dir = -ref_dir;
    }

    let y_dir = v.cross(ref_dir).normalize();
    let x_dir = y_dir.cross(v).normalize();

    let rotation = DQuat::from_mat3(&DMat3::from_cols(x_dir, y_dir, v));
    rotation
}

pub fn cal_ori_by_z_axis_ref_y(v: DVec3) -> DQuat {
    let mut ref_dir = if v.normalize().dot(DVec3::Z).abs() > 0.999 {
        DVec3::Y
    } else {
        DVec3::Z
    };

    let x_dir = ref_dir.cross(v).normalize();
    let y_dir = v.cross(x_dir).normalize();

    let rotation = DQuat::from_mat3(&DMat3::from_cols(x_dir, y_dir, v));
    rotation
}

pub fn cal_ori_by_extru_axis(v: DVec3, neg: bool) -> DQuat {
    let mut y_ref_dir = if v.normalize().dot(DVec3::Z).abs() > 0.999 {
        DVec3::X
    } else {
        DVec3::Z
    };
    if neg {
        y_ref_dir = -y_ref_dir;
    }

    let x_dir = y_ref_dir.cross(v).normalize();
    let y_dir = v.cross(x_dir).normalize();
    // dbg!((y_ref_dir, x_dir, y_dir, v));
    let rotation = DQuat::from_mat3(&DMat3::from_cols(x_dir, y_dir, v));
    rotation
}

pub async fn get_spline_pts(refno: RefU64) -> anyhow::Result<Vec<DVec3>> {
    let mut response = SUL_DB.query(
        format!("select value (select in.refno.POS as pos, order_num from <-pe_owner[where in.noun='SPINE'].in<-pe_owner order by order_num).pos from only pe:{}", refno)).await?;
    let pts: Vec<DVec3> = response.take(0)?;
    Ok(pts)
}

pub async fn get_spline_line_dir(refno: RefU64) -> anyhow::Result<DVec3> {
    let mut response = SUL_DB.query(
        format!("select value (select in.refno.POS as pos, order_num from <-pe_owner[where in.noun='SPINE'].in<-pe_owner order by order_num).pos from only pe:{}", refno)).await?;
    let pts: Vec<DVec3> = response.take(0)?;
    if pts.len() == 2 {
        return Ok((pts[1] - pts[0]).normalize());
    }
    Err(anyhow!("没有找到两个点"))
}

#[cached(result = true)]
pub async fn get_world_transform(refno: RefU64) -> anyhow::Result<Option<Transform>> {
    get_world_mat4(refno, false)
        .await
        .map(|m| m.map(|x| Transform::from_matrix(x.as_mat4())))
}

//获得世界坐标系
///使用cache，需要从db manager里移除出来
///获得世界坐标系, 需要缓存数据，如果已经存在数据了，直接获取
#[cached(result = true)]
pub async fn get_world_mat4(refno: RefU64, is_local: bool) -> anyhow::Result<Option<DMat4>> {
    let mut ancestors: Vec<NamedAttrMap> = super::get_ancestor_attmaps(refno).await?;
    if ancestors.len() <= 1 {
        return Ok(Some(DMat4::IDENTITY));
    }
    ancestors.reverse();
    let mut rotation = DQuat::IDENTITY;
    let mut translation = DVec3::ZERO;
    let mut prev_mat4 = DMat4::IDENTITY;
    let mut mat4 = DMat4::IDENTITY;

    let mut owner = refno;
    for atts in ancestors.windows(2) {
        let o_att = &atts[0];
        let att = &atts[1];
        let cur_type = att.get_type_str();
        let ower_type = o_att.get_type_str();
        let refno = att.get_refno().unwrap_or_default();
        owner = o_att.get_refno_or_default();
        prev_mat4 = mat4;

        let mut pos = att.get_position().unwrap_or_default().as_dvec3();
        // dbg!(pos);
        let mut quat = DQuat::IDENTITY;
        let bangle = att.get_f32("BANG").unwrap_or_default() as f64;
        let has_bang = att.contains_key("BANG");
        //土建特殊情况的一些处理
        if att.contains_key("ZDIS") && (!att.contains_key("POSL")) {
            let zdist = att.get_f32("ZDIS").unwrap_or_default();
            let pkdi = att.get_f32("PKDI").unwrap_or_default();
            //zdis 起点应该是从poss 开始，所以这里需要加上这个偏移
            let result = cal_zdis_pkdi_in_section(owner, pkdi, zdist).await;
            pos += (result.1) ;
            quat *= result.0;
            if has_bang {
                quat = quat * DQuat::from_rotation_z(bangle.to_radians());
            }
            translation = translation + rotation * pos;
            // dbg!(translation);
            // dbg!(dquat_to_pdms_ori_xyz_str(&rotation));
            rotation = quat;
            // dbg!(dquat_to_pdms_ori_xyz_str(&rotation));
            continue;
        }

        if att.contains_key("NPOS") {
            let npos = att.get_vec3("NPOS").unwrap_or_default();
            pos += npos.as_dvec3();
        }

        let owner_is_gensec = ower_type == "GENSEC";
        let quat_v = att.get_rotation();
        let mut need_bangle = false;
        let mut pos_draw_dir = None;
        if !owner_is_gensec && quat_v.is_some() {
            quat = quat_v.unwrap().as_dquat();
        } else {
            let (l_poss, l_pose) = if owner_is_gensec {
                //找到spine，获取spine的两个顶点
                let pts: Vec<DVec3> = get_spline_pts(owner).await?;
                if pts.len() == 2 {
                    (Some(pts[0]), Some(pts[1]))
                } else {
                    (None, None)
                }
            } else {
                (att.get_dposs(), att.get_dpose())
            };
            if let (Some(poss), Some(pose)) = (l_poss, l_pose) {
                // dbg!((l_poss, l_pose));
                need_bangle = true;
                let z_axis = (pose - poss).normalize();
                // dbg!(z_axis);
                if !z_axis.is_normalized() {
                    return Ok(None);
                }
                pos_draw_dir = Some(z_axis);
                quat = if owner_is_gensec {
                    cal_ori_by_z_axis_ref_x(z_axis, true)
                } else {
                    cal_ori_by_z_axis_ref_y(z_axis)
                };
                // dbg!(dquat_to_pdms_ori_xyz_str(&quat));
            }
        }

        if need_bangle || has_bang {
            quat = quat * DQuat::from_rotation_z(bangle.to_radians());
        }
        //对于有CUTB的情况，需要直接对齐过去, 不需要在这里计算
        let c_ref = att.get_foreign_refno("CREF").unwrap_or_default();
        let mut cut_dir = DVec3::Y;
        let mut has_cut_back = false;
        //如果posl有，就不起用CUTB，相当于CUTB是一个手动对齐
        //直接在世界坐标系下求坐标，跳过局部求解
        if c_ref.is_valid() && att.get_str("POSL").is_none() && att.contains_key("CUTB") {
            cut_dir = att.get_dvec3("CUTP").unwrap_or(cut_dir);
            let cut_len = att.get_f64("CUTB").unwrap_or_default();
            if let Ok(c_att) = super::get_named_attmap(c_ref).await {
                let c_t = Box::pin(get_world_transform(c_ref))
                    .await?
                    .unwrap_or_default();
                if let (Some(poss), Some(pose)) = (c_att.get_poss(), c_att.get_pose()) {
                    let w_poss = c_t.translation.as_dvec3();
                    let axis = pose - poss;
                    let len = axis.length() as f64;
                    let w_pose = (w_poss + c_t.rotation.as_dquat() * DVec3::Z * len);
                    let dist_s = translation.distance(w_poss);
                    let dist_e = translation.distance(w_pose);
                    //取离node最近的点
                    if dist_s < dist_e {
                        translation = w_poss - cut_dir * cut_len;
                    } else {
                        translation = w_pose - cut_dir * cut_len;
                    }
                    has_cut_back = true;
                }
                //有 cref 的时候，需要保持方向和 cref 一致
                if let Some(opdir) = att.get_dvec3("OPDI").map(|x| x.normalize()) {
                    quat = cal_ori_by_z_axis_ref_x(opdir, true);
                    // dbg!(math_tool::dquat_to_pdms_ori_xyz_str(&quat));
                }
            }
        }
        //todo fix 处理 posl的计算
        if att.contains_key("POSL") {
            let pos_line = att.get_str("POSL").unwrap_or("NA");
            let pos_line = if pos_line.is_empty() { "NA" } else { pos_line };
            let delta_vec = att.get_dvec3("DELP").unwrap_or_default();
            //plin里的位置偏移
            let mut plin_pos = DVec3::ZERO;
            let mut pline_plax = DVec3::X;
            // POSL 的处理, 获得父节点的形集, 自身的形集处理，已经在profile里处理过
            let mut is_lmirror = false;
            let ancestor_refnos =
                crate::query_filter_ancestors(owner, HAS_PLIN_TYPES.map(String::from).to_vec())
                    .await?;
            if let Some(plin_owner) = ancestor_refnos.into_iter().next() {
                let target_own_att = crate::get_named_attmap(plin_owner)
                    .await
                    .unwrap_or_default();
                is_lmirror = target_own_att.get_bool("LMIRR").unwrap_or_default();
                let own_pos_line = target_own_att.get_str("JUSL").unwrap_or("NA");
                let own_pos_line = if own_pos_line.is_empty() {
                    "NA"
                } else {
                    own_pos_line
                };

                if let Ok(Some(param)) = crate::query_pline(plin_owner, pos_line.into()).await {
                    plin_pos = param.pt;
                    pline_plax = param.plax;
                    #[cfg(feature = "debug")]
                    {
                        dbg!(plin_owner);
                        dbg!(pos_line);
                        dbg!(&param);
                    }
                }
                if let Ok(Some(own_param)) = query_pline(plin_owner, own_pos_line.into()).await {
                    plin_pos -= own_param.pt;
                    #[cfg(feature = "debug")]
                    {
                        dbg!(own_pos_line);
                        dbg!(&own_param);
                    }
                }
            }
            let mut z_axis = if is_lmirror {
                -pline_plax
            } else {
                pline_plax
            };
            if att.contains_key("YDIR") {
                if let Some(v) = att.get_dvec3("YDIR"){
                    if v.y != 0.0{
                        z_axis = v.normalize();
                    }
                }
            }
            let new_quat = {
                if cur_type == "FITT" {
                    // dbg!(z_axis);
                    let zdist = att.get_f32("ZDIS").unwrap_or_default() as f64;
                    pos += zdist * DVec3::Z;
                    //受到bang的影响，需要变换
                    //绕着z轴旋转
                    let y_axis = DQuat::from_axis_angle(z_axis, bangle.to_radians()) * DVec3::Z;
                    let x_axis = y_axis.cross(z_axis).normalize();
                    // dbg!((x_axis, y_axis, z_axis));
                    DQuat::from_mat3(&DMat3::from_cols(x_axis, y_axis, z_axis))
                }else {
                    cal_ori_by_z_axis_ref_y(z_axis) * quat
                }
            };
            // dbg!(dquat_to_pdms_ori_xyz_str(&new_quat));
            translation += rotation * (pos + plin_pos) + rotation * new_quat * delta_vec;

            //没有POSL时，需要使用cutback的方向
            // dbg!(dquat_to_pdms_ori_xyz_str(&rotation));
            rotation = rotation * new_quat;
            // dbg!(dquat_to_pdms_ori_xyz_str(&rotation));
            if pos_line == "unset" && has_cut_back {
                let mat3 = DMat3::from_quat(rotation);
                let y_axis = mat3.y_axis;
                let ref_axis = cut_dir;
                // dbg!(cut_dir);
                #[cfg(feature = "debug")]
                {
                    dbg!(cut_dir);
                }
                let x_axis = y_axis.cross(ref_axis).normalize();
                let z_axis = x_axis.cross(y_axis).normalize();
                let new_mat = DMat3::from_cols(x_axis, y_axis, z_axis);
                rotation = DQuat::from_mat3(&new_mat);
            }
            // dbg!(dquat_to_pdms_ori_xyz_str(&rotation));
        } else {
            translation = translation + rotation * pos;
            rotation = rotation * quat;
            if let Some(v) = att.get_dvec3("YDIR")  {
                let y_ref_axis = v.normalize();
                // dbg!(y_ref_axis);
                let m = DMat3::from_quat(rotation);
                let z_axis = m.z_axis;
                let x_axis = y_ref_axis.cross(z_axis).normalize();
                let y_axis = z_axis.cross(x_axis).normalize();
                rotation = DQuat::from_mat3(&DMat3::from_cols(x_axis, y_axis, z_axis));
            }
        }

        mat4 = DMat4::from_rotation_translation(rotation, translation);
    }

    if rotation.is_nan() || translation.is_nan() {
        return Ok(None);
    }

    if is_local {
        mat4 = prev_mat4.inverse() * mat4;
    }

    Ok(Some(mat4))
}

///查询形集PLIN的值，todo 需要做缓存优化
// #[cached]
pub async fn query_pline(refno: RefU64, jusl: String) -> anyhow::Result<Option<PlinParamData>> {
    let cat_att = crate::get_cat_attmap(refno).await.unwrap_or_default();
    let psref = cat_att
        .get_foreign_refno("PSTR")
        .unwrap_or(cat_att.get_foreign_refno("PTSS").unwrap_or_default());
    if !psref.is_valid() {
        return Ok(None);
    }
    let c_refnos = crate::get_children_refnos(psref).await.unwrap_or_default();
    // dbg!(&c_refnos);
    for c_refno in c_refnos {
        let a = crate::get_named_attmap(c_refno).await?;
        let Some(p_key) = a.get_as_string("PKEY") else {
            continue;
        };
        let param = PlinParam {
            vxy: [
                a.get_as_string("PX").unwrap_or("0".to_string()),
                a.get_as_string("PY").unwrap_or("0".to_string()),
            ],
            dxy: [
                a.get_as_string("DX").unwrap_or("0".to_string()),
                a.get_as_string("DY").unwrap_or("0".to_string()),
            ],
            plax: a.get_as_string("PLAX").unwrap_or("unset".to_string()),
        };
        let x = super::resolve_expression(&param.vxy[0], refno, false).await?;
        let y = super::resolve_expression(&param.vxy[1], refno, false).await?;
        let dx = super::resolve_expression(&param.dxy[0], refno, false).await?;
        let dy = super::resolve_expression(&param.dxy[1], refno, false).await?;
        let plax = parse_expr_to_dir(&param.plax)
            .unwrap_or(DVec3::Y)
            .normalize();
        let plin_data = PlinParamData {
            pt: DVec3::new(x, y, 0.0) + DVec3::new(dx, dy, 0.0) * plax,
            plax,
        };
        if p_key == jusl {
            return Ok(Some(plin_data));
        }
    }
    Ok(None)
}

/// 计算ZDIS和PKDI, refno 是有这个SPLINE属性或者SCTN这种的参考号
pub async fn cal_zdis_pkdi_in_section(refno: RefU64, pkdi: f32, zdis: f32) -> (DQuat, DVec3) {
    let mut pos = DVec3::default();
    let mut quat = DQuat::IDENTITY;
    let mut spline_paths = get_spline_path(refno).await.unwrap();

    let mut sweep_paths = spline_paths
        .iter()
        .map(|x| x.generate_paths().0)
        .flatten()
        .collect::<Vec<_>>();
    let lens: Vec<f32> = sweep_paths.iter().map(|x| x.length()).collect::<Vec<_>>();
    let total_len: f32 = lens.iter().sum();
    // dbg!(&spline_paths);
    // dbg!(&sweep_paths);
    if spline_paths.is_empty() {
        return (quat, pos);
    }
    let world_mat4 = Box::pin(get_world_mat4(refno, false))
        .await
        .unwrap_or_default()
        .unwrap_or_default();
    let (_, w_quat, _) = world_mat4.to_scale_rotation_translation();
    let mut tmp_dist = zdis as f64;
    let mut tmp_porp = pkdi.clamp(0.0, 1.0);
    let start_len = (total_len * tmp_porp) as f64;
    //pkdi 给了一个比例的距离
    tmp_dist += start_len;
    //后续要考虑反方向的情况
    let mut cur_len = 0.0;
    for (i, path) in sweep_paths.into_iter().enumerate() {
        tmp_dist -= cur_len;
        cur_len = lens[i] as f64;
        //在第一段范围内，或者是最后一段，就没有长度的限制
        if tmp_dist > cur_len || i == lens.len() - 1 {
            match path {
                SweepPath3D::Line(l) => {
                    let mut z_dir = get_spline_line_dir(refno)
                        .await
                        .unwrap_or_default()
                        .normalize_or_zero();
                    // dbg!(&l);
                    if z_dir.length() == 0.0 {
                        z_dir = DVec3::Z;
                        let mut y_dir = world_mat4.z_axis.truncate();
                        if y_dir.normalize().dot(DVec3::Z).abs() > 0.999 {
                            y_dir = DVec3::X
                        };
                        let x_dir = y_dir.cross(z_dir).normalize();
                        quat = DQuat::from_mat3(&DMat3::from_cols(x_dir, y_dir, z_dir));
                    } else {
                        quat = cal_ori_by_extru_axis(z_dir, false);
                        z_dir = DMat3::from_quat(quat).z_axis;

                        quat = w_quat * quat;
                    }

                    let spine = &spline_paths[i];
                    // dbg!(spine);
                    //l 的数据都是从0开始的，所以这里不需要加上 start
                    pos += z_dir * tmp_dist + spine.pt0.as_dvec3();
                    // dbg!(dir);
                    // dbg!(dquat_to_pdms_ori_xyz_str(&quat));
                    break;
                }
                SweepPath3D::SpineArc(arc) => {
                    //使用弧长去计算当前的点的位置
                    if arc.radius > LEN_TOL {
                        let arc_center = arc.center.as_dvec3();
                        let arc_radius = arc.radius as f64;
                        let v = (arc.start_pt.as_dvec3() - arc_center).normalize();
                        let mut start_angle = DVec3::X.angle_between(v);
                        if DVec3::X.cross(v).z < 0.0 {
                            start_angle = -start_angle;
                        }
                        let mut theta = (tmp_dist / arc_radius);
                        if arc.clock_wise {
                            theta = -theta;
                        }
                        theta = start_angle + theta;
                        pos = arc_center + arc_radius * DVec3::new(theta.cos(), theta.sin(), 0.0);
                        let y_axis = DVec3::Z;
                        let mut x_axis = (arc_center - pos).normalize();
                        // dbg!(x_axis);
                        if arc.clock_wise {
                            x_axis = -x_axis;
                        }
                        // dbg!(arc.clock_wise);
                        let z_axis = x_axis.cross(y_axis).normalize();
                        // dbg!((x_axis, y_axis, z_axis));
                        quat = DQuat::from_mat3(&DMat3::from_cols(x_axis, y_axis, z_axis));
                        // dbg!(dquat_to_pdms_ori_xyz_str(&quat));
                        quat = w_quat * quat;
                    }
                }
                _ => {}
            }
        }
    }
    (quat, pos)
}

pub async fn get_spline_path(refno: RefU64) -> anyhow::Result<Vec<Spine3D>> {
    let type_name = crate::get_type_name(refno).await?;
    // dbg!(&type_name);
    let mut paths = vec![];
    if type_name == "GENSEC" || type_name == "WALL" {
        let children_refs = crate::get_children_refnos(refno).await.unwrap_or_default();
        // dbg!(&children_refs);
        for &x in children_refs.iter() {
            let spine_att = crate::get_named_attmap(x).await?;
            // dbg!(&spine_att.get_type_str());
            if spine_att.get_type_str() != "SPINE" {
                continue;
            }
            let ch_atts = crate::get_children_named_attmaps(x).await.unwrap_or_default();
            let len = ch_atts.len();
            if len < 1 { continue; }

            let mut i = 0;
            while i < ch_atts.len() - 1 {
                let att1 = &ch_atts[i];
                let t1 = att1.get_type_str();
                let att2 = &ch_atts[(i+1)%len];
                let t2 = att2.get_type_str();
                if t1 == "POINSP" && t2 == "POINSP" {
                    paths.push(Spine3D {
                        refno: att1.get_refno().unwrap(),
                        pt0: att1.get_position().unwrap_or_default(),
                        pt1: att2.get_position().unwrap_or_default(),
                        curve_type: SpineCurveType::LINE,
                        preferred_dir: spine_att.get_vec3("YDIR").unwrap_or(Vec3::Z),
                        ..Default::default()
                    });
                    // dbg!(&paths);
                    i += 1;
                } else if t1 == "POINSP" && t2 == "CURVE" {
                    let att3 = &ch_atts[(i+2)%len];
                    let pt0 = att1.get_position().unwrap_or_default();
                    let pt1 = att3.get_position().unwrap_or_default();
                    let mid_pt = att2.get_position().unwrap_or_default();
                    let cur_type_str = att2.get_str("CURTYP").unwrap_or("unset");
                    let curve_type = match cur_type_str {
                        "CENT" => { SpineCurveType::CENT }
                        "THRU" => { SpineCurveType::THRU }
                        _ => { SpineCurveType::UNKNOWN }
                    };
                    paths.push(Spine3D {
                        refno: att2.get_refno().unwrap(),
                        pt0,
                        pt1,
                        thru_pt: mid_pt,
                        center_pt: mid_pt,
                        cond_pos: att2.get_vec3("CPOS").unwrap_or_default(),
                        curve_type,
                        preferred_dir: spine_att.get_vec3("YDIR").unwrap_or(Vec3::Z),
                        radius: att2.get_f32("RAD").unwrap_or_default(),
                    });
                    i += 2;
                }
            }
        }
    }

    // dbg!(&paths);

    Ok(paths)
}

///沿着 dir 方向找到最近的目标构件
pub async fn query_neareast_along_axis(
    refno: RefU64,
    dir: Vec3,
    target_type: &str,
) -> anyhow::Result<Option<(RefU64, f32)>> {
    let pos = get_world_transform(refno)
        .await?
        .unwrap_or_default()
        .translation;
    //不用 room 的方法查询一次，直接用射线去查找
    let ray = Ray::new(pos.into(), dir.into());
    // dbg!(&ray);
    let rtree = GLOBAL_AABB_TREE.read().await;
    let mut filter = HashSet::new();
    filter.insert(target_type.to_string());
    let nearest = rtree
        .query_nearest_by_ray(QueryRay::new(ray, filter, true))
        .await;

    nearest
}

pub async fn query_neareast_by_pos_dir(
    pos: Vec3,
    dir: Vec3,
    target_type: &str,
) -> anyhow::Result<Option<(RefU64, f32)>> {
    // let pos = get_world_transform(refno).await?.unwrap_or_default().translation;
    //不用 room 的方法查询一次，直接用射线去查找
    let ray = Ray::new(pos.into(), dir.into());
    // dbg!(&ray);
    let rtree = GLOBAL_AABB_TREE.read().await;
    let mut filter = HashSet::new();
    filter.insert(target_type.to_string());
    let nearest = rtree
        .query_nearest_by_ray(QueryRay::new(ray, filter, true))
        .await;

    nearest
}
