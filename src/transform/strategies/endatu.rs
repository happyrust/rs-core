use super::TransformStrategy;
use crate::rs_surreal::spatial::{
    SectionEnd, cal_cutp_ori, cal_ori_by_opdir, cal_ori_by_ydir, cal_ori_by_z_axis_ref_y,
    cal_spine_orientation_basis_with_ydir, cal_zdis_pkdi_in_section_by_spine, get_spline_path,
};
use crate::{NamedAttrMap, RefnoEnum};
use super::{EndatuError, EndatuResult, EndatuValidator, get_cached_endatu_index};
use async_trait::async_trait;
use glam::{DMat3, DMat4, DQuat, DVec3};

/// ENDATU 专用的 ZDIS 处理器
/// 
/// 基于 IDA Pro 分析的 core.dll 实现，严格遵循原系统的处理逻辑
pub struct EndAtuZdisHandler;

impl EndAtuZdisHandler {
    /// 处理 ENDATU 的特殊 ZDIS 逻辑
    /// 
    /// 这个实现完全符合 core.dll 的处理方式：
    /// 1. 使用缓存的索引查询
    /// 2. 严格的参数验证
    /// 3. 符合 PDMS 错误码的错误处理
    pub async fn handle_endatu_zdis(
        refno: RefnoEnum,
        parent_refno: RefnoEnum,
        att: &NamedAttrMap,
        pos: &mut DVec3,
        quat: &mut DQuat,
    ) -> EndatuResult<bool> {
        // 检查 ZDIS 属性是否存在
        if !att.contains_key("ZDIS") {
            return Ok(false);
        }

        // 验证属性
        EndatuValidator::validate_endatu_attributes(att)?;

        // 获取 ZDIS 值
        let zdis = att.get_f32("ZDIS")
            .ok_or_else(|| EndatuError::AttributeMissing("ZDIS".to_string()))?;

        // 使用缓存的索引查询，符合 core.dll 的性能优化策略
        let endatu_index = get_cached_endatu_index(parent_refno, refno).await?;
        
        // 验证索引有效性（core.dll 中 ENDATU 索引只能是 0 或 1）
        EndatuValidator::validate_endatu_index(endatu_index)?;

        // 根据 core.dll 的逻辑确定端部位置
        let section_end = match endatu_index {
            Some(0) => Some(SectionEnd::START),
            Some(1) => Some(SectionEnd::END),
            _ => None,
        };

        // 执行 ZDIS 计算
        if let Some(result) = cal_zdis_pkdi_in_section_by_spine(
            parent_refno,
            0.0, // PKDI 设为 0，符合 core.dll 的 ENDATU 处理
            zdis,
            section_end,
        )
        .await
        .map_err(|e| EndatuError::GeometryCalculationError(
            format!("ZDIS 计算失败: {}", e)
        ))? {
            *pos += result.1;
            *quat = result.0;
            return Ok(true);
        }

        Ok(false)
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
        // 使用 ENDATU 专用错误处理
        let result: EndatuResult<Option<DMat4>> = self.get_local_transform_with_error_handling(refno, parent_refno, att, parent_att)
            .await;
        result.map_err(|e: EndatuError| e.to_anyhow())
    }
}

