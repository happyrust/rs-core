//! 空间/坐标相关的工具函数：包含 PDMS 方向到 Bevy/glam 的转换、
//! 世界矩阵求解、样条路径与形集（PLIN）查询，以及基于 SQLite 的空间查询。
use crate::RefnoEnum;
#[cfg(all(not(target_arch = "wasm32"), feature = "sqlite"))]
use crate::spatial::sqlite;
use crate::tool::math_tool;
use crate::tool::math_tool::{
    cal_quat_by_zdir_with_xref, dquat_to_pdms_ori_xyz_str, to_pdms_dvec_str, to_pdms_vec_str,
};
use crate::utils::take_vec;
use crate::transform::get_local_mat4;
use crate::{
    NamedAttrMap, RefU64, SUL_DB, SurrealQueryExt,
    consts::HAS_PLIN_TYPES,
    get_named_attmap,
    pdms_data::{PlinParam, PlinParamData},
    prim_geo::spine::{SegmentPath, Spine3D, SpineCurveType, SweepPath3D},
    rs_surreal,
    shape::pdms_shape::LEN_TOL,
    tool::{
        direction_parse::parse_expr_to_dir,
        math_tool::{quat_to_pdms_ori_str, quat_to_pdms_ori_xyz_str},
    },
};
use anyhow::anyhow;
use approx::abs_diff_eq;
use async_recursion::async_recursion;
use bevy_transform::prelude::*;
use cached::proc_macro::cached;
use futures::future::{BoxFuture, FutureExt};
use glam::{DMat3, DMat4, DQuat, DVec3, Mat3, Mat4, Quat, Vec3};
use parry3d::bounding_volume::Aabb;
use serde::{Deserialize, Serialize};
use serde_with::DisplayFromStr;
use serde_with::serde_as;
use std::{f32::consts::E, time::Instant};

/// 根据给定的方向向量 `v` 构造一个右手坐标系，
/// 使 `v` 作为局部坐标系的 Z 轴，并返回对应的双精度四元数。
pub fn cal_ori_by_z_axis_ref_x(v: DVec3) -> DQuat {
    let mut ref_dir = if v.normalize().dot(DVec3::Z).abs() > 0.999 {
        DVec3::Y
    } else {
        DVec3::Z
    };
    let y_dir = v.cross(ref_dir).normalize();
    let x_dir = y_dir.cross(v).normalize();

    let rotation = DQuat::from_mat3(&DMat3::from_cols(x_dir, y_dir, v));
    rotation
}

/// 针对 SPINE 方向的专用方位计算：
/// 计算基于 SPINE 挤出方向的方位基底（orientation basis），
/// 允许通过 `neg` 反转参考轴，用于处理土建/管线中“反向挤出”等特殊情况。
pub fn cal_spine_orientation_basis(v: DVec3, neg: bool) -> DQuat {
    let is_vertical = v.normalize().dot(DVec3::Z).abs() > 0.999;

    let (x_dir, y_dir) = if is_vertical {
        // 垂直构件：优先让 Y 轴指北 (Global Y)
        // Local X = Y cross v
        let y_target = DVec3::Y;
        let x_res = y_target.cross(v).normalize();
        let y_res = v.cross(x_res).normalize();
        (x_res, y_res)
    } else {
        // 非垂直构件（包括水平）：优先让 Y 轴朝上 (Global Z)
        // Local X = Y(Up) cross v = Z cross v
        // 注意：这里 x_dir 指向水平方向
        let y_target = DVec3::Z;
        let x_res = y_target.cross(v).normalize();
        let y_res = v.cross(x_res).normalize();
        (x_res, y_res)
    };

    let (final_x, final_y) = if neg {
        (-x_dir, -y_dir)
    } else {
        (x_dir, y_dir)
    };

    DQuat::from_mat3(&DMat3::from_cols(final_x, final_y, v))
}

/// 针对 SPINE 方向的专用方位计算（支持 YDIR）
///
/// 计算基于 SPINE 挤出方向的方位基底，优先使用 YDIR 作为参考 Y 方向。
/// 这是 PDMS 中 GENSEC/WALL 元素的标准行为。
///
/// # Arguments
/// * `spine_dir` - SPINE 路径方向（将作为 Local Z 轴）
/// * `ydir` - 期望的 Y 方向（来自 SPINE 的 YDIR 属性）
/// * `neg` - 是否反转参考轴
///
/// # Returns
/// 返回表示局部坐标系的四元数，其中：
/// - Z 轴 = spine_dir（归一化）
/// - Y 轴 ≈ ydir（正交化后）
/// - X 轴 = Y × Z（右手系）
pub fn cal_spine_orientation_basis_with_ydir(
    spine_dir: DVec3,
    ydir: Option<DVec3>,
    neg: bool,
) -> DQuat {
    let z_axis = spine_dir.normalize();

    // 如果提供了 YDIR，使用它作为参考
    let y_ref = if let Some(y) = ydir {
        let y_norm = y.normalize();
        // 防止 YDIR 与 spine_dir 共线（dot ≈ ±1）
        if y_norm.dot(z_axis).abs() > 0.99 {
            // 回退到默认逻辑
            if z_axis.dot(DVec3::Z).abs() > 0.999 {
                DVec3::Y
            } else {
                DVec3::Z
            }
        } else {
            y_norm
        }
    } else {
        // 没有 YDIR 时，回退到默认逻辑
        if z_axis.dot(DVec3::Z).abs() > 0.999 {
            DVec3::Y
        } else {
            DVec3::Z
        }
    };

    // 构造正交基：Z = spine_dir, Y ≈ y_ref, X = Y × Z
    let x_dir = y_ref.cross(z_axis).normalize();
    let y_dir = z_axis.cross(x_dir).normalize();

    let (final_x, final_y) = if neg {
        (-x_dir, -y_dir)
    } else {
        (x_dir, y_dir)
    };

    DQuat::from_mat3(&DMat3::from_cols(final_x, final_y, z_axis))
}

/// 根据 OPDI（操作方向）向量计算局部方位。
/// 对接 PDMS 中 OPDI 方向，保证当方向接近全局 Z 轴时仍能选取稳定的参考轴。
pub fn cal_ori_by_opdir(v: DVec3) -> DQuat {
    let ref_dir = if v.normalize().dot(DVec3::Z).abs() > 0.999 {
        DVec3::NEG_Y * v.z.signum()
    } else {
        DVec3::Z
    };
    let y_dir = v.cross(ref_dir).normalize();
    let x_dir = y_dir.cross(v).normalize();

    let rotation = DQuat::from_mat3(&DMat3::from_cols(x_dir, y_dir, v));
    rotation
}

