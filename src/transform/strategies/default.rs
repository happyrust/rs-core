use super::TransformStrategy;
use crate::rs_surreal::spatial::{
    SectionEnd, cal_cutp_ori, cal_ori_by_opdir, cal_ori_by_ydir, cal_ori_by_z_axis_ref_x,
    cal_ori_by_z_axis_ref_y, cal_spine_orientation_basis, cal_spine_orientation_basis_with_ydir,
    cal_zdis_pkdi_in_section_by_spine, get_spline_path, query_pline,
};
use crate::{
    NamedAttrMap, RefnoEnum, SUL_DB, get_named_attmap,
    pdms_data::PlinParam,
    tool::direction_parse::parse_expr_to_dir,
};
use async_trait::async_trait;
use glam::{DMat3, DMat4, DQuat, DVec3};

/// ZDIS 属性处理器
pub struct ZdisHandler;

/// POSL/PLIN 属性处理器  
pub struct PoslHandler;

/// YDIR/OPDI 属性处理器
pub struct YdirHandler;

/// BANG 属性处理器
pub struct BangHandler;

/// CUTP 属性处理器
pub struct CutpHandler;

impl ZdisHandler {
    /// 处理通用类型的 ZDIS 属性（非 ENDATU）
    pub async fn handle_generic_zdis(
        att: &NamedAttrMap,
        parent_refno: RefnoEnum,
        cur_type: &str,
        pos: &mut DVec3,
        quat: &mut DQuat,
        is_world_quat: &mut bool,
        translation: &mut DVec3,
        rotation: DQuat,
    ) -> anyhow::Result<()> {
        if att.contains_key("ZDIS") && cur_type != "ENDATU" {
            let zdist = att.get_f32("ZDIS").unwrap_or_default();
            let pkdi = att.get_f32("PKDI").unwrap_or_default();

            if let Some((tmp_quat, tmp_pos)) =
                cal_zdis_pkdi_in_section_by_spine(parent_refno, pkdi, zdist, None).await?
            {
                *quat = tmp_quat;
                *pos = tmp_pos;
                *is_world_quat = true;
            } else {
                *translation += rotation * DVec3::Z * zdist as f64;
            }
        }
        Ok(())
    }

    /// 处理 POINSP 类型的特殊 ZDIS 逻辑
    pub async fn handle_poinsp_zdis(
        att: &NamedAttrMap,
        parent_refno: RefnoEnum,
        pos: &mut DVec3,
    ) -> anyhow::Result<()> {
        if att.contains_key("ZDIS") {
            let zdist = att.get_f32("ZDIS").unwrap_or_default();
            let pkdi = att.get_f32("PKDI").unwrap_or_default();

            if let Some((_, tmp_pos)) =
                cal_zdis_pkdi_in_section_by_spine(parent_refno, pkdi, zdist, None).await?
            {
                *pos = *pos + tmp_pos; // 保留原始局部位置，加上偏移
            }
        }
        Ok(())
    }
}

impl PoslHandler {
    /// 处理 POSL/PLIN 属性逻辑
    pub async fn handle_posl(
        att: &NamedAttrMap,
        cur_type: &str,
        pos: &mut DVec3,
        quat: &mut DQuat,
        bangle: f64,
        apply_bang: bool,
        ydir_axis: Option<DVec3>,
        delta_vec: DVec3,
        translation: &mut DVec3,
        rotation: DQuat,
    ) -> anyhow::Result<()> {
        let pos_line = att.get_str("POSL").map(|x| x.trim()).unwrap_or_default();
        
        if !pos_line.is_empty() {
            let mut plin_pos = DVec3::ZERO;
            let mut pline_plax = DVec3::X;
            let mut is_lmirror = false;

            let ancestor_refnos =
                crate::query_filter_ancestors(att.get_owner(), &crate::consts::HAS_PLIN_TYPES).await?;
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
                }

                if let Ok(Some(own_param)) = crate::query_pline(plin_owner, own_pos_line.into()).await {
                    plin_pos -= own_param.pt;
                }
            }

            let z_axis = if is_lmirror { -pline_plax } else { pline_plax };
            let plin_pos = if is_lmirror { -plin_pos } else { plin_pos };

            let mut new_quat = {
                if cur_type == "FITT" {
                    // FITT需要特殊的坐标系构建：相对于父级旋转90度
                    // 验证集期望：Y is E, Z is S
                    // 父级STWALL：Y is U, Z is E
                    // 使用z_axis作为父级实际的Z轴向量(E方向)
                    let parent_z_axis = z_axis;                    // 父级Z轴 = E方向
                    let parent_y_axis = DVec3::Z.cross(parent_z_axis).normalize(); // 父级Y轴 = U方向
                    
                    // FITT的局部坐标系：Y轴=E方向，Z轴=S方向
                    let fitt_y_axis = parent_z_axis;           // Y轴 = 父级Z轴 (E)
                    let fitt_z_axis = -parent_y_axis;          // Z轴 = -父级Y轴 (S)
                    let fitt_x_axis = fitt_y_axis.cross(fitt_z_axis).normalize();
                    
                    DQuat::from_mat3(&DMat3::from_cols(fitt_x_axis, fitt_y_axis, fitt_z_axis))
                } else if cur_type == "SCOJ" {
                    cal_ori_by_z_axis_ref_x(z_axis) * *quat
                } else {
                    cal_ori_by_z_axis_ref_y(z_axis) * *quat
                }
            };

            if let Some(v) = ydir_axis {
                new_quat = cal_ori_by_ydir(v.normalize(), z_axis);
            }

            if apply_bang {
                new_quat = new_quat * DQuat::from_rotation_z(bangle.to_radians());
            }

