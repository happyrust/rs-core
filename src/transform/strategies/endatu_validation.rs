use crate::{NamedAttrMap};
use super::endatu_error::EndatuError;
use glam::DVec3;

/// ENDATU 属性验证器，符合 core.dll 的严格参数检查
pub struct EndatuValidator;

impl EndatuValidator {
    /// 验证 ENDATU 的所有必需属性
    pub fn validate_endatu_attributes(att: &NamedAttrMap) -> Result<(), EndatuError> {
        // 验证 ZDIS 属性
        Self::validate_zdis(att)?;
        
        // 验证方向向量属性
        Self::validate_direction_vectors(att)?;
        
        // 验证角度属性
        Self::validate_angles(att)?;
        
        Ok(())
    }

    /// 验证 ZDIS 值范围
    /// 
    /// core.dll 中 ZDIS 的有效范围是 -10000.0 到 10000.0
    /// 超出范围会导致坐标计算错误 (错误码 251)
    fn validate_zdis(att: &NamedAttrMap) -> Result<(), EndatuError> {
        if let Some(zdis) = att.get_f32("ZDIS") {
            if !(-10000.0..=10000.0).contains(&zdis) {
                return Err(EndatuError::InvalidZdisValue(zdis));
            }
            
            // 检查 ZDIS 是否为有效数值
            if !zdis.is_finite() {
                return Err(EndatuError::InvalidZdisValue(zdis));
            }
        }
        
        // 验证 PKDI 值（如果存在）
        if let Some(pkdi) = att.get_f32("PKDI") {
            if pkdi < 0.0 || pkdi > 1.0 {
                return Err(EndatuError::GeometryCalculationError(
                    format!("PKDI 值超出有效范围 [0,1]: {}", pkdi)
                ));
            }
            
            if !pkdi.is_finite() {
                return Err(EndatuError::GeometryCalculationError(
                    format!("PKDI 值无效: {}", pkdi)
                ));
            }
        }
        
        Ok(())
    }

    /// 验证方向向量
    /// 
    /// core.dll 对零方向向量有严格检查
    fn validate_direction_vectors(att: &NamedAttrMap) -> Result<(), EndatuError> {
        // 验证 OPDI 向量
        if let Some(opdi) = att.get_dvec3("OPDI") {
            if opdi.length_squared() == 0.0 {
                return Err(EndatuError::ZeroDirectionVector);
            }
            
            // 检查向量是否包含有效数值
            if !opdi.x.is_finite() || !opdi.y.is_finite() || !opdi.z.is_finite() {
                return Err(EndatuError::GeometryCalculationError(
                    "OPDI 向量包含无效数值".to_string()
                ));
            }
        }
        
        // 验证 YDIR 向量
        if let Some(ydir) = att.get_dvec3("YDIR") {
            if ydir.length_squared() == 0.0 {
                return Err(EndatuError::ZeroDirectionVector);
            }
            
            if !ydir.x.is_finite() || !ydir.y.is_finite() || !ydir.z.is_finite() {
                return Err(EndatuError::GeometryCalculationError(
                    "YDIR 向量包含无效数值".to_string()
                ));
            }
        }
        
        // 验证 CUTP 向量
        if let Some(cutp) = att.get_dvec3("CUTP") {
            if cutp.length_squared() == 0.0 {
                return Err(EndatuError::ZeroDirectionVector);
            }
            
            if !cutp.x.is_finite() || !cutp.y.is_finite() || !cutp.z.is_finite() {
                return Err(EndatuError::GeometryCalculationError(
                    "CUTP 向量包含无效数值".to_string()
                ));
            }
        }
        
        // 验证位置向量
        if let Some(pos) = att.get_position() {
            let pos_dvec = pos.as_dvec3();
            if !pos_dvec.x.is_finite() || !pos_dvec.y.is_finite() || !pos_dvec.z.is_finite() {
                return Err(EndatuError::GeometryCalculationError(
                    "位置向量包含无效数值".to_string()
                ));
            }
        }
        
        // 验证 NPOS 向量
        if let Some(npos) = att.get_vec3("NPOS") {
            let npos_dvec = npos.as_dvec3();
            if !npos_dvec.x.is_finite() || !npos_dvec.y.is_finite() || !npos_dvec.z.is_finite() {
                return Err(EndatuError::GeometryCalculationError(
                    "NPOS 向量包含无效数值".to_string()
                ));
            }
        }
        
        Ok(())
    }

