use super::TransformStrategy;
use crate::rs_surreal::spatial::{
    construct_basis_x_cutplane, construct_basis_z_opdir, construct_basis_z_y_exact,
    construct_basis_z_ref_y, construct_basis_z_y_hint, cal_zdis_pkdi_in_section_by_spine,
    get_spline_path, query_pline,
};
use crate::{NamedAttrMap, RefnoEnum, get_named_attmap};
use async_trait::async_trait;
use bevy_transform::prelude::Transform;
use glam::{DMat3, DMat4, DQuat, DVec3};
use super::NposHandler;

/// SJOI 专用的 CREF/CUTP 处理器
pub struct SjoiCrefHandler;

/// SJOI 专用的连接逻辑处理器
pub struct SjoiConnectionHandler;

impl SjoiCrefHandler {
    /// 处理 SJOI 的 CREF 连接逻辑
    pub async fn handle_sjoi_cref(
        att: &NamedAttrMap,
        parent_refno: RefnoEnum,
        translation: &mut DVec3,
        rotation: DQuat,
    ) -> anyhow::Result<(DVec3, f64)> {
        // 快速路径：如果没有 CREF，直接返回默认值
        let Some(c_ref) = att.get_foreign_refno("CREF") else {
            return Ok((DVec3::Z, 0.0));
        };

        let cut_dir = att.get_dvec3("CUTP").unwrap_or(DVec3::Z);
        let cut_len = att.get_f64("CUTB").unwrap_or_default();

        // 缓存属性获取，避免重复查询
        let Ok(c_att) = get_named_attmap(c_ref).await else {
            return Ok((DVec3::Z, 0.0));
        };

        let jline = c_att.get_str("JLIN").map(|x| x.trim()).unwrap_or("NA");

        if let Ok(Some(param)) = query_pline(c_ref, jline.into()).await {
            let jlin_pos = param.pt;

            // 并行获取世界坐标变换（优化性能）
            let (c_world_result, parent_world_result) = tokio::join!(
                crate::transform::get_world_mat4(c_ref, false),
                crate::transform::get_world_mat4(parent_refno, false)
            );

            let c_world = c_world_result?.unwrap_or(DMat4::IDENTITY);
            let parent_world = parent_world_result?.unwrap_or(DMat4::IDENTITY);
            let c_local_mat = parent_world.inverse() * c_world;
            let c_t = Transform::from_matrix(c_local_mat.as_mat4());

            // 预计算旋转后的向量（避免重复计算）
            let rotation_quat = c_t.rotation.as_dquat();
            let jlin_offset = rotation_quat * jlin_pos;
            let c_axis = rotation_quat * DVec3::Z;
            let c_wpos = c_t.translation.as_dvec3() + jlin_offset;

            // 沿梁轴方向计算
            let z_axis = rotation * DVec3::Z;

            // 优化：提前计算点积，避免重复计算
            let cutp_dot = c_axis.dot(cut_dir);
            let same_plane = cutp_dot.abs() > 0.001;

            if same_plane {
                let zaxis_dot = z_axis.dot(c_axis);
                let delta = (c_wpos - *translation).dot(z_axis);
                *translation = *translation + delta * z_axis;

                // 检查是否垂直（使用预计算的结果）
                let final_cut_len = if zaxis_dot.abs() < 0.001 {
                    cut_len
                } else {
                    0.0
                };

                return Ok((z_axis, final_cut_len));
            }
        }

        Ok((DVec3::Z, 0.0))
    }
}

pub struct SjoiStrategy;

#[async_trait]
impl TransformStrategy for SjoiStrategy {
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
        let mut is_world_quat = false;

        // 1. 处理 SJOI 特有的 CREF 连接逻辑
        let (connection_axis, cut_len) =
            SjoiCrefHandler::handle_sjoi_cref(att, parent_refno, &mut translation, rotation)
                .await?;

        // 2. 处理 NPOS 属性
        NposHandler::apply_npos_offset(&mut pos, att);

        // 3. 处理 BANG 属性
        let bangle = att.get_f32("BANG").unwrap_or_default() as f64;
        let apply_bang = att.contains_key("BANG") && bangle != 0.0;

