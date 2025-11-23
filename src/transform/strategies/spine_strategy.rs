/// SPINE 策略实现模块

use super::{TransformStrategy, NposHandler, BangHandler};
use crate::rs_surreal::spatial::{
    construct_basis_z_opdir, construct_basis_z_y_exact, construct_basis_z_ref_y,
    construct_basis_z_y_hint,
};
use crate::prim_geo::spine::{Spine3D, SpineCurveType};
use crate::{NamedAttrMap, RefnoEnum, get_type_name, get_children_refnos, get_named_attmap, get_children_named_attmaps};
use async_trait::async_trait;
use glam::{DMat4, DQuat, DVec3, Vec3};


pub struct SpineStrategy;


impl SpineStrategy {
    /// 处理 GENSEC 的特殊挤出方向逻辑
    pub async fn extract_spine_extrusion(
        parent_refno: RefnoEnum,
        att: &NamedAttrMap,
        parent_att: &NamedAttrMap,
    ) -> anyhow::Result<(Option<DVec3>, Option<DVec3>)> {

        if let Ok(spine_paths) = get_spline_path(parent_att).await {
            if let Some(first_spine) = spine_paths.first() {
                let dir = (first_spine.pt1 - first_spine.pt0).normalize();
                let pos_extru_dir = Some(dir.as_dvec3());
                let ydir = first_spine.preferred_dir;
                let spine_ydir = if ydir.length_squared() > 0.01 {
                    Some(ydir.as_dvec3())
                } else {
                    None
                };
                return Ok((pos_extru_dir, spine_ydir));
            }
        }

        if let Some(end) = att.get_dpose()
            && let Some(start) = att.get_dposs()
        {
            return Ok((Some((end - start).normalize()), None));
        }

        Ok((None, None))
    }
}



#[async_trait]
impl TransformStrategy for SpineStrategy {
    async fn get_local_transform(
        &self,
        _refno: RefnoEnum,
        parent_refno: RefnoEnum,
        att: &NamedAttrMap,
        parent_att: &NamedAttrMap,
    ) -> anyhow::Result<Option<DMat4>> {
        let cur_type = att.get_type_str();
        let parent_type = parent_att.get_type_str();
        let mut pos = att.get_position().unwrap_or_default().as_dvec3();
        let mut quat = DQuat::IDENTITY;

        // 1. 处理 NPOS 属性
        NposHandler::apply_npos_offset(&mut pos, att);

        // 2. 处理 GENSEC 特有的挤出方向
        let (pos_extru_dir, spine_ydir) =
            Self::extract_spine_extrusion(parent_refno, att, parent_att).await?;

        dbg!(&pos_extru_dir);
        dbg!(&spine_ydir);

        // 3. 处理旋转初始化
        if let Some(extru_dir) = pos_extru_dir {
            quat = Self::initialize_rotation(
                extru_dir,
                spine_ydir,
            );
        } else {
            return Ok(None);
        }

        // 4. 添加 BANG 的处理 handler
        BangHandler::apply_bang(&mut quat, parent_att);
       

        // 5. 处理 YDIR/OPDI 属性
        // let ydir_axis = att.get_dvec3("YDIR");
        // let delta_vec = att.get_dvec3("DELP").unwrap_or_default();
        // let mut has_opdir = false;

        // if let Some(opdir) = att.get_dvec3("OPDI").map(|x| x.normalize()) {
        //     quat = construct_basis_z_opdir(opdir);
        //     has_opdir = true;
        //     pos += delta_vec;
        // } 
        if quat.is_nan() || pos.is_nan() {
            return Ok(None);
        }
        let mat4 = DMat4::from_rotation_translation(quat, pos);
        Ok(Some(mat4))
    }
}

impl SpineStrategy {
    /// 初始化 SPINE 的旋转逻辑：基于YDIR和两点相减方向计算方位
    fn initialize_rotation(
        pos_extru_dir: DVec3,
        spine_ydir: Option<DVec3>,
    ) ->  DQuat {
        // 优先使用YDIR属性
        if let Some(ydir) = spine_ydir {
            // 基于YDIR和挤出方向计算方位
            return construct_basis_z_y_hint(pos_extru_dir, Some(ydir), false);
        } else {
            // 没有YDIR时，仅基于两点相减的方向计算
            return construct_basis_z_ref_y(pos_extru_dir);
        }
    }
}

/// 从 GENSEC/WALL 元素提取 SPINE 路径
/// 
/// 此函数从给定的 GENSEC 或 WALL 元素中提取所有子 SPINE 元素的路径信息。
/// 每个 SPINE 由 POINSP 和 CURVE 点组成，支持直线和曲线两种类型。
pub async fn get_spline_path(spine_att: &NamedAttrMap) -> anyhow::Result<Vec<Spine3D>> {
    let mut paths = vec![];

    let ch_atts = get_children_named_attmaps( spine_att.get_refno().unwrap())
        .await
        .unwrap_or_default();
    let len = ch_atts.len();
    if len < 1 {
        return Ok(paths);
    }
    let ydir = spine_att.get_vec3("YDIR").unwrap_or(Vec3::Z);

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
                preferred_dir: ydir,
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
                preferred_dir: ydir,
                radius: att2.get_f32("RAD").unwrap_or_default(),
            });
            i += 2;
        }
    }

    dbg!(&paths);

    Ok(paths)
}