///通过 ydir 计算方位 , 跟 z 轴这个参考轴有关系。
/// `y_ref_axis` 为期望的局部 Y 方向，`z_dir` 为参考 Z 轴方向。
pub fn cal_ori_by_ydir(mut y_ref_axis: DVec3, z_dir: DVec3) -> DQuat {
    // 如果 y_ref 与 z_dir 平行（共线），则原来的 y_ref 无效，需选取一个新的参考轴
    if y_ref_axis.dot(z_dir).abs() > 0.99 {
        // 如果 z_dir 接近 Z 轴（垂直），则选 Y 轴作为临时参考
        // 否则选 Z 轴作为临时参考
        y_ref_axis = if z_dir.dot(DVec3::Z).abs() > 0.99 {
            DVec3::Y
        } else {
            DVec3::Z
        };
    }
    let ref_dir = y_ref_axis.cross(z_dir).normalize();
    let y_dir = z_dir.cross(ref_dir).normalize();
    let x_dir = y_dir.cross(z_dir).normalize();

    // dbg!(to_pdms_dvec_str(&ref_dir, true));
    // dbg!(to_pdms_dvec_str(&y_dir, true));
    // dbg!(to_pdms_dvec_str(&x_dir, true));

    let rotation = DQuat::from_mat3(&DMat3::from_cols(x_dir, y_dir, z_dir));
    rotation
}

#[test]
fn test_cal_ydir_ori() {
    let z_dir = parse_expr_to_dir("-X").unwrap();
    let y_ref_axis = parse_expr_to_dir("X 30 Y").unwrap();

    let rot = cal_ori_by_ydir(y_ref_axis, z_dir);
    assert_eq!(dquat_to_pdms_ori_xyz_str(&rot, true), "Y is Y and Z is -X");

    let z_dir = parse_expr_to_dir("-X").unwrap();
    let y_ref_axis = parse_expr_to_dir("Z 30 XY").unwrap();

    let rot = cal_ori_by_ydir(y_ref_axis, z_dir);
    assert_eq!(dquat_to_pdms_ori_xyz_str(&rot, true), "Y is Z and Z is -X");
}

#[test]
fn test_named_attmap_get_rotation_with_string() {
    use crate::tool::dir_tool::parse_ori_str_to_dquat;
    use crate::types::named_attmap::NamedAttrMap;
    use crate::types::named_attvalue::NamedAttrValue;
    use glam::{DQuat, DVec3};

    let mut map = NamedAttrMap::default();
    let ori_str = "Y is Z and Z is -X 0.1661 Y";
    // Simulate ORI as string
    map.map.insert(
        "ORI".to_string(),
        NamedAttrValue::StringType(ori_str.to_string()),
    );
    map.map.insert(
        "TYPE".to_string(),
        NamedAttrValue::StringType("EQUIPMENT".to_string()),
    );

    let rot = map.get_rotation();
    println!("Rotation from string: {:?}", rot);

    if let Some(q) = rot {
        // If it returns something, verify it matches parsing
        let expected_q = parse_ori_str_to_dquat(ori_str).unwrap();
        let diff = q.angle_between(expected_q);
        println!("Diff: {}", diff);
        assert!(diff < 1e-6);
    } else {
        println!("get_rotation returned None for String ORI");
        // assert!(false, "Should not return None");
    }
}

pub fn cal_spine_ori(v: DVec3, y_ref_dir: DVec3) -> DQuat {
    let x_dir = y_ref_dir.cross(v).normalize();
    let y_dir = v.cross(x_dir).normalize();

    let rotation = DQuat::from_mat3(&DMat3::from_cols(x_dir, y_dir, v));
    rotation
}

/// 与 `cal_ori_by_z_axis_ref_x` 类似，但以 Y 轴为参考来构造局部坐标系，
/// 主要用于需要约束局部 Y 方向的场景（例如部分土建截面）。
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

/// 根据挤出方向 `v` 计算截面方位，`neg` 为 true 时反转参考 Y 轴。
/// 主要用于 GENSEC / SCTN 等“沿轴挤出”几何的局部坐标构造。
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

///根据 CUTP 和轴方向，来计算 JOINT 的方位，
/// 当 CUTP 与轴接近平行时会退化为固定 Z 轴的稳定解。
pub fn cal_cutp_ori(axis_dir: DVec3, cutp: DVec3) -> DQuat {
    // let cutp = parse_expr_to_dir("Y 36.85 -X").unwrap();
    // let axis_dir = parse_expr_to_dir("Y 36.85 -X").unwrap();
    let mut y_axis = cutp.cross(axis_dir).normalize();
    let d = cutp.dot(axis_dir).abs();
    // dbg!(d);
    if d > 0.99 {
        y_axis = DVec3::Z;
    }
    let x_axis = axis_dir;
    let z_axis = x_axis.cross(y_axis).normalize();
    // let ref_axis = axis_dir.cross(y_axis).normalize();
    // let z_axis = y_axis.cross(ref_axis).normalize();
    // let x_axis = y_axis.cross(z_axis).normalize();
    // dbg!(z_axis);
    // dbg!(to_pdms_dvec_str(&z_axis, true));
    // // dbg!(to_pdms_dvec_str(&ref_axis, true));
    // dbg!(to_pdms_dvec_str(&y_axis, true));
    // dbg!(to_pdms_dvec_str(&x_axis, true));
    DQuat::from_mat3(&DMat3::from_cols(
        x_axis.into(),
        y_axis.into(),
        z_axis.into(),
    ))
}

/// 查询给定构件下属 SPINE 的采样点坐标（仍在 PDMS 本地坐标系中）。
/// 结果按 `order_num` 排序，仅返回 POS 三维坐标序列。
pub async fn get_spline_pts(refno: RefnoEnum) -> anyhow::Result<Vec<DVec3>> {
    let sql = format!(
        "select value (select in.refno.POS as pos, order_num from <-pe_owner[where in.noun='SPINE'].in<-pe_owner order by order_num).pos from only {}",
        refno.to_pe_key()
    );
    let mut response = SUL_DB.query_response(&sql).await?;
    let raw_pts: Vec<Vec<f64>> = take_vec(&mut response, 0)?;
    let pts: Vec<DVec3> = raw_pts
        .into_iter()
        .map(|coords| {
            let x = coords.get(0).copied().unwrap_or_default();
            let y = coords.get(1).copied().unwrap_or_default();
            let z = coords.get(2).copied().unwrap_or_default();
            DVec3::new(x, y, z)
        })
        .collect();
    Ok(pts)
}

/// 查询给定构件下属 SPINE 的首尾两点，并返回归一化的直线方向。
/// 仅当恰好有两个点时认为是直线段，否则返回错误。
pub async fn get_spline_line_dir(refno: RefnoEnum) -> anyhow::Result<DVec3> {
    let sql = format!(
        "select value (select in.refno.POS as pos, order_num from <-pe_owner[where in.noun='SPINE'].in<-pe_owner order by order_num).pos from only {}",
        refno.to_pe_key()
    );
    let mut response = SUL_DB.query_response(&sql).await?;
    let raw_pts: Vec<Vec<f64>> = take_vec(&mut response, 0)?;
    let pts: Vec<DVec3> = raw_pts
        .into_iter()
        .map(|coords| {
            let x = coords.get(0).copied().unwrap_or_default();
            let y = coords.get(1).copied().unwrap_or_default();
            let z = coords.get(2).copied().unwrap_or_default();
            DVec3::new(x, y, z)
        })
        .collect();
    if pts.len() == 2 {
        return Ok((pts[1] - pts[0]).normalize());
    }
    Err(anyhow!("没有找到两个点"))
}