            let offset = if cur_type == "FITT" {
                // FITT需要特殊的位置计算：将位置从父级坐标系转换到FITT的局部坐标系
                new_quat.inverse() * (*pos + plin_pos) + rotation * new_quat * delta_vec
            } else {
                rotation * (*pos + plin_pos) + rotation * new_quat * delta_vec
            };
            *translation += offset;
            *quat = new_quat;
        }
        Ok(())
    }
}

impl YdirHandler {
    /// 处理 YDIR 和 OPDI 属性
    pub fn handle_ydir_opdi(
        att: &NamedAttrMap,
        pos_extru_dir: Option<DVec3>,
        quat: &mut DQuat,
        pos: &mut DVec3,
        delta_vec: DVec3,
        has_opdir: &mut bool,
    ) -> anyhow::Result<()> {
        let ydir_axis = att.get_dvec3("YDIR");
        
        if let Some(opdir) = att.get_dvec3("OPDI").map(|x| x.normalize()) {
            *quat = cal_ori_by_opdir(opdir);
            *has_opdir = true;
            *pos += delta_vec;
        } else if let Some(v) = ydir_axis {
            let z_axis = if let Some(axis) = pos_extru_dir {
                axis
            } else {
                DVec3::X
            };
            *quat = cal_ori_by_ydir(v.normalize(), z_axis);
        }
        Ok(())
    }
}

impl BangHandler {
    /// 处理 BANG 属性
    pub fn apply_bang(quat: &mut DQuat, bangle: f64, apply_bang: bool) {
        if apply_bang {
            *quat = *quat * DQuat::from_rotation_z(bangle.to_radians());
        }
    }

    /// 判断是否应该应用 BANG
    pub fn should_apply_bang(att: &NamedAttrMap, cur_type: &str) -> (bool, f64) {
        let bangle = att.get_f32("BANG").unwrap_or_default() as f64;
        let apply_bang = att.contains_key("BANG") && bangle != 0.0;
        
        // GENSEC 特殊处理：不应用 BANG
        if cur_type == "GENSEC" {
            (false, bangle)
        } else {
            (apply_bang, bangle)
        }
    }
}

impl CutpHandler {
    /// 处理 CUTP 属性
    pub fn handle_cutp(
        att: &NamedAttrMap,
        quat: &mut DQuat,
        rotation: DQuat,
        has_opdir: bool,
        has_local_ori: bool,
        is_world_quat: &mut bool,
    ) -> anyhow::Result<()> {
        let has_cut_dir = att.contains_key("CUTP");
        let cut_dir = att.get_dvec3("CUTP").unwrap_or(DVec3::Z);
        
        if has_cut_dir && !has_opdir && !has_local_ori {
            let mat3 = DMat3::from_quat(rotation);
            *quat = cal_cutp_ori(mat3.z_axis, cut_dir);
            *is_world_quat = true;
        }
        Ok(())
    }
}

pub struct DefaultStrategy;

#[async_trait]
impl TransformStrategy for DefaultStrategy {
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
        let mut is_world_quat = false;
        let mut has_opdir = false;

        // 1. 处理 NPOS 属性
        if att.contains_key("NPOS") {
            let npos = att.get_vec3("NPOS").unwrap_or_default();
            pos += npos.as_dvec3();
        }

        // 2. 处理 BANG 属性
        let (apply_bang, bangle) = BangHandler::should_apply_bang(att, cur_type);

        // 3. 处理 ZDIS 属性（通用类型，非 ENDATU）
        if cur_type == "POINSP" {
            ZdisHandler::handle_poinsp_zdis(att, parent_refno, &mut pos).await?;
        } else {
            ZdisHandler::handle_generic_zdis(
                att, parent_refno, cur_type, &mut pos, &mut quat, 
                &mut is_world_quat, &mut translation, rotation
            ).await?;
        }

        // 4. 处理父级相关的变换（Spine/Extrusion）
        let (pos_extru_dir, spine_ydir) = Self::extract_extrusion_direction(parent_refno, parent_type, att).await?;

        // 5. 处理旋转初始化
        Self::initialize_rotation(
            att, cur_type, parent_type, pos_extru_dir, spine_ydir, 
            &mut quat, is_world_quat
        ).await?;

        // 6. 处理 YDIR/OPDI 属性
        let ydir_axis = att.get_dvec3("YDIR");
        let delta_vec = att.get_dvec3("DELP").unwrap_or_default();
        
        if att.get_str("POSL").map(|x| x.trim()).unwrap_or_default().is_empty() {
            // 没有 POSL 时的处理
            YdirHandler::handle_ydir_opdi(
                att, pos_extru_dir, &mut quat, &mut pos, delta_vec, &mut has_opdir
            )?;
            
            BangHandler::apply_bang(&mut quat, bangle, apply_bang);
            
            // 处理 CUTP 属性
            let has_local_ori = att.get_rotation().is_some();
            CutpHandler::handle_cutp(
                att, &mut quat, rotation, has_opdir, has_local_ori, &mut is_world_quat
            )?;
            
            translation = translation + rotation * pos;
            
            if is_world_quat {
                rotation = quat;
            } else {
                rotation = rotation * quat;
            }
        } else {
            // 有 POSL 时的处理
            PoslHandler::handle_posl(
                att, cur_type, &mut pos, &mut quat, bangle, apply_bang,
                ydir_axis, delta_vec, &mut translation, rotation
            ).await?;
            
            rotation = rotation * quat;
        }

        let mat4 = DMat4::from_rotation_translation(rotation, translation);
        if rotation.is_nan() || translation.is_nan() {
            return Ok(None);
        }

        Ok(Some(mat4))
    }
}

impl DefaultStrategy {
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
                        *quat = cal_spine_orientation_basis_with_ydir(z_axis, spine_ydir, false);
                    }
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
