use super::NposHandler;
use super::{BangHandler, TransformStrategy};
use crate::rs_surreal::spatial::{
    SectionEnd, cal_zdis_pkdi_in_section_by_spine, construct_basis_x_cutplane,
    construct_basis_z_default, construct_basis_z_opdir, construct_basis_z_ref_x,
    construct_basis_z_ref_y, construct_basis_z_y_exact, construct_basis_z_y_hint, is_virtual_node,
    query_pline,
};
use crate::{
    NamedAttrMap, RefnoEnum, SUL_DB, get_named_attmap, pdms_data::PlinParam,
    tool::direction_parse::parse_expr_to_dir,
};
use async_trait::async_trait;
use glam::{DMat3, DMat4, DQuat, DVec3};
use std::sync::Arc;
use crate::tool::math_tool;

/// POSL/PLIN 属性处理器  
pub struct PoslHandler;

/// YDIR/OPDI 属性处理器
pub struct YdirHandler;

/// CUTP 属性处理器
pub struct CutpHandler;

impl PoslHandler {
    /// 处理 POSL/PLIN 属性逻辑
    pub async fn handle_posl(
        att: &NamedAttrMap,
        parent_att: &NamedAttrMap,
        pos: &mut DVec3,
        quat: &mut DQuat,
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

                if !own_pos_line.is_empty() && own_pos_line != "NA" {
                    if let Ok(Some(own_param)) =
                        crate::query_pline(plin_owner, own_pos_line.into()).await
                    {
                        plin_pos -= own_param.pt;
                    }
                }
            } else {
                return Ok(());
            }

            let z_axis = if is_lmirror { -pline_plax } else { pline_plax };
            let plin_pos = if is_lmirror { -plin_pos } else { plin_pos };

            // YDIR 优先取自身的，如果没有则取 Owner 的
            let eff_ydir = parent_att.get_dvec3("YDIR").unwrap_or(DVec3::Y);
            let cur_type = att.get_type_str();

            // 对于 FITT 和 PLDATU 类型，使用 U 方向作为 Y 轴
            let final_ydir = if cur_type == "FITT" || cur_type == "PLDATU" {
                // 根据测试用例 "Y is U and Z is W"，这些类型的 Y 轴应该指向 U 方向
                DVec3::Z
            } else {
                eff_ydir
            };

            let mut local_quat = if cur_type == "SCOJ" {
                construct_basis_z_ref_x(z_axis)
            } else {
                construct_basis_z_y_exact(final_ydir, z_axis)
            };

            // 应用 BANG
            BangHandler::apply_bang(&mut local_quat, att);

            // 处理 DELP 和 ZDIS 属性：
            let mut local_offset = att.get_dvec3("DELP").unwrap_or(DVec3::ZERO);
            let mut offset = local_quat.mul_vec3(local_offset);
            if let Some(zdis) = att.get_f64("ZDIS") {
                offset.z += zdis;
            }
            // #[cfg(feature = "debug_spatial")]
            println!("  local_quat PDMS ORI: {}, offset is  {:?}", math_tool::dquat_to_pdms_ori_str(&local_quat, true), offset);


            // 最终位置 = PLINE 位置 + 旋转后的局部偏移 + 原始位置
            let final_pos = plin_pos + offset + *pos;

            // 更新传入的位置和朝向
            *pos = final_pos;
            *quat = local_quat;
        }

        Ok(())
    }
}

impl CutpHandler {
    /// 处理 CUTP 属性
    pub fn handle_cutp(att: &NamedAttrMap, quat: &mut DQuat) -> anyhow::Result<()> {
        let has_cut_dir = att.contains_key("CUTP");
        if has_cut_dir {
            let cut_dir = att.get_dvec3("CUTP").unwrap_or(DVec3::Z);
            let mat3 = DMat3::from_quat(*quat);
            *quat = construct_basis_x_cutplane(mat3.z_axis, cut_dir);
        }
        Ok(())
    }
}

pub struct DefaultStrategy {
    att: Arc<NamedAttrMap>,
    parent_att: Arc<NamedAttrMap>,
}

impl DefaultStrategy {
    pub fn new(att: Arc<NamedAttrMap>, parent_att: Arc<NamedAttrMap>) -> Self {
        Self { att, parent_att }
    }
}

#[async_trait]
impl TransformStrategy for DefaultStrategy {
    async fn get_local_transform(&mut self) -> anyhow::Result<Option<DMat4>> {
        // 获取所有需要的数据
        let att = &self.att;
        let parent_att = &self.parent_att;
        let cur_type = att.get_type_str();

        // 虚拟节点（如 SPINE）没有变换，直接跳过
        if is_virtual_node(cur_type) {
            return Ok(Some(DMat4::IDENTITY));
        }

        // 处理 NPOS 属性
        let mut position = att.get_position().unwrap_or_default().as_dvec3();
        let mut rotation = att.get_rotation().unwrap_or(DQuat::IDENTITY);
        NposHandler::apply_npos_offset(&mut position, att);

        // 调用 handle_posl 处理
        PoslHandler::handle_posl(att, parent_att, &mut position, &mut rotation).await?;

        // 处理 CUTP 属性（切割平面方向）
        // let has_opdir = att.contains_key("OPDIR");
        // let has_local_ori = !att.get_str("POSL").unwrap_or_default().is_empty();
        // let mut is_world_quat = false;

        // dbg!(&position);

        //todo need fix cutp ?
        // CutpHandler::handle_cutp(att, &mut rotation)?;

        // 构造最终的变换矩阵
        let mat4 = DMat4::from_rotation_translation(rotation, position);

        Ok(Some(mat4))
    }
}
