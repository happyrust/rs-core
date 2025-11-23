use super::TransformStrategy;
use crate::rs_surreal::spatial::{
    SectionEnd, cal_cutp_ori, cal_ori_by_opdir, cal_ori_by_ydir, cal_ori_by_z_axis_ref_y,
    cal_spine_orientation_basis_with_ydir, cal_zdis_pkdi_in_section_by_spine, get_spline_path,
};
use crate::{NamedAttrMap, RefnoEnum};
use async_trait::async_trait;
use glam::{DMat3, DMat4, DQuat, DVec3};

/// ENDATU 专用的 ZDIS 处理器
pub struct EndAtuZdisHandler;

impl EndAtuZdisHandler {
    /// 处理 ENDATU 的特殊 ZDIS 逻辑
    /// 根据 ENDATU 在父级中的索引确定是 START 还是 END
    pub async fn handle_endatu_zdis(
        refno: RefnoEnum,
        parent_refno: RefnoEnum,
        att: &NamedAttrMap,
        pos: &mut DVec3,
        quat: &mut DQuat,
    ) -> anyhow::Result<bool> {
        if att.contains_key("ZDIS") {
            // 确定是第几个 ENDATU
            let endatu_index: Option<u32> =
                crate::get_index_by_noun_in_parent(parent_refno, refno, Some("ENDATU"))
                    .await
                    .unwrap();

            let section_end = if endatu_index == Some(0) {
                Some(SectionEnd::START)
            } else if endatu_index == Some(1) {
                Some(SectionEnd::END)
            } else {
                None
            };

            if let Some(result) = cal_zdis_pkdi_in_section_by_spine(
                parent_refno,
                0.0,
                att.get_f32("ZDIS").unwrap_or_default(),
                section_end,
            )
            .await?
            {
                *pos += result.1;
                *quat = result.0;
                return Ok(true); // 表示已处理，应该直接返回
            }
        }
        Ok(false) // 表示未处理，需要继续标准流程
    }
}

pub struct EndAtuStrategy;

#[async_trait]
impl TransformStrategy for EndAtuStrategy {
    async fn get_local_transform(
        &self,
        refno: RefnoEnum,
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

        // 1. 处理 ENDATU 特有的 ZDIS 逻辑
        if EndAtuZdisHandler::handle_endatu_zdis(
            refno, parent_refno, att, &mut pos, &mut quat
        ).await? {
            // ZDIS 处理成功，直接返回结果
            translation = translation + rotation * pos;
            rotation = quat;
            return Ok(Some(DMat4::from_rotation_translation(
                rotation,
                translation,
            )));
        }

        // 2. 如果 ZDIS 没有触发早期返回，继续标准逻辑流程
        // 处理 NPOS 属性
        if att.contains_key("NPOS") {
            let npos = att.get_vec3("NPOS").unwrap_or_default();
            pos += npos.as_dvec3();
        }

        // 3. 处理 BANG 属性
        let bangle = att.get_f32("BANG").unwrap_or_default() as f64;
        let apply_bang = att.contains_key("BANG") && bangle != 0.0;

        // 4. 处理父级相关的变换（Spine/Extrusion）
        let (pos_extru_dir, spine_ydir) = Self::extract_extrusion_direction(parent_refno, parent_type, att).await?;

        // 5. 处理旋转初始化
        Self::initialize_rotation(
            att, cur_type, parent_type, pos_extru_dir, spine_ydir, 
            &mut quat, false
        ).await?;

        // 6. 处理 YDIR/OPDI 属性
        let ydir_axis = att.get_dvec3("YDIR");
        let delta_vec = att.get_dvec3("DELP").unwrap_or_default();
        let mut has_opdir = false;

        if let Some(opdir) = att.get_dvec3("OPDI").map(|x| x.normalize()) {
            quat = cal_ori_by_opdir(opdir);
            has_opdir = true;
            pos += delta_vec; // ENDATU 通常没有 POSL
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

            // 应用 BANG
            if apply_bang {
                quat = quat * DQuat::from_rotation_z(bangle.to_radians());
            }

            // 处理 CUTP
            let has_local_ori = att.get_rotation().is_some();
            let has_cut_dir = att.contains_key("CUTP");
            let cut_dir = att.get_dvec3("CUTP").unwrap_or(DVec3::Z);
            
            if has_cut_dir && !has_opdir && !has_local_ori {
                let mat3 = DMat3::from_quat(rotation);
                quat = cal_cutp_ori(mat3.z_axis, cut_dir);
            }
        }

        translation = translation + rotation * pos;
        rotation = rotation * quat;

        let mat4 = DMat4::from_rotation_translation(rotation, translation);
        if rotation.is_nan() || translation.is_nan() {
            return Ok(None);
        }

        Ok(Some(mat4))
    }
}

impl EndAtuStrategy {
    /// 提取挤出方向信息
    async fn extract_extrusion_direction(
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
        
        // 处理 DPOSE/DPOSS 属性
        if let Some(end) = att.get_dpose() && let Some(start) = att.get_dposs() {
            return Ok((Some((end - start).normalize()), None));
        }
        
        Ok((None, None))
    }
    
    /// 初始化旋转
    async fn initialize_rotation(
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
