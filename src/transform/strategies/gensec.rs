use super::TransformStrategy;
use crate::rs_surreal::spatial::{
    cal_ori_by_opdir, cal_ori_by_ydir, cal_ori_by_z_axis_ref_y,
    cal_spine_orientation_basis_with_ydir, get_spline_path,
};
use crate::{NamedAttrMap, RefnoEnum};
use async_trait::async_trait;
use glam::{DMat4, DQuat, DVec3};

/// GENSEC 专用的 BANG 处理器
pub struct GensecBangHandler;

/// GENSEC 专用的挤出方向处理器
pub struct GensecExtrusionHandler;

impl GensecBangHandler {
    /// GENSEC 特殊处理：BANG 总是被忽略
    pub fn should_apply_bang(_att: &NamedAttrMap, _cur_type: &str) -> (bool, f64) {
        (false, 0.0) // GENSEC 永远不应用 BANG
    }
}

impl GensecExtrusionHandler {
    /// 处理 GENSEC 的特殊挤出方向逻辑
    pub async fn extract_gensec_extrusion(
        parent_refno: RefnoEnum,
        parent_type: &str,
        att: &NamedAttrMap,
    ) -> anyhow::Result<(Option<DVec3>, Option<DVec3>)> {
        let parent_is_gensec = parent_type == "GENSEC";
        
        if parent_is_gensec {
            if let Ok(spine_paths) = get_spline_path(parent_refno).await {
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
        }
        
        // GENSEC 通常不处理 DPOSE/DPOSS，但保留兼容性
        if let Some(end) = att.get_dpose() && let Some(start) = att.get_dposs() {
            return Ok((Some((end - start).normalize()), None));
        }
        
        Ok((None, None))
    }
}

pub struct GensecStrategy;

#[async_trait]
impl TransformStrategy for GensecStrategy {
    async fn get_local_transform(
        &self,
        _refno: RefnoEnum,
        parent_refno: RefnoEnum,
        att: &NamedAttrMap,
        parent_att: &NamedAttrMap,
    ) -> anyhow::Result<Option<DMat4>> {
        let cur_type = att.get_type_str();
        let parent_type = parent_att.get_type_str();

        let mut rotation = DQuat::IDENTITY;
        let mut translation = DVec3::ZERO;
        let mut pos = att.get_position().unwrap_or_default().as_dvec3();
        let mut quat = DQuat::IDENTITY;
        let is_world_quat = false;

        // 1. 处理 NPOS 属性
        if att.contains_key("NPOS") {
            let npos = att.get_vec3("NPOS").unwrap_or_default();
            pos += npos.as_dvec3();
        }

        // 2. GENSEC 特殊处理：BANG 总是被忽略
        let (_apply_bang, _bangle) = GensecBangHandler::should_apply_bang(att, cur_type);

        // 3. 处理 GENSEC 特有的挤出方向
        let (pos_extru_dir, spine_ydir) = GensecExtrusionHandler::extract_gensec_extrusion(
            parent_refno, parent_type, att
        ).await?;

        // 4. 处理旋转初始化
        Self::initialize_gensec_rotation(
            att, cur_type, parent_type, pos_extru_dir, spine_ydir, 
            &mut quat, is_world_quat
        ).await?;

        // 5. 处理 YDIR/OPDI 属性
        let ydir_axis = att.get_dvec3("YDIR");
        let delta_vec = att.get_dvec3("DELP").unwrap_or_default();
        let mut has_opdir = false;

        if let Some(opdir) = att.get_dvec3("OPDI").map(|x| x.normalize()) {
            quat = cal_ori_by_opdir(opdir);
            has_opdir = true;
            pos += delta_vec;
        } else {
            // 处理 YDIR
            if let Some(v) = ydir_axis {
                let z_axis = if let Some(axis) = pos_extru_dir {
                    axis
                } else {
                    DVec3::X
                };
                quat = cal_ori_by_ydir(v.normalize(), z_axis);
            }
            // GENSEC: 不应用 BANG
        }

        translation = translation + rotation * pos;
        
        if !is_world_quat {
            rotation = rotation * quat;
        } else {
            rotation = quat;
        }

        let mat4 = DMat4::from_rotation_translation(rotation, translation);
        if rotation.is_nan() || translation.is_nan() {
            return Ok(None);
        }

        Ok(Some(mat4))
    }
}

impl GensecStrategy {
    /// 初始化 GENSEC 特有的旋转逻辑
    async fn initialize_gensec_rotation(
        att: &NamedAttrMap,
        cur_type: &str,
        parent_type: &str,
        pos_extru_dir: Option<DVec3>,
        spine_ydir: Option<DVec3>,
        quat: &mut DQuat,
        _is_world_quat: bool,
    ) -> anyhow::Result<()> {
        let parent_is_gensec = parent_type == "GENSEC";
        let quat_v = att.get_rotation();
        let has_local_ori = quat_v.is_some();
        
        if (!parent_is_gensec && has_local_ori) || (parent_is_gensec && cur_type == "TMPL") {
            *quat = quat_v.unwrap_or_default();
        } else {
            if let Some(z_axis) = pos_extru_dir {
                if parent_is_gensec {
                    if !z_axis.is_normalized() {
                        return Ok(());
                    }
                    *quat = cal_spine_orientation_basis_with_ydir(z_axis, spine_ydir, false);
                } else {
                    if !z_axis.is_normalized() {
                        return Ok(());
                    }
                    *quat = cal_ori_by_z_axis_ref_y(z_axis);
                }
            }
        }
        Ok(())
    }
}
