use super::{TransformStrategy, BangHandler};
use crate::rs_surreal::spatial::{
    SectionEnd, construct_basis_x_cutplane, construct_basis_z_opdir, construct_basis_z_y_exact,
    construct_basis_z_ref_x, construct_basis_z_ref_y, construct_basis_z_default,
    construct_basis_z_y_hint, cal_zdis_pkdi_in_section_by_spine, is_virtual_node,
    query_pline,
};
use crate::{
    NamedAttrMap, RefnoEnum, SUL_DB, get_named_attmap, pdms_data::PlinParam,
    tool::direction_parse::parse_expr_to_dir,
};
use async_trait::async_trait;
use glam::{DMat3, DMat4, DQuat, DVec3};
use super::NposHandler;

/// ZDIS 属性处理器
pub struct ZdisHandler;

/// POSL/PLIN 属性处理器  
pub struct PoslHandler;

/// YDIR/OPDI 属性处理器
pub struct YdirHandler;

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
        parent_att: &NamedAttrMap,
        cur_type: &str,
        pos: &mut DVec3,
        quat: &mut DQuat,
        effective_att: &NamedAttrMap,
        should_apply_bang: bool,
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
                crate::query_filter_ancestors(att.get_owner(), &crate::consts::HAS_PLIN_TYPES)
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
                }

                if let Ok(Some(own_param)) =
                    crate::query_pline(plin_owner, own_pos_line.into()).await
                {
                    plin_pos -= own_param.pt;
                }
            }

            let z_axis = if is_lmirror { -pline_plax } else { pline_plax };
            let plin_pos = if is_lmirror { -plin_pos } else { plin_pos };

            // YDIR 优先取自身的，如果没有则取 Owner 的 (如 FITT 继承 STWALL 的 YDIR)
            let eff_ydir = if let Some(v) = ydir_axis {
                Some(v)
            } else {
                parent_att.get_dvec3("YDIR")
            };

            let mut new_quat = if cur_type == "SCOJ" {
                construct_basis_z_ref_x(z_axis) * *quat
            } else {
                if let Some(ydir) = eff_ydir {
                    construct_basis_z_y_exact(ydir.normalize(), z_axis)
                } else {
                    construct_basis_z_ref_y(z_axis) * *quat
                }
            };

            // 应用 BANG（如果需要且不是 GENSEC）
            if should_apply_bang {
                BangHandler::apply_bang(&mut new_quat, effective_att);
            }

            // 位置计算：
            // 1. plin_pos: POSL 在路径上的点 (父级/世界空间)
            // 2. rotation: 父级当前的旋转 (通常为 Identity, 除非由外部传入)
            // 3. new_quat: 当前元素相对于路径的旋转
            // 4. delta_vec (DELP): 局部偏移，需应用当前旋转
            // 5. pos: 这里的 pos 主要是 ZDIS/NPOS 等预先累加的偏移，通常也是局部 Z 轴或位移
            //    注意：如果 pos 是 ZDIS 产生的 (0,0,z)，它应该是在 local frame 下的。
            //    所以应该变换后加。

            // 修正公式: Translation = Rotation_Parent * (Plin_Pos) + Rotation_Parent * Rotation_Self * (DELP + POS)
            // 假设 rotation (parent) 为 Identity 或已包含在 plin_pos 转换中 (query_pline通常返回Owner系坐标)

            // 如果 pos 包含 ZDIS (局部 Z 轴偏移):
            let local_offset = delta_vec + *pos;
            let world_offset = rotation * new_quat * local_offset;

            // plin_pos 是路径上的点，需应用父级旋转 (如果 rotation 是父级旋转)
            // 但 DefaultStrategy 中 rotation 初始为 Identity，且 translation 初始为 0
            // 这里的 rotation 参数实际上是 accumulated rotation?
            // 在 DefaultStrategy 调用时，rotation 是 Identity.

            let offset = rotation * plin_pos + world_offset;

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
            *quat = construct_basis_z_opdir(opdir);
            *has_opdir = true;
            *pos += delta_vec;
        } else if let Some(v) = ydir_axis {
            let z_axis = if let Some(axis) = pos_extru_dir {
                axis
            } else {
                DVec3::X
            };
            *quat = construct_basis_z_y_exact(v.normalize(), z_axis);
        }
        Ok(())
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
            *quat = construct_basis_x_cutplane(mat3.z_axis, cut_dir);
            *is_world_quat = true;
        }
        Ok(())
    }
}