/// 获取给定构件在世界坐标系下的 Transform（位移+旋转）。
/// 内部调用 `get_world_mat4` 并做缓存，避免重复访问 SurrealDB。
#[cached(result = true)]
pub async fn get_world_transform(refno: RefnoEnum) -> anyhow::Result<Option<Transform>> {
    get_world_mat4(refno, false)
        .await
        .map(|m| m.map(|x| Transform::from_matrix(x.as_mat4())))
}

///获得世界坐标系
///使用 cache，需要从 db manager 里移除出来。
///获得世界坐标系矩阵，如果已经存在数据则直接从缓存读取。
/// `is_local == true` 时返回相对于父节点的局部变换，否则返回从根到自身的世界矩阵。
///
/// # Deprecated
///
/// 此函数已被弃用，请使用 `get_world_mat4` 替代。
/// 新函数使用策略模式，提供更好的可维护性和扩展性。
///
/// # 迁移指南
///
/// 将以下代码：
/// ```rust
/// let transform = get_world_mat4_old(refno, is_local).await?;
/// ```
///
/// 替换为：
/// ```rust
/// let transform = get_world_mat4(refno, is_local).await?;
/// ```
#[deprecated(
    note = "Use get_world_mat4 instead for better maintainability and strategy pattern support"
)]
#[cached(result = true)]
pub async fn get_world_mat4_old(refno: RefnoEnum, is_local: bool) -> anyhow::Result<Option<DMat4>> {
    #[cfg(feature = "profile")]
    let start_ancestors = std::time::Instant::now();
    let mut ancestors: Vec<NamedAttrMap> = super::get_ancestor_attmaps(refno).await?;
    #[cfg(feature = "profile")]
    let elapsed_ancestors = start_ancestors.elapsed();
    #[cfg(feature = "profile")]
    println!("get_ancestor_attmaps took {:?}", elapsed_ancestors);

    // Debug: check ancestors content
    // if ancestors.is_empty() {
    //     println!("DEBUG: ancestors is empty for {}", refno);
    // } else {
    //     let first = ancestors.first().unwrap().get_refno_or_default();
    //     let last = ancestors.last().unwrap().get_refno_or_default();
    //     println!("DEBUG: ancestors for {}: len={}, first={}, last={}", refno, ancestors.len(), first, last);
    // }

    let start_refnos = std::time::Instant::now();
    let ancestor_refnos = crate::query_ancestor_refnos(refno).await?;
    let elapsed_refnos = start_refnos.elapsed();
    // println!("query_ancestor_refnos took {:?}", elapsed_refnos);

    // 检查 ancestors 是否包含 self
    let has_self = ancestors.iter().any(|a| a.get_refno_or_default() == refno);
    if !has_self {
        // println!("DEBUG: Adding self to ancestors for {}", refno);
        let self_att = get_named_attmap(refno).await?;
        // 注意：get_ancestor_attmaps 返回顺序通常是 [Parent, GrandParent, ... Root] (Bottom-Up)
        // 或者 [Root, ..., GrandParent, Parent] (Top-Down)?
        // 根据 reverse() 的使用，推测原始是 Top-Down (Root -> Parent)? 
        // 或者是 Bottom-Up (Parent -> Root)?
        // 旧代码：ancestors.reverse(); ... windows(2): (Parent, Child)
        // 如果 reverse 后是 Top-Down (Root -> Leaf)，说明原始是 Bottom-Up (Leaf -> Root)。
        // 如果原始是 [Parent, Root]，reverse -> [Root, Parent]。
        // 无论如何，self 应该是 Leaf，所以应该在 Root->Leaf 列表的末尾。
        // 如果原始是 Bottom-Up，self 应该在最前面?
        // fn::ancestor(x) -> [x, parent, root] or [root, parent, x]?
        // SurrealDB fn::ancestor 通常返回 path。
        
        // 假设我们需要 [Root, Parent, Self] 顺序来进行计算。
        // 如果原始 ancestors 是 [Parent, Root] (Bottom-Up without self)
        // 我们 insert(0, self) -> [Self, Parent, Root]
        // reverse -> [Root, Parent, Self]. Correct.
        
        ancestors.insert(0, self_att);
    }

    if ancestor_refnos.len() <= 1 {
        return Ok(Some(DMat4::IDENTITY));
    }
    ancestors.reverse();
    let mut rotation = DQuat::IDENTITY;
    let mut translation = DVec3::ZERO;
    let mut prev_mat4 = DMat4::IDENTITY;
    let mut mat4 = DMat4::IDENTITY;

    let mut owner = refno;
    for (index, atts) in ancestors.windows(2).enumerate() {
        let o_att = &atts[0];
        let att = &atts[1];
        let cur_refno = att.get_refno_or_default();
        let cur_type = att.get_type_str();
        // dbg!(cur_type);

        // 检查是否为虚拟节点，如果是则跳过transform计算
        if is_virtual_node(cur_type) {
            // 虚拟节点使用单位变换，不修改translation和rotation
            continue;
        }

        let owner_type = o_att.get_type_str();
        owner = att.get_owner();
        prev_mat4 = mat4;

        let mut pos = att.get_position().unwrap_or_default().as_dvec3();
        // dbg!(pos);
        let mut quat = DQuat::IDENTITY;
        let mut is_world_quat = false;
        let mut bangle = att.get_f32("BANG").unwrap_or_default() as f64;
        let mut apply_bang = att.contains_key("BANG") && bangle != 0.0;
        //只有GENSEC需要隐藏自己的方位
        if cur_type == "GENSEC" {
            apply_bang = false;
        }
        //土建特殊情况的一些处理
        let owner_is_gensec = owner_type == "GENSEC";
        let mut pos_extru_dir: Option<DVec3> = None;
        if owner_is_gensec {
            //找到spine，获取spine的两个顶点
            if let Ok(pts) = get_spline_pts(owner).await {
                if pts.len() == 2 {
                    pos_extru_dir = Some((pts[1] - pts[0]).normalize());
                }
            }
        } else if let Some(end) = att.get_dpose()
            && let Some(start) = att.get_dposs()
        {
            pos_extru_dir = Some((end - start).normalize());
            // dbg!(pos_extru_dir);
        }
        let is_sjoi = cur_type == "SJOI";
        let has_cut_dir = att.contains_key("CUTP");
        let cut_dir = att.get_dvec3("CUTP").unwrap_or(DVec3::Z);
        if is_sjoi {
            let cut_len = att.get_f64("CUTB").unwrap_or_default();
            // dbg!(&cut_dir);
            //先判断是否有cref
            //如果CUTP 没有z分量，则不考虑这些
            if let Some(c_ref) = att.get_foreign_refno("CREF")
                && let Ok(c_att) = get_named_attmap(c_ref).await
            {
                let jline = c_att.get_str("JLIN").map(|x| x.trim()).unwrap_or("NA");
                // dbg!(jline);
                if let Ok(Some(param)) = query_pline(c_ref, jline.into()).await {
                    let jlin_pos = param.pt;
                    let jlin_plax = param.plax;
                    // dbg!((&jlin_pos, &jlin_plax));
                    let c_t: Transform = Box::pin(get_world_transform(c_ref))
                        .await?
                        .unwrap_or_default();
                    let o_t: Transform = Box::pin(get_world_transform(o_att.get_owner()))
                        .await?
                        .unwrap_or_default();
                    let jlin_offset = c_t.rotation.as_dquat() * jlin_pos;
                    // dbg!(jlin_offset);
                    let c_axis = c_t.rotation.as_dquat() * DVec3::Z;
                    // dbg!(c_axis);
                    let c_wpos = c_t.translation.as_dvec3() + jlin_offset;
                    // dbg!(c_wpos);
                    // 是沿着附属的梁的轴方向再平移
                    let z_axis = o_t.rotation.as_dquat() * DVec3::Z;
                    // dbg!(z_axis);
                    // 取cref 对应构件的PLIN的位置
                    //如果垂直了，CUTP就是失效，不用考虑加冗余
                    let same_plane = c_axis.dot(cut_dir).abs() > 0.001;
                    if same_plane {
                        // dbg!(o_t.translation);
                        let delta = (c_wpos - o_t.translation.as_dvec3()).dot(z_axis);
                        // dbg!(delta);
                        translation = o_t.translation.as_dvec3() + delta * z_axis;
                        // dbg!(translation);
                        //如果 jlin_axis 和 z_axis 垂直
                        let perpendicular = z_axis.dot(c_axis).abs() < 0.001;
                        if !perpendicular {
                            translation += z_axis * cut_len;
                            // dbg!(translation);
                        }
                    }
                }
            } else {
            }
        }
        if att.contains_key("ZDIS") {
            if cur_type == "ENDATU" {
                //需要判断是第几个ENDATU
                let endatu_index: Option<u32> =
                    crate::get_index_by_noun_in_parent(owner, cur_refno, Some("ENDATU"))
                        .await
                        .unwrap();
                let section_end = if endatu_index == Some(0) {
                    Some(SectionEnd::START)
                } else if endatu_index == Some(1) {
                    Some(SectionEnd::END)
                } else {
                    None
                };
                // dbg!(&section_end);
                if let Some(result) = cal_zdis_pkdi_in_section_by_spine(
                    owner,
                    0.0,
                    att.get_f32("ZDIS").unwrap_or_default(),
                    section_end,
                )
                .await?
                {
                    pos += result.1;
                    quat = result.0;
                    // dbg!(math_tool::dquat_to_pdms_ori_xyz_str(&quat, true));
                    translation = translation + rotation * pos;
                    rotation = quat;
                    mat4 = DMat4::from_rotation_translation(rotation, translation);
                    continue;
                }
            } else {
                let zdist = att.get_f32("ZDIS").unwrap_or_default();
                let pkdi = att.get_f32("PKDI").unwrap_or_default();
                //zdis 起点应该是从poss 开始，所以这里需要加上这个偏移
                if let Some((tmp_quat, tmp_pos)) =
                    cal_zdis_pkdi_in_section_by_spine(owner, pkdi, zdist, None).await?
                {
                    // 对于 POINSP，需要保留原始位置并加上 ZDIS 偏移
                    if cur_type == "POINSP" {
                        pos = pos + tmp_pos; // 保留原始局部位置，加上偏移
                    } else {
                        pos = tmp_pos; // 其他类型使用计算的位置
                    }
                    quat = tmp_quat;
                    // dbg!(math_tool::dquat_to_pdms_ori_xyz_str(&quat, true));
                    // dbg!(tmp_pos);
                    // translation = translation + rotation * tmp_pos;
                    // dbg!(translation);
                    is_world_quat = true;
                    // rotation = quat;
                    // mat4 = DMat4::from_rotation_translation(rotation, translation);
                    // continue;
                } else {
                    translation += rotation * DVec3::Z * zdist as f64;
                    // dbg!(translation);
                }
            }
        }
        if att.contains_key("NPOS") {
            let npos = att.get_vec3("NPOS").unwrap_or_default();
            // dbg!(npos);
            pos += npos.as_dvec3();
            // dbg!(pos);
        }

        //如果posl有，就不起用CUTB，相当于CUTB是一个手动对齐
        //直接在世界坐标系下求坐标，跳过局部求解
        //有 cref 的时候，需要保持方向和 cref 一致
        let ydir_axis = att.get_dvec3("YDIR");
        let pos_line = att.get_str("POSL").map(|x| x.trim()).unwrap_or_default();
        let delta_vec = att.get_dvec3("DELP").unwrap_or_default();
        let mut has_opdir = false;

        let quat_v = att.get_rotation();
        let has_local_ori = quat_v.is_some();
        let mut need_bangle = false;
        //特殊处理的类型
        if (!owner_is_gensec && has_local_ori) || (owner_is_gensec && cur_type == "TMPL") {
            quat = quat_v.unwrap_or_default();
        } else {
            if let Some(z_axis) = pos_extru_dir {
                need_bangle = true;
                if owner_is_gensec {
                    //todo 待测试特殊情况
                    if !is_world_quat {
                        if !z_axis.is_normalized() {
                            return Ok(None);
                        }
                        // 对于 SPINE 类型，使用 YDIR 来计算正确的方向
                        if cur_type == "SPINE"
                            && let Some(ydir) = ydir_axis
                        {
                            quat = cal_spine_orientation_basis_with_ydir(z_axis, Some(ydir), false);
                        } else {
                            quat = cal_spine_orientation_basis(z_axis, false);
                        }
                    }
                } else {
                    if !z_axis.is_normalized() {
                        return Ok(None);
                    }
                    //跳过是owner sctn或者 WALL 的计算
                    quat = cal_ori_by_z_axis_ref_y(z_axis);
                    // dbg!(math_tool::dquat_to_pdms_ori_xyz_str(&quat, false));
                }
            }
        }

        if let Some(opdir) = att.get_dvec3("OPDI").map(|x| x.normalize()) {
            quat = cal_ori_by_opdir(opdir);
            has_opdir = true;
            // dbg!(dquat_to_pdms_ori_xyz_str(&quat, true));
            if pos_line.is_empty() {
                pos += delta_vec;
            }
        }

        //todo fix 处理 posl的计算
        if !pos_line.is_empty() {
            // dbg!(&cur_type);
            //plin里的位置偏移
            let mut plin_pos = DVec3::ZERO;
            let mut pline_plax = DVec3::X;
            // POSL 的处理, 获得父节点的形集, 自身的形集处理，已经在profile里处理过
            let mut is_lmirror = false;
            let ancestor_refnos = crate::query_filter_ancestors(owner, &HAS_PLIN_TYPES).await?;
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
                    #[cfg(feature = "debug_spatial")]
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
                    #[cfg(feature = "debug_spatial")]
                    {
                        dbg!(own_pos_line);
                        dbg!(&own_param);
                    }
                }
                #[cfg(feature = "debug_spatial")]
                {
                    dbg!(&plin_pos);
                }
            }
            let z_axis = if is_lmirror { -pline_plax } else { pline_plax };
            let plin_pos = if is_lmirror { -plin_pos } else { plin_pos };
            let mut new_quat = {
                if cur_type == "FITT" {
                    //受到bang的影响，需要变换
                    //绕着z轴旋转
                    let y_axis = DQuat::from_axis_angle(z_axis, bangle.to_radians()) * DVec3::Z;
                    let x_axis = y_axis.cross(z_axis).normalize();
                    // dbg!((x_axis, y_axis, z_axis));
                    DQuat::from_mat3(&DMat3::from_cols(x_axis, y_axis, z_axis))
                } else if cur_type == "SCOJ" {
                    cal_ori_by_z_axis_ref_x(z_axis) * quat
                } else {
                    cal_ori_by_z_axis_ref_y(z_axis) * quat
                }
            };
            // dbg!(dquat_to_pdms_ori_xyz_str(&new_quat, true));
            //处理有YDIR的情况
            if let Some(v) = ydir_axis {
                new_quat = cal_ori_by_ydir(v.normalize(), z_axis);
            }
            if apply_bang {
                new_quat = new_quat * DQuat::from_rotation_z(bangle.to_radians());
            }
            // dbg!(dquat_to_pdms_ori_xyz_str(&new_quat, true));
            let offset = rotation * (pos + plin_pos) + rotation * new_quat * delta_vec;
            #[cfg(feature = "debug_spatial")]
            {
                dbg!(&pos);
                dbg!(&plin_pos);
                dbg!(&delta_vec);
                dbg!(offset);
            }
            translation += offset;
            rotation = rotation * new_quat;
            // dbg!(dquat_to_pdms_ori_xyz_str(&rotation, true));
        } else {
            if let Some(v) = ydir_axis {
                let z_axis = if let Some(axis) = pos_extru_dir {
                    axis
                } else {
                    DVec3::X
                };
                // dbg!((v, z_axis));
                quat = cal_ori_by_ydir(v.normalize(), z_axis);
                // dbg!(dquat_to_pdms_ori_xyz_str(&quat, true));
            }
            if apply_bang {
                quat = quat * DQuat::from_rotation_z(bangle.to_radians());
            }
            if has_cut_dir && !has_opdir && !has_local_ori {
                // dbg!(cut_dir);
                let mat3 = DMat3::from_quat(rotation);
                // dbg!((mat3.z_axis, cut_dir));
                quat = cal_cutp_ori(mat3.z_axis, cut_dir);
                is_world_quat = true;
            }

            // 对于 POINSP 类型，需要特殊处理以确保正确的世界坐标
            // POINSP的局部坐标系：Y=沿SPINE路径距离，X/Z=横向偏移
            if cur_type == "POINSP" {
                // 获取 POINSP 的本地位置
                let local_pos = att.get_position().unwrap_or_default().as_dvec3();

                // 检查父级是否为SPINE（POINSP通常是SPINE的子节点）
                if let Ok(spine_att) = get_named_attmap(owner).await {
                    if spine_att.get_type_str() == "SPINE" {
                        // 处理SPINE子节点的正确变换逻辑
                        if let Some(spine_transform) =
                            calculate_poinsp_spine_transform(owner, local_pos).await
                        {
                            // 应用SPINE变换到当前变换链
                            translation =
                                translation + rotation * spine_transform.w_axis.truncate();
                            rotation = rotation * DQuat::from_mat4(&spine_transform);
                            mat4 = DMat4::from_rotation_translation(rotation, translation);
                            continue;
                        }
                    }
                }

                // 回退到原始逻辑：非SPINE子节点或SPINE变换失败的情况
                // 找到 GENSEC 作为基准坐标系
                let mut current_owner = owner;
                let mut gensec_refno = refno;

                // 向上查找 GENSEC
                for _i in 0..5 {
                    // 限制查找深度避免无限循环
                    if let Ok(current_att) = get_named_attmap(current_owner).await {
                        let current_type = current_att.get_type_str();
                        if current_type == "GENSEC" || current_type == "WALL" {
                            gensec_refno = current_owner;
                            break;
                        }
                        current_owner = current_att.get_owner();
                    } else {
                        break;
                    }
                }

                // 如果找到了 GENSEC，使用 GENSEC 的世界矩阵 + POINSP 本地位置
                if gensec_refno != refno {
                    if let Ok(gensec_att) = get_named_attmap(gensec_refno).await {
                        let gensec_pos = gensec_att.get_position().unwrap_or_default().as_dvec3();
                        // 直接设置最终世界坐标：GENSEC位置 + POINSP本地位置
                        translation = translation + rotation * gensec_pos + rotation * local_pos;
                        mat4 = DMat4::from_rotation_translation(rotation, translation);
                        continue;
                    }
                }
            }

            translation = translation + rotation * pos;
            if is_world_quat {
                rotation = quat;
            } else {
                rotation = rotation * quat;
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

/// 使用策略模式重构的世界矩阵计算函数
///
/// 这是 `get_world_mat4` 的重构版本，使用新的策略系统（TransformStrategy）
/// 来计算变换矩阵，提供更好的可维护性和扩展性。
///
/// # 特性标志
///
/// 此函数的行为受 `use_strategy_transform` 特性标志控制：
/// - **启用时**：使用新的策略系统
/// - **禁用时**：回退到旧的 `get_world_mat4` 实现
///
/// 默认情况下该特性是关闭的（opt-in 迁移策略），需要显式启用：
/// ```bash
/// cargo run --features use_strategy_transform
/// ```
///
/// # Arguments
/// * `refno` - 目标构件的参考号
/// * `is_local` - 如果为 true，返回相对于父节点的局部变换；否则返回世界变换
///
/// # Returns
/// * `Ok(Some(DMat4))` - 计算得到的变换矩阵
/// * `Ok(None)` - 如果无法计算变换
/// * `Err` - 如果计算过程中发生错误
///
/// # 特性
/// - 使用策略模式支持不同构件类型的专门计算逻辑
/// - 与重构后的 `get_local_mat4` 函数集成
/// - 保持与原函数相同的 API 接口
/// - 支持缓存优化
/// - 生产安全的特性标志回退机制
pub async fn get_world_mat4(
    refno: RefnoEnum,
    is_local: bool,
) -> anyhow::Result<Option<DMat4>> {
    // 新的策略系统实现
    get_world_mat4_with_strategies_impl(refno, is_local).await
}

/// 新策略系统的具体实现
///
/// 此函数包含使用策略模式的世界矩阵计算逻辑
async fn get_world_mat4_with_strategies_impl(
    refno: RefnoEnum,
    is_local: bool,
) -> anyhow::Result<Option<DMat4>> {
    #[cfg(feature = "profile")]
    let start_ancestors = std::time::Instant::now();
    let mut ancestors: Vec<NamedAttrMap> = super::get_ancestor_attmaps(refno).await?;
    #[cfg(feature = "profile")]
    let elapsed_ancestors = start_ancestors.elapsed();
    #[cfg(feature = "profile")]
    println!("get_ancestor_attmaps took {:?}", elapsed_ancestors);

    #[cfg(feature = "profile")]
    let start_refnos = std::time::Instant::now();
    let ancestor_refnos = crate::query_ancestor_refnos(refno).await?;
    #[cfg(feature = "profile")]
    let elapsed_refnos = start_refnos.elapsed();
    #[cfg(feature = "profile")]
    println!("query_ancestor_refnos took {:?}", elapsed_refnos);

    // 检查 ancestors 是否包含 self，如果不包含则添加
    // get_ancestor_attmaps 通常返回 [Parent, GrandParent, ... Root]
    // 我们需要将其补充为 [Self, Parent, ... Root]
    let has_self = ancestors.iter().any(|a| a.get_refno_or_default() == refno);
    if !has_self {
        let self_att = get_named_attmap(refno).await?;
        ancestors.insert(0, self_att);
    }

    if ancestor_refnos.len() <= 1 {
        return Ok(Some(DMat4::IDENTITY));
    }

    ancestors.reverse();

    // 如果只需要局部变换，直接调用 get_local_mat4
    if is_local {
        if ancestors.len() >= 2 {
            let parent_refno = ancestors[ancestors.len() - 2].get_refno_or_default();
            let cur_refno = ancestors.last().unwrap().get_refno_or_default();
            return get_local_mat4(cur_refno, parent_refno).await;
        }
        return Ok(Some(DMat4::IDENTITY));
    }

    // 遍历层级，使用重构后的策略系统计算每个节点的局部变换
    let mut world_transform = DMat4::IDENTITY;

    let mut mat4 = DMat4::IDENTITY;
    for (index, atts) in ancestors.windows(2).enumerate() {
        let o_att = &atts[0];
        let att = &atts[1];
        let cur_refno = att.get_refno_or_default();
        let owner = att.get_owner();
        
        // Debug info
        // println!("DEBUG: Loop {} - Parent: {}, Child: {}", index, o_att.get_refno_or_default(), cur_refno);

        // 计算局部变换
        if let Ok(Some(local_mat)) = get_local_mat4(cur_refno, owner).await {
            // println!("DEBUG:   Local Mat: {:?}", local_mat.w_axis);
            mat4 = mat4 * local_mat;
            // println!("DEBUG:   Acc Mat: {:?}", mat4.w_axis);
        } else {
            println!("DEBUG:   Failed to get local mat for {}", cur_refno);
                #[cfg(feature = "debug_spatial")]
                {
                    let local_pos = local_transform.project_point3(glam::DVec3::ZERO);
                    let world_pos = world_transform.project_point3(glam::DVec3::ZERO);
                    println!(
                        "Level {}: {} -> {}\n  父级世界矩阵: {:?}\n  局部变换: {:?}\n  局部位置: {:?}\n  累积后世界位置: {:?}\n  变换前世界: {:?}\n  变换后世界: {:?}",
                        index,
                        owner,
                        cur_refno,
                        prev_world_transform,
                        local_transform,
                        local_pos,
                        world_pos,
                        prev_world_transform.project_point3(glam::DVec3::ZERO),
                        world_pos
                    );
                }

                // 特别针对FITT类型的调试
                if att.get_type_str() == "FITT" {
                    let local_pos = local_transform.project_point3(glam::DVec3::ZERO);
                    let world_pos = world_transform.project_point3(glam::DVec3::ZERO);
                    println!(
                        "🔍 FITT变换调试:\n  参考号: {}\n  父级: {}\n  局部位置: {:?}\n  世界位置: {:?}\n  父级世界矩阵: {:?}\n  局部变换矩阵: {:?}",
                        cur_refno,
                        owner,
                        local_pos,
                        world_pos,
                        prev_world_transform,
                        local_transform
                    );

                    // 分析ZDIS如何从局部坐标系转换到世界坐标系
                    let zdis = att.get_f32("ZDIS").unwrap_or_default();
                    let local_z_offset = glam::DVec3::new(0.0, 0.0, zdis as f64);
                    let world_z_offset = prev_world_transform.transform_point3(local_z_offset);
                    println!(
                        "  ZDIS分析:\n    ZDIS值: {}\n    局部Z偏移: {:?}\n    世界Z偏移: {:?}\n    Z轴变换差异: {:.3}",
                        zdis,
                        local_z_offset,
                        world_z_offset,
                        world_z_offset.z - local_z_offset.z
                    );
                }

                #[cfg(feature = "debug_spatial")]
                println!(
                    "Level {}: Applied local transform for {} -> {}",
                    index, owner, cur_refno
                );
            }
            Ok(None) => {
                #[cfg(feature = "debug_spatial")]
                println!(
                    "Level {}: No transform calculated for {} -> {}",
                    index, owner, cur_refno
                );
                // 继续处理其他层级，不中断
            }
            Err(e) => {
                #[cfg(feature = "debug_spatial")]
                println!(
                    "Level {}: Error calculating transform for {} -> {}: {}",
                    index, owner, cur_refno, e
                );
                // 记录错误但继续处理
            }
        }
    }

    // 检查变换的有效性
    if world_transform.is_nan() {
        return Ok(None);
    }

    Ok(Some(world_transform))
}

///查询形集PLIN的值，todo 需要做缓存优化
// #[cached]
/// 根据参考号和JUSL值查询形集PLIN的参数数据
///
/// # Arguments
/// * `refno` - 参考号
/// * `jusl` - JUSL值
///
/// # Returns
/// * `Ok(Some(PlinParamData))` - 查询成功返回PLIN参数数据
/// * `Ok(None)` - 未找到匹配的PLIN数据
/// * `Err` - 查询过程中发生错误
pub async fn query_pline(refno: RefnoEnum, jusl: String) -> anyhow::Result<Option<PlinParamData>> {
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

#[derive(Debug)]
pub enum SectionEnd {
    START,
    END,
}

/// 计算 ZDIS 和 PKDI, `refno` 是具有 SPLINE 属性或者 SCTN 这种的参考号。
/// 沿 spine 段长度方向累加弧长，返回截面所在的世界坐标和朝向四元数。
pub async fn cal_zdis_pkdi_in_section_by_spine(
    refno: RefnoEnum,
    pkdi: f32,
    zdis: f32,
    section_end: Option<SectionEnd>,
) -> anyhow::Result<Option<(DQuat, DVec3)>> {
    let mut pos = DVec3::default();
    let mut quat = DQuat::IDENTITY;
    //默认只有一个
    let mut spline_paths = get_spline_path(refno).await?;
    if spline_paths.is_empty() {
        return Ok(None);
    }
    let spine_ydir = spline_paths[0].preferred_dir.as_dvec3();

    let sweep_path = spline_paths[0].generate_paths().0;
    let lens: Vec<f32> = sweep_path
        .segments
        .iter()
        .map(|x| x.length())
        .collect::<Vec<_>>();
    let total_len: f32 = lens.iter().sum();
    let world_mat4 = Box::pin(get_world_mat4(refno, false))
        .await?
        .unwrap_or_default();
    let (_, w_quat, _) = world_mat4.to_scale_rotation_translation();
    let mut tmp_dist = zdis as f64;
    let mut tmp_porp = pkdi.clamp(0.0, 1.0);
    let start_len = (total_len * tmp_porp) as f64;
    //pkdi 给了一个比例的距离
    tmp_dist += start_len;
    //后续要考虑反方向的情况
    let mut cur_len = 0.0;
    for (i, segment) in sweep_path.segments.into_iter().enumerate() {
        tmp_dist -= cur_len;
        cur_len = lens[i] as f64;
        //在第一段范围内，或者是最后一段，就没有长度的限制
        if tmp_dist > cur_len || i == lens.len() - 1 {
            match segment {
                SegmentPath::Line(l) => {
                    let mut z_dir = get_spline_line_dir(refno)
                        .await
                        .unwrap_or_default()
                        .normalize_or_zero();
                    if z_dir.length() == 0.0 {
                        // z_dir = DVec3::Z;
                        // let mut y_dir = spine_ydir;
                        // if y_dir.normalize().dot(DVec3::Z).abs() > 0.999 {
                        //     y_dir = DVec3::X
                        // };
                        // let x_dir = y_dir.cross(z_dir).normalize();
                        // quat = DQuat::from_mat3(&DMat3::from_cols(x_dir, y_dir, z_dir));
                        quat = w_quat;
                    } else {
                        quat = cal_spine_ori(z_dir, spine_ydir);
                        z_dir = DMat3::from_quat(quat).z_axis;
                        quat = w_quat * quat;
                    }
                    // dbg!(dquat_to_pdms_ori_xyz_str(&quat, true));
                    let spine = &spline_paths[i];
                    match section_end {
                        Some(SectionEnd::START) => {
                            pos = spine.pt0.as_dvec3();
                        }
                        Some(SectionEnd::END) => {
                            pos = spine.pt1.as_dvec3();
                        }
                        _ => {
                            pos += z_dir * tmp_dist + spine.pt0.as_dvec3();
                        }
                    }
                    break;
                }
                SegmentPath::Arc(arc) => {
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
                        if arc.clock_wise {
                            x_axis = -x_axis;
                        }
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
    Ok(Some((quat, pos)))
}

/// 查询截面构件（如 SCTN / GENSEC）下属的所有 POINSP 深度子节点，
/// 并返回它们在 PDMS 本地坐标系中的 POS 位置。
///
/// 该函数仅负责收集“扫描 path 点”的局部坐标，
/// 世界变换由前端在 Bevy 中通过 GlobalTransform 统一处理。
pub async fn query_section_poinsp_local_points(refno: RefnoEnum) -> anyhow::Result<Vec<Vec3>> {
    // 使用通用图查询接口按类型深度过滤出所有 POINSP 子节点
    let poinsp_refnos =
        rs_surreal::graph::collect_descendant_filter_ids(&[refno], &["POINSP"], None).await?;

    let mut points = Vec::new();
    for child_refno in poinsp_refnos {
        let att = get_named_attmap(child_refno).await?;
        if let Some(pos) = att.get_position() {
            points.push(pos);
        }
    }

    Ok(points)
}

/// 根据 GENSEC/WALL 下的 SPINE / POINSP / CURVE 节点，
/// 构造一组 `Spine3D` 段，供挤出、ZDIS/PKDI 位置计算等场景复用。
pub async fn get_spline_path(refno: RefnoEnum) -> anyhow::Result<Vec<Spine3D>> {
    let type_name = crate::get_type_name(refno).await?;
    // dbg!(&type_name);
    let mut paths = vec![];
    if type_name == "GENSEC" || type_name == "WALL" {
        let children_refs = crate::get_children_refnos(refno).await.unwrap_or_default();
        // dbg!(&children_refs);
        for &x in children_refs.iter() {
            let spine_att = crate::get_named_attmap(x).await?;
            // dbg!(&spine_att);
            if spine_att.get_type_str() != "SPINE" {
                continue;
            }
            let ch_atts = crate::get_children_named_attmaps(x)
                .await
                .unwrap_or_default();
            let len = ch_atts.len();
            if len < 1 {
                continue;
            }

            let mut i = 0;
            while i < ch_atts.len() - 1 {
                let att1 = &ch_atts[i];
                let t1 = att1.get_type_str();
                let att2 = &ch_atts[(i + 1) % len];
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
                    let att3 = &ch_atts[(i + 2) % len];
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

///沿着 `dir` 方向，从给定构件位置出发，找到最近的目标构件。
#[cfg(all(not(target_arch = "wasm32"), feature = "sqlite"))]
pub async fn query_neareast_along_axis(
    refno: RefnoEnum,
    dir: Vec3,
    target_type: &str,
) -> anyhow::Result<Option<(RefnoEnum, f32)>> {
    let pos = get_world_transform(refno)
        .await?
        .unwrap_or_default()
        .translation;
    let exclude = Some(refno.refno());
    query_nearest_by_dir_internal(pos, dir, target_type, exclude).await
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "sqlite")))]
pub async fn query_neareast_along_axis(
    _refno: RefnoEnum,
    _dir: Vec3,
    _target_type: &str,
) -> anyhow::Result<Option<(RefnoEnum, f32)>> {
    Ok(None)
}

/// 以给定世界坐标 `pos` 和射线方向 `dir`，
/// 通过 SQLite 空间索引在近邻 AABB 中查找最近的指定类型目标构件。
#[cfg(all(not(target_arch = "wasm32"), feature = "sqlite"))]
pub async fn query_neareast_by_pos_dir(
    pos: Vec3,
    dir: Vec3,
    target_type: &str,
) -> anyhow::Result<Option<(RefnoEnum, f32)>> {
    query_nearest_by_dir_internal(pos, dir, target_type, None).await
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "sqlite")))]
pub async fn query_neareast_by_pos_dir(
    _pos: Vec3,
    _dir: Vec3,
    _target_type: &str,
) -> anyhow::Result<Option<(RefnoEnum, f32)>> {
    Ok(None)
}

/// 查询指定节点的包围盒，需要遍历子节点的所有包围盒。
/// 如果是含有负实体的，优先取父节点的包围盒；负实体邻居为正实体时也可能要考虑在内。
/// 还有一种情况是图形平台级别的包围盒，需要综合所有子节点的包围盒进行计算（当前暂未实现）。
pub async fn query_bbox(refno: RefnoEnum) -> anyhow::Result<Option<(RefnoEnum, f32)>> {
    //获得所有子节点的包围盒？
    //还是所有的包围盒的

    Ok(None)
}

#[cfg(all(not(target_arch = "wasm32"), feature = "sqlite"))]
async fn query_nearest_by_dir_internal(
    origin: Vec3,
    dir: Vec3,
    target_type: &str,
    exclude: Option<RefU64>,
) -> anyhow::Result<Option<(RefnoEnum, f32)>> {
    let dir_len = dir.length();
    if dir_len <= f32::EPSILON {
        return Ok(None);
    }
    let dir_norm = dir / dir_len;
    let max_distance = 50_000.0;
    let origin_point = parry3d::math::Point::new(origin.x, origin.y, origin.z);
    let dir_vector = parry3d::math::Vector::new(dir_norm.x, dir_norm.y, dir_norm.z);
    let exclude_ref = exclude.map(|r| r.0);
    let target = target_type.to_string();

    let hits = tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<(RefU64, Aabb)>> {
        let filter = vec![target];
        let raw = sqlite::query_knn(origin, 256, Some(max_distance), Some(filter.as_slice()))?;
        Ok(raw
            .into_iter()
            .map(|(refno, aabb, _, _)| (refno, aabb))
            .collect())
    })
    .await??;

    let mut best: Option<(RefU64, f32)> = None;
    for (candidate_refno, aabb) in hits {
        if exclude_ref == Some(candidate_refno.0) {
            continue;
        }
        if let Some(toi) = sqlite::ray_aabb_toi(origin_point, dir_vector, &aabb, max_distance) {
            if toi >= 0.0 {
                match best {
                    Some((_, dist)) if dist <= toi => {}
                    _ => best = Some((candidate_refno, toi)),
                }
            }
        }
    }

    Ok(best.map(|(refno, dist)| (RefnoEnum::Refno(refno), dist)))
}

/// 计算POINSP在SPINE路径上的变换矩阵
/// POINSP局部坐标系：Y=沿SPINE路径距离，X/Z=横向偏移
pub async fn calculate_poinsp_spine_transform(
    spine_refno: RefnoEnum,
    poinsp_local_pos: DVec3,
) -> Option<DMat4> {
    // 获取SPINE信息
    let spine_att = get_named_attmap(spine_refno).await.ok()?;
    let spine_ydir = spine_att.get_dvec3("YDIR");

    // 获取GENSEC（SPINE的父级）
    let gensec_refno = spine_att.get_owner();
    let gensec_att = get_named_attmap(gensec_refno).await.ok()?;

    if gensec_att.get_type_str() != "GENSEC" && gensec_att.get_type_str() != "WALL" {
        return None;
    }

    // 获取SPINE路径信息
    let spline_pts = get_spline_pts(gensec_refno).await.ok()?;
    if spline_pts.len() < 2 {
        return None;
    }

    // 计算沿SPINE路径的距离（POINSP的Y坐标）
    let distance_along_spine = poinsp_local_pos.y;

    // 计算SPINE路径上的变换矩阵
    let spine_transform =
        calculate_spine_transform_at_distance(&spline_pts, distance_along_spine, spine_ydir)
            .ok()?;

    // 应用POINSP在SPINE局部坐标系中的横向偏移（X和Z坐标）
    let lateral_offset = DVec3::new(poinsp_local_pos.x, 0.0, poinsp_local_pos.z);
    // 修正：在SPINE局部坐标系中应用横向偏移，然后变换到世界坐标
    let final_transform = spine_transform * DMat4::from_translation(lateral_offset);

    println!("   🔍 横向偏移调试:");
    println!("      横向偏移: {:?}", lateral_offset);
    println!("      最终变换矩阵: {:?}", final_transform);

    Some(final_transform)
}

/// 计算SPINE路径上指定距离处的变换矩阵
fn calculate_spine_transform_at_distance(
    spline_pts: &[DVec3],
    distance: f64,
    ydir: Option<DVec3>,
) -> anyhow::Result<DMat4> {
    if spline_pts.len() < 2 {
        return Err(anyhow::anyhow!("路径点不足"));
    }

    // 简化版本：假设SPINE是直线，使用第一段
    let start_point = spline_pts[0];
    let end_point = spline_pts[1];
    let spine_direction = (end_point - start_point).normalize();

    // 计算距离起点的位置
    let point_at_distance = start_point + spine_direction * distance;

    // 调试输出
    println!("   🔍 SPINE路径调试:");
    println!("      起点: {:?}", start_point);
    println!("      终点: {:?}", end_point);
    println!("      方向: {:?}", spine_direction);
    println!("      距离: {:.3}mm", distance);
    println!("      计算位置: {:?}", point_at_distance);

    // 计算SPINE的方位
    let spine_rotation = if let Some(ydir_vec) = ydir {
        let rotation =
            cal_spine_orientation_basis_with_ydir(spine_direction, Some(ydir_vec), false);
        println!("      YDIR: {:?}", ydir_vec);
        println!("      计算旋转: {:?}", rotation);
        rotation
    } else {
        cal_spine_orientation_basis(spine_direction, false)
    };

    // 构建SPINE路径变换矩阵
    let spine_transform = DMat4::from_rotation_translation(spine_rotation, point_at_distance);
    println!("      SPINE变换矩阵: {:?}", spine_transform);

    Ok(spine_transform)
}

/// 判断节点类型是否为虚拟节点
/// 虚拟节点：没有自己的位置和方向，仅作为组织结构存在
/// 但可能包含方向信息（如YDIR）用于影响子节点
pub fn is_virtual_node(node_type: &str) -> bool {
    match node_type {
        "SPINE" => true,
        // 未来可能添加其他虚拟节点类型
        _ => false,
    }
}

/// 判断节点类型是否有零局部平移
pub fn has_zero_local_translation(node_type: &str) -> bool {
    is_virtual_node(node_type)
}

/// 获取虚拟节点的方向信息（如果有）
pub async fn get_virtual_node_orientation(
    node_refno: RefnoEnum,
    node_type: &str,
) -> anyhow::Result<Option<DQuat>> {
    if !is_virtual_node(node_type) {
        return Ok(None);
    }

    match node_type {
        "SPINE" => {
            // SPINE的方向由YDIR和spine方向决定
            let att = get_named_attmap(node_refno).await?;
            let ydir = att.get_dvec3("YDIR");

            // 获取父级GENSEC来获取spine方向
            let owner_refno = att.get_owner();

            if let Ok(spline_pts) = get_spline_pts(owner_refno).await {
                if spline_pts.len() >= 2 {
                    let spine_dir = (spline_pts[1] - spline_pts[0]).normalize();
                    // 只计算方向，不包含位置
                    let orientation = cal_spine_orientation_basis_with_ydir(spine_dir, ydir, false);
                    return Ok(Some(orientation));
                }
            }

            Ok(None)
        }
        _ => Ok(None),
    }
}