        // 4. 处理 ZDIS 属性（通用逻辑）
        if att.contains_key("ZDIS") {
            let zdist = att.get_f32("ZDIS").unwrap_or_default();
            let pkdi = att.get_f32("PKDI").unwrap_or_default();

            if let Some((tmp_quat, tmp_pos)) =
                cal_zdis_pkdi_in_section_by_spine(parent_refno, pkdi, zdist, None).await?
            {
                quat = tmp_quat;
                pos = tmp_pos;
                is_world_quat = true;
            } else {
                translation += rotation * DVec3::Z * zdist as f64;
            }
        }

        // 5. 处理父级相关的变换
        let (pos_extru_dir, spine_ydir) =
            Self::extract_extrusion_direction(parent_refno, parent_type, att, parent_att).await?;

        // 6. 处理旋转初始化
        Self::initialize_rotation(
            att,
            cur_type,
            parent_type,
            pos_extru_dir,
            spine_ydir,
            &mut quat,
            is_world_quat,
        )
        .await?;

        // 7. 处理 YDIR/OPDI 属性
        let ydir_axis = att.get_dvec3("YDIR");
        let delta_vec = att.get_dvec3("DELP").unwrap_or_default();
        let mut has_opdir = false;

        if let Some(opdir) = att.get_dvec3("OPDI").map(|x| x.normalize()) {
            quat = construct_basis_z_opdir(opdir);
            has_opdir = true;
            if att
                .get_str("POSL")
                .map(|x| x.trim())
                .unwrap_or_default()
                .is_empty()
            {
                pos += delta_vec;
            }
        } else {
            // 处理 YDIR（假设 SJOI 不常用 POSL，但保留标准逻辑）
            if let Some(v) = ydir_axis {
                let z_axis = if let Some(axis) = pos_extru_dir {
                    axis
                } else {
                    DVec3::X
                };
                quat = construct_basis_z_y_exact(v.normalize(), z_axis);
            }

            // 应用 BANG
            if apply_bang {
                quat = quat * DQuat::from_rotation_z(bangle.to_radians());
            }
        }

        // 8. 处理 CUTP（通用逻辑）
        let has_local_ori = att.get_rotation().is_some();
        let cut_dir = att.get_dvec3("CUTP").unwrap_or(DVec3::Z);

        if att.contains_key("CUTP") && !has_opdir && !has_local_ori {
            let mat3 = DMat3::from_quat(rotation);
            quat = construct_basis_x_cutplane(mat3.z_axis, cut_dir);
            is_world_quat = true;
        }

        // 9. 应用连接偏移（如果有）
        if cut_len > 0.0 {
            translation += connection_axis * cut_len;
        }

        translation = translation + rotation * pos;

        if is_world_quat {
            rotation = quat;
        } else {
            rotation = rotation * quat;
        }

        let mat4 = DMat4::from_rotation_translation(rotation, translation);
        if rotation.is_nan() || translation.is_nan() {
            return Ok(None);
        }

        Ok(Some(mat4))
    }
}

impl SjoiStrategy {

    /// 初始化旋转
    async fn initialize_rotation(
        att: &NamedAttrMap,
        cur_type: &str,
        parent_type: &str,
        pos_extru_dir: Option<DVec3>,
        spine_ydir: Option<DVec3>,
        quat: &mut DQuat,
        is_world_quat: bool,
    ) -> anyhow::Result<()> {
        let parent_is_gensec = parent_type == "GENSEC";
        let quat_v = att.get_rotation();
        let has_local_ori = quat_v.is_some();

        if (!parent_is_gensec && has_local_ori) || (parent_is_gensec && cur_type == "TMPL") {
            *quat = quat_v.unwrap_or_default();
        } else {
            if let Some(z_axis) = pos_extru_dir {
                if parent_is_gensec {
                    if !is_world_quat {
                        if !z_axis.is_normalized() {
                            return Ok(());
                        }
                        *quat = construct_basis_z_y_hint(z_axis, spine_ydir, false);
                    }
                } else {
                    if !z_axis.is_normalized() {
                        return Ok(());
                    }
                    *quat = construct_basis_z_ref_y(z_axis);
                }
            }
        }
        Ok(())
    }
}
