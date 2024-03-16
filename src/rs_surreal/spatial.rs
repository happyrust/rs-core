use std::{f32::consts::E, collections::HashSet, time::Instant};
use crate::room::room::GLOBAL_AABB_TREE;
use crate::{
    consts::HAS_PLIN_TYPES,
    pdms_data::{PlinParam, PlinParamData},
    prim_geo::spine::{Spine3D, SpineCurveType, SweepPath3D},
    shape::pdms_shape::LEN_TOL,
    tool::{
        direction_parse::parse_expr_to_dir,
        math_tool::{quat_to_pdms_ori_str, quat_to_pdms_ori_xyz_str},
    },
    NamedAttrMap, RefU64, SUL_DB, accel_tree::acceleration_tree::QueryRay,
};
use crate::tool::math_tool;
use approx::abs_diff_eq;
use async_recursion::async_recursion;
use bevy_transform::prelude::*;
use cached::proc_macro::cached;
use glam::{DVec3, Mat3, Quat, Vec3};
use parry3d::query::Ray;
use serde::{Deserialize, Serialize};
use parry3d::bounding_volume::Aabb;
use serde_with::serde_as;
use serde_with::DisplayFromStr;

#[derive(Serialize, Deserialize, Debug)]
pub struct GeomInstQuery{
    #[serde(alias="id")]
    pub refno: RefU64,
    pub world_aabb: Aabb,
    pub world_trans: Transform,
    pub insts: Vec<ModelHashInst>,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ModelHashInst {
    #[serde_as(as = "DisplayFromStr")]
    pub geo_hash: u64,
    pub transform: Transform,
    #[serde(default)]
    pub is_tubi: bool,
}

//获得世界坐标系
///使用cache，需要从db manager里移除出来
///获得世界坐标系, 需要缓存数据，如果已经存在数据了，直接获取
#[async_recursion]
pub async fn get_world_transform(refno: RefU64) -> anyhow::Result<Option<Transform>> {
    let mut ancestors: Vec<NamedAttrMap> = super::get_ancestor_attmaps(refno).await?;
    ancestors.reverse();
    // dbg!(&ancestors);
    let mut rotation = Quat::IDENTITY;
    let mut translation = Vec3::ZERO;

    for atts in ancestors.windows(2) {
        let o_att = &atts[0];
        let att = &atts[1];
        let refno = att.get_refno().unwrap_or_default();
        let owner = o_att.get_refno_or_default();
        if refno == "25688/43205".into(){
            dbg!(refno);
        }
        let mut pos = att.get_position().unwrap_or_default();
        // dbg!(pos);
        let mut quat = Quat::IDENTITY;
        //土建特殊情况的一些处理
        if att.contains_key("ZDIS") {
            let zdist = att.get_f32("ZDIS").unwrap_or_default();
            let pkdi = att.get_f32("PKDI").unwrap_or_default();
            let result = cal_zdis_pkdi_in_section(owner, pkdi, zdist).await;
            pos += result.1;
            quat *= result.0;
        }

        if att.contains_key("NPOS") {
            let npos = att.get_vec3("NPOS").unwrap_or_default();
            pos += npos;
        }

        let owner_is_gensec = o_att.get_type() == "GENSEC";
        let quat_v = att.get_rotation();
        let mut need_bangle = false;
        if !owner_is_gensec && quat_v.is_some() {
            quat = quat_v.unwrap();
        } else {
            let (l_poss, l_pose) = if owner_is_gensec {
                //找到spine，获取spine的两个顶点
                let mut response = SUL_DB.query(
                    format!("select value <-pe_owner[where in.noun='SPINE'].in<-pe_owner.in.refno.POS from only pe:{}", owner)).await?;
                let pts: Vec<f32> = response.take(0)?;
                if pts.len() == 6 {
                    (
                        Some(Vec3::new(pts[0], pts[1], pts[2])),
                        Some(Vec3::new(pts[3], pts[4], pts[5])),
                    )
                } else {
                    (None, None)
                }
            } else {
                (att.get_poss(), att.get_pose())
            };
            if let (Some(poss), Some(pose)) = (l_poss, l_pose) {
                need_bangle = true;
                let extru_dir = (pose - poss).normalize();
                if !extru_dir.is_normalized() {
                    return Ok(None);
                }
                let d = extru_dir.dot(Vec3::Z).abs();
                let ref_axis = if abs_diff_eq!(1.0, d) {
                    Vec3::Y
                } else {
                    Vec3::Z
                };
                let p_axis = ref_axis.cross(extru_dir).normalize();
                let y_axis = extru_dir.cross(p_axis).normalize();
                quat = Quat::from_mat3(&Mat3::from_cols(p_axis, y_axis, extru_dir));
            }
        }

        let bangle = att.get_f32("BANG").unwrap_or_default();
        if need_bangle || att.contains_key("BANG") {
            quat = quat * Quat::from_rotation_z(bangle.to_radians());
        }
        //固定方位，不会怎旋转方向，但是会移动
        let fixed_posl_ori = att.get_type_str() == "ENDATU";

        //对于有CUTB的情况，需要直接对齐过去, 不需要在这里计算
        let c_ref = att.get_foreign_refno("CREF").unwrap_or_default();
        
        let mut cut_dir = Vec3::Y;
        let mut has_cut_back = false;
        //如果posl有，就不起用CUTB，相当于CUTB是一个手动对齐
        //直接在世界坐标系下求坐标，跳过局部求解
        if c_ref.is_valid() && att.get_str("POSL").is_none() && att.contains_key("CUTB") {
            cut_dir = att.get_vec3("CUTP").unwrap_or(cut_dir);
            let cut_len = att.get_f32("CUTB").unwrap_or_default();
            if let Ok(c_att) = super::get_named_attmap(c_ref).await {
                let c_t = get_world_transform(c_ref).await?.unwrap_or_default();
                if let (Some(poss), Some(pose)) = (c_att.get_poss(), c_att.get_pose()) {
                    let w_poss = c_t.translation;
                    let axis = pose - poss;
                    let len = axis.length();
                    let w_pose = w_poss + c_t.rotation * Vec3::Z * len;
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
                rotation = c_t.rotation;
                if let Some(opdir) = att.get_vec3("OPDI") {
                    //有自定义调节，需要选装到目标方向
                    let opdir = opdir.normalize();
                    rotation = rotation * Quat::from_rotation_arc(*c_t.local_z(), opdir);
                    #[cfg(feature = "debug")]
                    dbg!(quat_to_pdms_ori_xyz_str(&quat));
                }
            }
            // dbg!(has_cut_back);
        }
        //todo fix 处理 posl的计算
        if att.contains_key("POSL") {
            let pos_line = att.get_str("POSL").unwrap_or("NA");
            let pos_line = if pos_line.is_empty() { "NA" } else { pos_line };
            let delta_vec = att.get_vec3("DELP").unwrap_or_default();
            // dbg!(pos_line);
            //plin里的位置偏移
            let mut plin_pos = Vec3::ZERO;
            let mut pline_plax = Vec3::X;
            let owner = att.get_owner();
            // POSL 的处理, 获得父节点的形集, 自身的形集处理，已经在profile里处理过
            let mut is_lmirror = false;
            let ance_result =
                crate::query_filter_ancestors(owner, HAS_PLIN_TYPES.map(String::from).to_vec())
                    .await?;
            if let Some(plin_owner) = ance_result.into_iter().next() {
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
                if let Ok(Some(own_param)) =
                    crate::query_pline(plin_owner, own_pos_line.into()).await
                {
                    plin_pos -= own_param.pt;
                    #[cfg(feature = "debug")]
                    {
                        dbg!(own_pos_line);
                        dbg!(&own_param);
                    }
                }
            }
            let y_axis = if att.contains_key("YDIR") {
                att.get_vec3("YDIR").unwrap_or_default()
            } else {
                Vec3::Z
            };
            //和LMIRROR 有关系
            let z_axis = if is_lmirror { -pline_plax } else { pline_plax };
            let x_axis = y_axis.cross(z_axis).normalize();
            let posl_quat = if fixed_posl_ori {
                Quat::IDENTITY
            } else {
                Quat::from_mat3(&Mat3::from_cols(x_axis, y_axis, z_axis))
            };
            // dbg!((x_axis, y_axis, z_axis));
            let new_quat = posl_quat * quat;
            translation += rotation * (pos + plin_pos) + rotation * new_quat * delta_vec;


            //没有POSL时，需要使用cutback的方向
            rotation = rotation * new_quat;
            if pos_line == "unset" && has_cut_back {
                // dbg!(has_cut_back);
                //need to perpendicular to the Y axis
                let mat3 = Mat3::from_quat(rotation);
                let y_axis = mat3.y_axis;
                let ref_axis = cut_dir;
                // dbg!(cut_dir);
                #[cfg(feature = "debug")]
                {
                    dbg!(cut_dir);
                }
                let x_axis = y_axis.cross(ref_axis).normalize();
                let z_axis = x_axis.cross(y_axis).normalize();
                let new_mat = Mat3::from_cols(x_axis, y_axis, z_axis);
                // dbg!(new_mat);
                rotation = Quat::from_mat3(&new_mat);
            }
        } else {
            translation = translation + rotation * pos;
            rotation = rotation * quat;
        }


        let trans = Transform {
            rotation,
            translation,
            scale: Vec3::ONE,
        };
        if trans.is_nan() {
            return Ok(None);
        }
        //将rotation 还原为角度
        #[cfg(feature = "debug")]
        {
            let rot_mat = Mat3::from_quat(rotation);
            let ori_str = math_tool::to_pdms_ori_xyz_str(&rot_mat);
            println!("{} : {:?}", refno.to_string(), (translation, ori_str));
        }
    }

    if rotation.is_nan() || translation.is_nan() {
        return Ok(None);
    }
    Ok(Some(Transform {
        rotation,
        translation,
        scale: Vec3::ONE,
    }))
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
        let x = super::resolve_expression_to_f32(&param.vxy[0], refno, false).await?;
        let y = super::resolve_expression_to_f32(&param.vxy[1], refno, false).await?;
        let dx = super::resolve_expression_to_f32(&param.dxy[0], refno, false).await?;
        let dy = super::resolve_expression_to_f32(&param.dxy[1], refno, false).await?;
        let plax = parse_expr_to_dir(&param.plax)
            .unwrap_or(DVec3::Y)
            .normalize()
            .as_vec3();
        let plin_data = PlinParamData {
            pt: Vec3::new(x, y, 0.0) + Vec3::new(dx, dy, 0.0) * plax,
            plax,
        };
        if p_key == jusl {
            return Ok(Some(plin_data));
        }
    }
    Ok(None)
}

/// 计算ZDIS和PKDI, refno 是有这个SPLINE属性或者SCTN这种的参考号
pub async fn cal_zdis_pkdi_in_section(refno: RefU64, pkdi: f32, zdis: f32) -> (Quat, Vec3) {
    let mut pos = Vec3::default();
    let mut quat = Quat::IDENTITY;
    let mut spline_paths = get_spline_path(refno).await.unwrap_or_default();
    let mut sweep_paths = spline_paths
        .iter()
        .map(|x| x.generate_paths().0)
        .flatten()
        .collect::<Vec<_>>();
    let lens: Vec<f32> = sweep_paths.iter().map(|x| x.length()).collect::<Vec<_>>();
    let total_len: f32 = lens.iter().sum();
    // dbg!(&spline_paths);
    if spline_paths.is_empty() {
        return (quat, pos);
    }
    let mut tmp_dist = zdis;
    let mut tmp_porp = pkdi.clamp(0.0, 1.0);
    let start_len = total_len * tmp_porp;
    //pkdi 给了一个比例的距离
    tmp_dist += start_len;
    //后续要考虑反方向的情况
    let mut cur_len = 0.0;
    for (i, path) in sweep_paths.into_iter().enumerate() {
        tmp_dist -= cur_len;
        cur_len = lens[i];
        //在第一段范围内，或者是最后一段，就没有长度的限制
        if tmp_dist > cur_len || i == lens.len() - 1 {
            match path {
                SweepPath3D::Line(l) => {
                    let mut dir = (l.end - l.start).normalize();
                    pos += dir * tmp_dist + l.start;
                    break;
                }
                SweepPath3D::SpineArc(arc) => {
                    //使用弧长去计算当前的点的位置
                    if arc.radius > LEN_TOL {
                        let v = (arc.start_pt - arc.center).normalize();
                        let mut start_angle = Vec3::X.angle_between(v);
                        if Vec3::X.cross(v).z < 0.0 {
                            start_angle = -start_angle;
                        }
                        let mut theta = (tmp_dist / arc.radius);
                        if arc.clock_wise {
                            theta = -theta;
                        }
                        theta = start_angle + theta;
                        pos = arc.center + arc.radius * Vec3::new(theta.cos(), theta.sin(), 0.0);
                        let y_axis = Vec3::Z;
                        let mut x_axis = (arc.center - pos).normalize();
                        if arc.clock_wise {
                            x_axis = -x_axis;
                        }
                        let z_axis = x_axis.cross(y_axis).normalize();
                        quat = Quat::from_mat3(&Mat3::from_cols(x_axis, y_axis, z_axis));
                        // dbg!(quat_to_pdms_ori_str(&quat));
                    }
                }
                _ => {}
            }
        }
    }
    (quat, pos)
}

pub async fn get_spline_path(refno: RefU64) -> anyhow::Result<Vec<Spine3D>> {
    let children_refs = super::get_children_refnos(refno).await?;
    let mut paths = vec![];
    for x in children_refs {
        let type_name = super::get_type_name(x).await?;
        if type_name != "SPINE" {
            continue;
        }
        let spine_att = super::get_named_attmap(x).await?;
        let children_atts = super::get_children_named_attmaps(x).await?;
        if (children_atts.len() - 1) % 2 == 0 {
            for i in 0..(children_atts.len() - 1) / 2 {
                let att1 = &(children_atts[2 * i]);
                let att2 = &(children_atts[2 * i + 1]);
                let att3 = &(children_atts[2 * i + 2]);
                let pt0 = att1.get_position().unwrap_or_default();
                let pt1 = att3.get_position().unwrap_or_default();
                let mid_pt = att2.get_position().unwrap_or_default();
                let cur_type_str = att2.get_str("CURTYP").unwrap_or("unset");
                let curve_type = match cur_type_str {
                    "CENT" => SpineCurveType::CENT,
                    "THRU" => SpineCurveType::THRU,
                    _ => SpineCurveType::UNKNOWN,
                };
                paths.push(Spine3D {
                    pt0,
                    pt1,
                    thru_pt: mid_pt,
                    center_pt: mid_pt,
                    cond_pos: att2.get_vec3("CPOS").unwrap_or_default(),
                    curve_type,
                    preferred_dir: spine_att.get_vec3("YDIR").unwrap_or(Vec3::Z),
                    radius: att2.get_f32("RADI").unwrap_or_default(),
                });
            }
        } else if children_atts.len() == 2 {
            let att1 = &children_atts[0];
            let att2 = &children_atts[1];
            let pt0 = att1.get_position().unwrap_or_default();
            let pt1 = att2.get_position().unwrap_or_default();
            if att1.get_type_str() == "POINSP" && att2.get_type_str() == "POINSP" {
                paths.push(Spine3D {
                    pt0,
                    pt1,
                    curve_type: SpineCurveType::LINE,
                    preferred_dir: spine_att.get_vec3("YDIR").unwrap_or(Vec3::Z),
                    ..Default::default()
                });
            }
        }
    }

    //考虑sctn这种直接拉升出来的情况
    if paths.is_empty() {
        let att = super::get_named_attmap(refno).await?;
        if let Some(poss) = att.get_poss()
            && let Some(pose) = att.get_pose()
        {
            paths.push(Spine3D {
                pt0: poss,
                pt1: pose,
                curve_type: SpineCurveType::LINE,
                preferred_dir: Vec3::Z,
                ..Default::default()
            });
        }
    }

    Ok(paths)
}


///沿着 dir 方向找到最近的目标构件
pub async fn query_neareast_along_axis(refno: RefU64, dir: Vec3, target_type: &str) -> anyhow::Result<RefU64> {
    dbg!(refno);
    let pos = get_world_transform(refno).await?.unwrap_or_default().translation;
    //不用 room 的方法查询一次，直接用射线去查找
    let ray = Ray::new(pos.into(), dir.into());
    // dbg!(&ray);
    let rtree = GLOBAL_AABB_TREE.read().await;
    let mut filter = HashSet::new();
    filter.insert(target_type.to_string());
    let nearest = rtree.query_nearest_by_ray(
        QueryRay::new(ray, filter, true)
    ).await;
    //查询了之后，过滤 type
    //然后加入是否使用 mesh 去判断最终的结果

    Ok(nearest)
}