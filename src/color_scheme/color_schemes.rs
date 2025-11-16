use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::pdms_types::PdmsGenericType;

/// 配色方案定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorScheme {
    pub name: String,
    pub description: String,
    pub colors: HashMap<String, [u8; 4]>,
}

/// 配色方案集合
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorSchemes {
    pub schemes: HashMap<String, ColorScheme>,
}

/// 配色方案管理器
#[derive(Debug, Clone)]
pub struct ColorSchemeManager {
    pub available_schemes: HashMap<String, ColorScheme>,
    pub current_scheme: String,
}

impl Default for ColorSchemeManager {
    fn default() -> Self {
        Self::default_schemes()
    }
}

impl ColorSchemeManager {
    /// 从配置文件加载配色方案
    pub fn load_from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let schemes: ColorSchemes = toml::from_str(&content)?;

        Ok(Self {
            available_schemes: schemes.schemes,
            current_scheme: "standard_pdms".to_string(),
        })
    }

    /// 获取当前配色方案
    pub fn get_current_scheme(&self) -> Option<&ColorScheme> {
        self.available_schemes.get(&self.current_scheme)
    }

    /// 切换配色方案
    pub fn set_current_scheme(&mut self, scheme_name: &str) -> bool {
        if self.available_schemes.contains_key(scheme_name) {
            self.current_scheme = scheme_name.to_string();
            true
        } else {
            false
        }
    }

    /// 获取指定几何类型的颜色
    pub fn get_color_for_type(&self, pdms_type: PdmsGenericType) -> Option<[u8; 4]> {
        let scheme = self.get_current_scheme()?;
        let type_name = format!("{:?}", pdms_type);

        if let Some(&rgba) = scheme.colors.get(&type_name) {
            Some(rgba)
        } else {
            // 回退到未知类型颜色
            scheme.colors.get("UNKOWN").copied()
        }
    }

    /// 获取所有可用配色方案的名称和描述
    pub fn get_available_schemes(&self) -> Vec<(String, String)> {
        self.available_schemes
            .iter()
            .map(|(key, scheme)| (key.clone(), scheme.name.clone()))
            .collect()
    }

    /// 创建默认配色方案管理器
    pub fn default_schemes() -> Self {
        let mut schemes = HashMap::new();

        // 标准 PDMS 配色方案
        let mut standard_colors = HashMap::new();
        standard_colors.insert("UNKOWN".to_string(), [192, 192, 192, 255]);
        standard_colors.insert("CE".to_string(), [0, 100, 200, 180]);
        standard_colors.insert("EQUI".to_string(), [255, 190, 0, 255]);
        standard_colors.insert("PIPE".to_string(), [255, 255, 0, 255]);
        standard_colors.insert("HANG".to_string(), [255, 126, 0, 255]);
        standard_colors.insert("STRU".to_string(), [0, 150, 255, 255]);
        standard_colors.insert("SCTN".to_string(), [188, 141, 125, 255]);
        standard_colors.insert("GENSEC".to_string(), [188, 141, 125, 255]);
        standard_colors.insert("WALL".to_string(), [150, 150, 150, 255]);
        standard_colors.insert("STWALL".to_string(), [150, 150, 150, 255]);
        standard_colors.insert("CWALL".to_string(), [120, 120, 120, 255]);
        standard_colors.insert("GWALL".to_string(), [173, 216, 230, 128]);
        standard_colors.insert("FLOOR".to_string(), [210, 180, 140, 255]);
        standard_colors.insert("CFLOOR".to_string(), [160, 130, 100, 255]);
        standard_colors.insert("PANE".to_string(), [220, 220, 220, 255]);
        standard_colors.insert("ROOM".to_string(), [144, 238, 144, 100]);
        standard_colors.insert("AREADEF".to_string(), [221, 160, 221, 80]);
        standard_colors.insert("HVAC".to_string(), [175, 238, 238, 255]);
        standard_colors.insert("EXTR".to_string(), [147, 112, 219, 255]);
        standard_colors.insert("REVO".to_string(), [138, 43, 226, 255]);
        standard_colors.insert("HANDRA".to_string(), [255, 215, 0, 255]);
        standard_colors.insert("CWBRAN".to_string(), [255, 140, 0, 255]);
        standard_colors.insert("CTWALL".to_string(), [176, 196, 222, 150]);
        standard_colors.insert("DEMOPA".to_string(), [255, 69, 0, 255]);
        standard_colors.insert("INSURQ".to_string(), [255, 182, 193, 255]);
        standard_colors.insert("STRLNG".to_string(), [0, 255, 255, 255]);

        schemes.insert("standard_pdms".to_string(), ColorScheme {
            name: "标准 PDMS 配色".to_string(),
            description: "与原始 PDMS 系统一致的标准配色方案".to_string(),
            colors: standard_colors,
        });

        Self {
            available_schemes: schemes,
            current_scheme: "standard_pdms".to_string(),
        }
    }

    /// 保存配色方案到文件
    pub fn save_to_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let schemes = ColorSchemes {
            schemes: self.available_schemes.clone(),
        };

        let content = toml::to_string_pretty(&schemes)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_schemes() {
        let manager = ColorSchemeManager::default_schemes();
        assert!(manager.get_current_scheme().is_some());

        // 测试获取特定类型的颜色
        let pipe_color = manager.get_color_for_type(PdmsGenericType::PIPE);
        assert_eq!(pipe_color, Some([255, 255, 0, 255]));

        let equi_color = manager.get_color_for_type(PdmsGenericType::EQUI);
        assert_eq!(equi_color, Some([255, 190, 0, 255]));
    }

    #[test]
    fn test_switch_scheme() {
        let mut manager = ColorSchemeManager::default_schemes();
        assert_eq!(manager.current_scheme, "standard_pdms");

        // 尝试切换到不存在的方案
        assert!(!manager.set_current_scheme("nonexistent"));
        assert_eq!(manager.current_scheme, "standard_pdms");
    }
}
