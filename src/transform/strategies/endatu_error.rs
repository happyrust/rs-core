use anyhow::anyhow;

/// ENDATU 专用错误类型，符合 core.dll 的错误码映射
#[derive(Debug, Clone, PartialEq)]
pub enum EndatuError {
    /// 无效的索引值 (对应 core.dll 错误码 251)
    InvalidIndex(u32),
    /// 坐标计算失败 (对应 core.dll 错误码 251)
    CoordinateCalculationFailed(i32),
    /// 属性缺失
    AttributeMissing(String),
    /// 变换矩阵错误
    TransformMatrixError,
    /// ZDIS 值超出有效范围
    InvalidZdisValue(f32),
    /// 零方向向量
    ZeroDirectionVector,
    /// 缓冲区溢出 (对应 core.dll 错误码 255)
    BufferOverflow,
    /// 通用几何计算错误
    GeometryCalculationError(String),
}

impl std::fmt::Display for EndatuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EndatuError::InvalidIndex(idx) => write!(f, "无效的 ENDATU 索引: {}", idx),
            EndatuError::CoordinateCalculationFailed(code) => write!(f, "坐标计算失败，错误码: {}", code),
            EndatuError::AttributeMissing(attr) => write!(f, "缺少必需属性: {}", attr),
            EndatuError::TransformMatrixError => write!(f, "变换矩阵计算错误"),
            EndatuError::InvalidZdisValue(zdis) => write!(f, "ZDIS 值超出有效范围: {}", zdis),
            EndatuError::ZeroDirectionVector => write!(f, "方向向量为零向量"),
            EndatuError::BufferOverflow => write!(f, "缓冲区溢出"),
            EndatuError::GeometryCalculationError(msg) => write!(f, "几何计算错误: {}", msg),
        }
    }
}

impl std::error::Error for EndatuError {}

impl EndatuError {
    /// 转换为 PDMS 兼容的错误码
    pub fn to_pdms_code(&self) -> i32 {
        match self {
            EndatuError::InvalidIndex(_) => 251,
            EndatuError::CoordinateCalculationFailed(code) => *code,
            EndatuError::BufferOverflow => 255,
            // 其他错误使用通用几何错误码
            _ => 252,
        }
    }

    /// 转换为 anyhow::Error
    pub fn to_anyhow(self) -> anyhow::Error {
        anyhow::anyhow!("{} (PDMS错误码: {})", self, self.to_pdms_code())
    }
}

/// ENDATU 结果类型别名
pub type EndatuResult<T> = Result<T, EndatuError>;