pub struct DefaultStrategy {
    att: NamedAttrMap,
}

impl DefaultStrategy {
    pub fn new(att: NamedAttrMap) -> Self {
        Self { att }
    }
}

#[async_trait]
impl TransformStrategy for DefaultStrategy {
    async fn get_local_transform(
        &mut self,
    ) -> anyhow::Result<Option<DMat4>> {
        // 直接使用通用查询函数获取所有需要的数据
        let att = &self.att;
        let cur_type = att.get_type_str();
        
        // 虚拟节点（如 SPINE）没有变换，直接跳过
        if is_virtual_node(cur_type) {
            return Ok(Some(DMat4::IDENTITY));
        }
        
        // 默认策略：只处理基本的 POS + ORI 变换
        let mut position = att.get_position().unwrap_or_default().as_dvec3();
        let mut rotation = att.get_rotation().unwrap_or(DQuat::IDENTITY);
        let mat4 = DMat4::from_rotation_translation(rotation, position);
        
        Ok(Some(mat4))
    }
}

// 复杂策略：处理需要复杂属性逻辑的元素（如STWALL、FITT、POINSP等）
// pub struct ComplexStrategy;

// #[async_trait]
// impl TransformStrategy for ComplexStrategy {
//     async fn get_local_transform(
//         &self,
//         refno: RefnoEnum,
//         parent_refno: RefnoEnum,
//         att: &NamedAttrMap,
//         parent_att: &NamedAttrMap,
//     ) -> anyhow::Result<Option<DMat4>> {
//         let cur_type = att.get_type_str();
        
//         // 虚拟节点（如 SPINE）没有变换，直接跳过
//         if is_virtual_node(cur_type) {
//             return Ok(Some(DMat4::IDENTITY));
//         }

//         let parent_type = parent_att.get_type_str();

//         let mut rotation = DQuat::IDENTITY;
//         let mut translation = DVec3::ZERO;
//         let mut pos = att.get_position().unwrap_or_default().as_dvec3();
//         let mut quat = DQuat::IDENTITY;
//         let mut is_world_quat = false;
//         let mut has_opdir = false;

//         // 1. 处理 NPOS 属性
//         NposHandler::apply_npos_offset(&mut pos, att);

//         // 2. 处理 BANG 属性
//         // 检查是否应该应用 BANG（GENSEC 类型除外）
//         let should_apply_bang = cur_type != "GENSEC" && (att.contains_key("BANG") || parent_att.contains_key("BANG"));
        
//         // 如果父节点有 BANG，优先使用父节点属性
//         let effective_att = if parent_att.contains_key("BANG") && parent_att.get_f32("BANG").unwrap_or(0.0) != 0.0 {
//             parent_att
//         } else {
//             att
//         };

//         // 3. 处理 ZDIS 属性（通用类型，非 ENDATU）
//         if cur_type == "POINSP" {
//             ZdisHandler::handle_poinsp_zdis(att, parent_refno, &mut pos).await?;
//         } else {
//             ZdisHandler::handle_generic_zdis(
//                 att,
//                 parent_refno,
//                 cur_type,
//                 &mut pos,
//                 &mut quat,
//                 &mut is_world_quat,
//                 &mut translation,
//                 rotation,
//             )
//             .await?;
//         }

//         // 4. 处理父级相关的变换（Spine/Extrusion）
//         let (pos_extru_dir, spine_ydir) =
//             Self::extract_extrusion_direction(parent_refno, parent_type, att, parent_att).await?;

//         // 5. 处理旋转初始化
//         Self::initialize_rotation(
//             att,
//             cur_type,
//             parent_type,
//             pos_extru_dir,
//             spine_ydir,
//             &mut quat,
//             is_world_quat,
//         )
//         .await?;

//         // 6. 处理 YDIR/OPDI 属性
//         let ydir_axis = att.get_dvec3("YDIR");
//         let delta_vec = att.get_dvec3("DELP").unwrap_or_default();

//         if att
//             .get_str("POSL")
//             .map(|x| x.trim())
//             .unwrap_or_default()
//             .is_empty()
//         {
//             // 没有 POSL 时的处理
//             YdirHandler::handle_ydir_opdi(
//                 att,
//                 pos_extru_dir,
//                 &mut quat,
//                 &mut pos,
//                 delta_vec,
//                 &mut has_opdir,
//             )?;