    /// 验证角度值
    fn validate_angles(att: &NamedAttrMap) -> Result<(), EndatuError> {
        // 验证 BANG 角度
        if let Some(bang) = att.get_f32("BANG") {
            if !bang.is_finite() {
                return Err(EndatuError::GeometryCalculationError(
                    format!("BANG 角度值无效: {}", bang)
                ));
            }
            
            // core.dll 中 BANG 角度通常限制在 -360 到 360 度
            if bang < -360.0 || bang > 360.0 {
                // 这里只是警告，不返回错误，因为某些情况下可能需要更大的角度
                println!("⚠️  BANG 角度值超出常规范围 [-360, 360]: {}", bang);
            }
        }
        
        Ok(())
    }

    /// 验证变换矩阵的有效性
    pub fn validate_transform_matrix(matrix: &glam::DMat4) -> Result<(), EndatuError> {
        // 检查矩阵是否包含 NaN 或无穷大
        if matrix.x_axis.x.is_nan() || matrix.x_axis.y.is_nan() || matrix.x_axis.z.is_nan() ||
           matrix.y_axis.x.is_nan() || matrix.y_axis.y.is_nan() || matrix.y_axis.z.is_nan() ||
           matrix.z_axis.x.is_nan() || matrix.z_axis.y.is_nan() || matrix.z_axis.z.is_nan() ||
           matrix.w_axis.x.is_nan() || matrix.w_axis.y.is_nan() || matrix.w_axis.z.is_nan() {
            return Err(EndatuError::TransformMatrixError);
        }
        
        // 检查矩阵是否包含无穷大
        if !matrix.x_axis.x.is_finite() || !matrix.x_axis.y.is_finite() || !matrix.x_axis.z.is_finite() ||
           !matrix.y_axis.x.is_finite() || !matrix.y_axis.y.is_finite() || !matrix.y_axis.z.is_finite() ||
           !matrix.z_axis.x.is_finite() || !matrix.z_axis.y.is_finite() || !matrix.z_axis.z.is_finite() ||
           !matrix.w_axis.x.is_finite() || !matrix.w_axis.y.is_finite() || !matrix.w_axis.z.is_finite() {
            return Err(EndatuError::TransformMatrixError);
        }
        
        // 检查行列式是否为零（奇异矩阵）
        let det = matrix.determinant();
        if det.abs() < 1e-10 {
            return Err(EndatuError::TransformMatrixError);
        }
        
        Ok(())
    }

    /// 验证 ENDATU 索引的有效性
    /// 
    /// core.dll 中 ENDATU 索引只能是 0 或 1
    pub fn validate_endatu_index(index: Option<u32>) -> Result<(), EndatuError> {
        if let Some(idx) = index {
            if idx > 1 {
                return Err(EndatuError::InvalidIndex(idx));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::test_helpers::*;
    use crate::types::attval::AttrVal;

    #[test]
    fn test_zdis_validation() {
        let mut att = create_complete_test_attmap();
        
        // 有效的 ZDIS 值
        att.insert("ZDIS".to_string(), AttrVal::DoubleType(100.0).into());
        assert!(EndatuValidator::validate_zdis(&att).is_ok());
        
        // 无效的 ZDIS 值（超出范围）
        att.insert("ZDIS".to_string(), AttrVal::DoubleType(15000.0).into());
        assert!(EndatuValidator::validate_zdis(&att).is_err());
        
        // NaN 值
        att.insert("ZDIS".to_string(), AttrVal::DoubleType(f64::NAN).into());
        assert!(EndatuValidator::validate_zdis(&att).is_err());
    }

    #[test]
    fn test_direction_vector_validation() {
        let mut att = create_test_attmap_with_attributes();
        
        // 有效的方向向量
        att.insert("OPDI".to_string(), AttrVal::Vec3Type([1.0, 0.0, 0.0]).into());
        assert!(EndatuValidator::validate_direction_vectors(&att).is_ok());
        
        // 零向量
        att.insert("OPDI".to_string(), AttrVal::Vec3Type([0.0, 0.0, 0.0]).into());
        assert!(EndatuValidator::validate_direction_vectors(&att).is_err());
    }

    #[test]
    fn test_index_validation() {
        assert!(EndatuValidator::validate_endatu_index(Some(0)).is_ok());
        assert!(EndatuValidator::validate_endatu_index(Some(1)).is_ok());
        assert!(EndatuValidator::validate_endatu_index(Some(2)).is_err());
        assert!(EndatuValidator::validate_endatu_index(None).is_ok());
    }
}