impl EndAtuStrategy {
    /// 带有详细错误处理的变换计算
    async fn get_local_transform_with_error_handling(
        &self,
        refno: RefnoEnum,
        parent_refno: RefnoEnum,
        att: &NamedAttrMap,
        parent_att: &NamedAttrMap,
    ) -> EndatuResult<Option<DMat4>> {
        let cur_type = att.get_type_str();
        let parent_type = parent_att.get_type_str();

        // 验证输入参数
        if cur_type != "ENDATU" {
            return Err(EndatuError::AttributeMissing(
                format!("期望类型 ENDATU，实际类型: {}", cur_type)
            ));
        }

        // 验证所有属性
        EndatuValidator::validate_endatu_attributes(att)?;

        let mut rotation = DQuat::IDENTITY;
        let mut translation = DVec3::ZERO;
        let mut pos = att.get_position()
            .unwrap_or_default()
            .as_dvec3();
        let mut quat = DQuat::IDENTITY;

        // === 属性处理优先级（完全符合 core.dll 顺序） ===

        // 1. ZDIS (最高优先级) - 如果存在且处理成功，直接返回
        if EndAtuZdisHandler::handle_endatu_zdis(
            refno, parent_refno, att, &mut pos, &mut quat
        ).await? {
            translation = translation + rotation * pos;
            rotation = quat;
            
            let mat4 = DMat4::from_rotation_translation(rotation, translation);
            EndatuValidator::validate_transform_matrix(&mat4)?;
            return Ok(Some(mat4));
        }

        // 2. NPOS (次高优先级)
        if att.contains_key("NPOS") {
            let npos = att.get_vec3("NPOS")
                .ok_or_else(|| EndatuError::AttributeMissing("NPOS".to_string()))?;
            pos += npos.as_dvec3();
        }

        // 3. OPDI (操作方向) - 高优先级，覆盖其他方向计算
        let mut has_opdir = false;
        if let Some(opdir) = att.get_dvec3("OPDI") {
            if opdir.length_squared() == 0.0 {
                return Err(EndatuError::ZeroDirectionVector);
            }
            quat = cal_ori_by_opdir(opdir.normalize());
            has_opdir = true;
        }

        // 4. 如果没有 OPDI，处理 YDIR
        if !has_opdir {
            // 获取父级挤出方向
            let (pos_extru_dir, spine_ydir) = Self::extract_extrusion_direction(
                parent_refno, parent_type, att
            ).await?;

            // 初始化基础旋转
            Self::initialize_rotation(
                att, cur_type, parent_type, pos_extru_dir, spine_ydir, 
                &mut quat, false
            ).await?;

            // 处理 YDIR
            if let Some(ydir_axis) = att.get_dvec3("YDIR") {
                if ydir_axis.length_squared() == 0.0 {
                    return Err(EndatuError::ZeroDirectionVector);
                }

                let z_axis = if let Some(axis) = Self::extract_extrusion_direction(
                    parent_refno, parent_type, att
                ).await?.0 {
                    axis
                } else {
                    DVec3::X
                };
                
                quat = cal_ori_by_ydir(ydir_axis.normalize(), z_axis);
            }

            // 5. BANG (基础角度) - 在方向确定后应用
            let bangle = att.get_f32("BANG").unwrap_or_default() as f64;
            if att.contains_key("BANG") && bangle != 0.0 {
                quat = quat * DQuat::from_rotation_z(bangle.to_radians());
            }

            // 6. CUTP (切割方向) - 仅在没有明确方向时使用
            let has_local_ori = att.get_rotation().is_some();
            let has_cut_dir = att.contains_key("CUTP");
            
            if has_cut_dir && !has_opdir && !has_local_ori {
                let cut_dir = att.get_dvec3("CUTP")
                    .ok_or_else(|| EndatuError::AttributeMissing("CUTP".to_string()))?;
                
                if cut_dir.length_squared() == 0.0 {
                    return Err(EndatuError::ZeroDirectionVector);
                }

                let mat3 = DMat3::from_quat(rotation);
                quat = cal_cutp_ori(mat3.z_axis, cut_dir);
            }
        }

        // 7. DELP (增量位置) - ENDATU 通常没有 POSL，但可能有 DELP
        if att.contains_key("DELP") {
            let delp = att.get_dvec3("DELP")
                .ok_or_else(|| EndatuError::AttributeMissing("DELP".to_string()))?;
            pos += delp;
        }

        // 最终变换计算
        translation = translation + rotation * pos;
        rotation = rotation * quat;

        let mat4 = DMat4::from_rotation_translation(rotation, translation);
        
        // 验证最终结果
        EndatuValidator::validate_transform_matrix(&mat4)?;

        Ok(Some(mat4))
    }

    /// 提取挤出方向信息（优化版本）
    async fn extract_extrusion_direction(
        parent_refno: RefnoEnum,
        parent_type: &str,
        att: &NamedAttrMap,
    ) -> EndatuResult<(Option<DVec3>, Option<DVec3>)> {
        let parent_is_gensec = parent_type == "GENSEC";
        
        if parent_is_gensec {
            if let Ok(spine_paths) = get_spline_path(parent_refno).await {
                if let Some(first_spine) = spine_paths.first() {
                    let dir = (first_spine.pt1 - first_spine.pt0).normalize();
                    if dir.length_squared() > 0.01 {
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
        }
        
        // 处理 DPOSE/DPOSS 属性
        if let (Some(end), Some(start)) = (att.get_dpose(), att.get_dposs()) {
            let dir = end - start;
            if dir.length_squared() > 0.01 {
                return Ok((Some(dir.normalize()), None));
            }
        }
        
        Ok((None, None))
    }
    
    /// 初始化旋转（优化版本）
    async fn initialize_rotation(
        att: &NamedAttrMap,
        cur_type: &str,
        parent_type: &str,
        pos_extru_dir: Option<DVec3>,
        spine_ydir: Option<DVec3>,
        quat: &mut DQuat,
        _is_world_quat: bool,
    ) -> EndatuResult<()> {
        let parent_is_gensec = parent_type == "GENSEC";
        let quat_v = att.get_rotation();
        let has_local_ori = quat_v.is_some();
        
        if (!parent_is_gensec && has_local_ori) || (parent_is_gensec && cur_type == "TMPL") {
            *quat = quat_v.unwrap_or_default();
        } else {
            if let Some(z_axis) = pos_extru_dir {
                if !z_axis.is_normalized() {
                    return Err(EndatuError::GeometryCalculationError(
                        "Z轴方向向量未归一化".to_string()
                    ));
                }

                if parent_is_gensec {
                    *quat = cal_spine_orientation_basis_with_ydir(z_axis, spine_ydir, false);
                } else {
                    *quat = cal_ori_by_z_axis_ref_y(z_axis);
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transform::strategies::EndatuValidator;
    use crate::test::test_helpers::create_test_attmap_with_attributes;
    use crate::types::attval::AttrVal;

    #[tokio::test]
    async fn test_endatu_zdis_handler() {
        let mut att = create_test_attmap_with_attributes();
        att.insert("ZDIS".to_string(), AttrVal::DoubleType(100.0).into());
        att.insert("OPDI".to_string(), AttrVal::Vec3Type([1.0, 0.0, 0.0]).into());
        assert!(EndatuValidator::validate_endatu_attributes(&att).is_ok());
        
        // 无效的 ZDIS
        att.insert("ZDIS".to_string(), AttrVal::DoubleType(15000.0).into());
        assert!(matches!(
            EndatuValidator::validate_endatu_attributes(&att),
            Err(EndatuError::InvalidZdisValue(_))
        ));
    }
}