//             // 应用 BANG（如果需要且不是 GENSEC）
//             if should_apply_bang {
//                 BangHandler::apply_bang(&mut quat, effective_att);
//             }

//             // 处理 CUTP 属性
//             let has_local_ori = att.get_rotation().is_some();
//             CutpHandler::handle_cutp(
//                 att,
//                 &mut quat,
//                 rotation,
//                 has_opdir,
//                 has_local_ori,
//                 &mut is_world_quat,
//             )?;

//             translation = translation + rotation * pos;

//             if is_world_quat {
//                 rotation = quat;
//             } else {
//                 rotation = rotation * quat;
//             }
//         } else {
//             // 有 POSL 时的处理
//             PoslHandler::handle_posl(
//                 att,
//                 parent_att,
//                 cur_type,
//                 &mut pos,
//                 &mut quat,
//                 effective_att,
//                 should_apply_bang,
//                 ydir_axis,
//                 delta_vec,
//                 &mut translation,
//                 rotation,
//             )
//             .await?;

//             rotation = rotation * quat;
//         }

//         let mat4 = DMat4::from_rotation_translation(rotation, translation);
//         if rotation.is_nan() || translation.is_nan() {
//             return Ok(None);
//         }

//         Ok(Some(mat4))
//     }
// }

// impl ComplexStrategy {
//     /// 提取挤出方向信息
//     async fn extract_extrusion_direction(
//         parent_refno: RefnoEnum,
//         parent_type: &str,
//         att: &NamedAttrMap,
//         parent_att: &NamedAttrMap,
//     ) -> anyhow::Result<(Option<DVec3>, Option<DVec3>)> {
//         let parent_is_gensec = parent_type == "GENSEC";

//         if parent_is_gensec {
//             if let Ok(spine_paths) = get_spline_path(parent_refno).await {
//                 if let Some(first_spine) = spine_paths.first() {
//                     let dir = (first_spine.pt1 - first_spine.pt0).normalize();
//                     let pos_extru_dir = Some(dir.as_dvec3());
//                     let mut ydir = first_spine.preferred_dir.as_dvec3();

//                     // 考虑 Parent BANG 对截面方向的影响
//                     let parent_bangle = parent_att.get_f32("BANG").unwrap_or(0.0) as f64;
//                     if parent_bangle.abs() > 0.001 {
//                         let rot =
//                             DQuat::from_axis_angle(dir.as_dvec3(), parent_bangle.to_radians());
//                         ydir = rot * ydir;
//                     }

//                     let spine_ydir = if ydir.length_squared() > 0.01 {
//                         Some(ydir)
//                     } else {
//                         None
//                     };
//                     return Ok((pos_extru_dir, spine_ydir));
//                 }
//             }
//         }

//         // 处理 DPOSE/DPOSS 属性
//         if let Some(end) = att.get_dpose()
//             && let Some(start) = att.get_dposs()
//         {
//             return Ok((Some((end - start).normalize()), None));
//         }

//         Ok((None, None))
//     }

//     /// 初始化旋转
//     async fn initialize_rotation(
//         att: &NamedAttrMap,
//         cur_type: &str,
//         parent_type: &str,
//         pos_extru_dir: Option<DVec3>,
//         spine_ydir: Option<DVec3>,
//         quat: &mut DQuat,
//         is_world_quat: bool,
//     ) -> anyhow::Result<()> {
//         let parent_is_gensec = parent_type == "GENSEC";
//         let quat_v = att.get_rotation();
//         let has_local_ori = quat_v.is_some();

//         if (!parent_is_gensec && has_local_ori) || (parent_is_gensec && cur_type == "TMPL") {
//             *quat = quat_v.unwrap_or_default();
//         } else {
//             if let Some(z_axis) = pos_extru_dir {
//                 if parent_is_gensec {
//                     if !is_world_quat {
//                         if !z_axis.is_normalized() {
//                             return Ok(());
//                         }
//                         *quat = construct_basis_z_y_hint(z_axis, spine_ydir, false);
//                     }
//                 } else {
//                     if !z_axis.is_normalized() {
//                         return Ok(());
//                     }
//                     *quat = construct_basis_z_ref_y(z_axis);
//                 }
//             }
//         }
//         Ok(())
//     }
// }
