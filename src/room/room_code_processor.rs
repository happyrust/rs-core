use crate::room::data_model::{RoomCode, ValidationError, ValidationResult, ValidationWarning};
use chrono::Utc;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tracing::{debug, info, warn};

/// 房间代码标准化处理器
///
/// 负责房间代码的解析、验证、标准化和格式转换
pub struct RoomCodeProcessor {
    /// 项目特定的房间代码规则
    project_rules: HashMap<String, ProjectRoomRule>,
    /// 缓存的房间代码映射
    code_cache: HashMap<String, RoomCode>,
}

/// 项目房间代码规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectRoomRule {
    /// 项目标识
    pub project_id: String,
    /// 项目名称
    pub project_name: String,
    /// 房间代码格式正则表达式
    pub code_pattern: String,
    /// 区域代码列表
    pub valid_area_codes: HashSet<String>,
    /// 房间号码范围
    pub room_number_range: (u32, u32),
    /// 是否允许字母房间号
    pub allow_alpha_room_number: bool,
    /// 特殊格式处理规则
    pub special_formats: Vec<SpecialFormatRule>,
}

/// 特殊格式处理规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialFormatRule {
    /// 规则名称
    pub name: String,
    /// 输入格式正则
    pub input_pattern: String,
    /// 输出格式模板
    pub output_template: String,
    /// 转换函数名
    pub transform_function: String,
}

/// 房间代码处理结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingResult {
    /// 原始输入
    pub original_input: String,
    /// 标准化后的房间代码
    pub standardized_code: Option<RoomCode>,
    /// 处理状态
    pub status: ProcessingStatus,
    /// 应用的规则
    pub applied_rules: Vec<String>,
    /// 处理消息
    pub messages: Vec<String>,
    /// 验证结果
    pub validation: ValidationResult,
}

/// 处理状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessingStatus {
    Success,
    Warning,
    Error,
    Skipped,
}

impl RoomCodeProcessor {
    /// 创建新的房间代码处理器
    pub fn new() -> Self {
        let mut processor = RoomCodeProcessor {
            project_rules: HashMap::new(),
            code_cache: HashMap::new(),
        };

        // 初始化默认项目规则
        processor.initialize_default_rules();
        processor
    }

    /// 初始化默认项目规则
    fn initialize_default_rules(&mut self) {
        // SSC 项目规则
        let ssc_rule = ProjectRoomRule {
            project_id: "SSC".to_string(),
            project_name: "石化项目".to_string(),
            code_pattern: r"^SSC-[A-Z]\d{3}$".to_string(),
            valid_area_codes: ["A", "B", "C", "D", "E", "F"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            room_number_range: (1, 999),
            allow_alpha_room_number: false,
            special_formats: vec![SpecialFormatRule {
                name: "5位房间号转换".to_string(),
                input_pattern: r"^SSC-([A-Z])(\d)(\d{3})$".to_string(),
                output_template: "SSC-{area}{number}".to_string(),
                transform_function: "convert_5digit_to_4digit".to_string(),
            }],
        };

        // HD 项目规则
        let hd_rule = ProjectRoomRule {
            project_id: "HD".to_string(),
            project_name: "海德项目".to_string(),
            code_pattern: r"^HD-[A-Z]\d{3}$".to_string(),
            valid_area_codes: ["A", "B", "C"].iter().map(|s| s.to_string()).collect(),
            room_number_range: (1, 999),
            allow_alpha_room_number: false,
            special_formats: vec![],
        };

        // HH 项目规则
        let hh_rule = ProjectRoomRule {
            project_id: "HH".to_string(),
            project_name: "华虹项目".to_string(),
            code_pattern: r"^HH-.*$".to_string(),
            valid_area_codes: HashSet::new(), // 允许任意区域代码
            room_number_range: (1, 9999),
            allow_alpha_room_number: true,
            special_formats: vec![],
        };

        self.project_rules.insert("SSC".to_string(), ssc_rule);
        self.project_rules.insert("HD".to_string(), hd_rule);
        self.project_rules.insert("HH".to_string(), hh_rule);
    }

    /// 处理房间代码
    pub fn process_room_code(&mut self, input: &str) -> ProcessingResult {
        let mut result = ProcessingResult {
            original_input: input.to_string(),
            standardized_code: None,
            status: ProcessingStatus::Error,
            applied_rules: Vec::new(),
            messages: Vec::new(),
            validation: ValidationResult {
                is_valid: false,
                errors: Vec::new(),
                warnings: Vec::new(),
                validated_at: Utc::now(),
            },
        };

        // 检查缓存
        if let Some(cached_code) = self.code_cache.get(input) {
            result.standardized_code = Some(cached_code.clone());
            result.status = ProcessingStatus::Success;
            result.messages.push("从缓存获取".to_string());
            return result;
        }

        // 预处理输入
        let cleaned_input = self.preprocess_input(input);
        result
            .messages
            .push(format!("预处理: {} -> {}", input, cleaned_input));

        // 尝试直接解析
        match RoomCode::parse(&cleaned_input) {
            Ok(code) => {
                // 验证项目规则
                if let Some(project_rule) = self.project_rules.get(&code.project_prefix) {
                    let validation = self.validate_with_project_rule(&code, project_rule);
                    result.validation = validation;

                    if result.validation.is_valid {
                        result.standardized_code = Some(code.clone());
                        result.status = ProcessingStatus::Success;
                        result
                            .applied_rules
                            .push(format!("项目规则: {}", project_rule.project_id));

                        // 缓存结果
                        self.code_cache.insert(input.to_string(), code);
                    } else {
                        result.status = ProcessingStatus::Error;
                        result.messages.push("项目规则验证失败".to_string());
                    }
                } else {
                    // 未知项目，尝试通用验证
                    match code.validate() {
                        Ok(_) => {
                            result.standardized_code = Some(code.clone());
                            result.status = ProcessingStatus::Warning;
                            result.messages.push("使用通用规则验证".to_string());
                            result.validation.warnings.push(ValidationWarning {
                                code: "UNKNOWN_PROJECT".to_string(),
                                message: format!("未知项目前缀: {}", code.project_prefix),
                                relation_id: None,
                                suggestion: Some("请添加项目规则配置".to_string()),
                            });
                        }
                        Err(e) => {
                            result.status = ProcessingStatus::Error;
                            result.validation.errors.push(ValidationError {
                                code: "VALIDATION_FAILED".to_string(),
                                message: e.to_string(),
                                relation_id: None,
                                details: HashMap::new(),
                            });
                        }
                    }
                }
            }
            Err(_) => {
                // 尝试特殊格式转换
                if let Some(converted_code) = self.try_special_format_conversion(&cleaned_input) {
                    result.standardized_code = Some(converted_code.clone());
                    result.status = ProcessingStatus::Success;
                    result.applied_rules.push("特殊格式转换".to_string());
                    result.messages.push("应用特殊格式转换规则".to_string());

                    // 缓存结果
                    self.code_cache.insert(input.to_string(), converted_code);
                } else {
                    result.status = ProcessingStatus::Error;
                    result.validation.errors.push(ValidationError {
                        code: "PARSE_FAILED".to_string(),
                        message: format!("无法解析房间代码: {}", cleaned_input),
                        relation_id: None,
                        details: HashMap::new(),
                    });
                }
            }
        }

        result
    }

    /// 预处理输入字符串
    fn preprocess_input(&self, input: &str) -> String {
        input
            .trim()
            .to_uppercase()
            .replace(" ", "")
            .replace("_", "-")
            .replace(".", "-")
    }

    /// 使用项目规则验证房间代码
    fn validate_with_project_rule(
        &self,
        code: &RoomCode,
        rule: &ProjectRoomRule,
    ) -> ValidationResult {
        let mut result = ValidationResult {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            validated_at: Utc::now(),
        };

        // 验证代码格式
        let regex = Regex::new(&rule.code_pattern).unwrap();
        if !regex.is_match(&code.full_code) {
            result.is_valid = false;
            result.errors.push(ValidationError {
                code: "PATTERN_MISMATCH".to_string(),
                message: format!("房间代码格式不匹配项目规则: {}", rule.code_pattern),
                relation_id: None,
                details: HashMap::new(),
            });
        }

        // 验证区域代码
        if !rule.valid_area_codes.is_empty() && !rule.valid_area_codes.contains(&code.area_code) {
            result.is_valid = false;
            result.errors.push(ValidationError {
                code: "INVALID_AREA_CODE".to_string(),
                message: format!("无效的区域代码: {}", code.area_code),
                relation_id: None,
                details: HashMap::new(),
            });
        }

        // 验证房间号码范围
        if let Ok(room_num) = code.room_number.parse::<u32>() {
            if room_num < rule.room_number_range.0 || room_num > rule.room_number_range.1 {
                result.warnings.push(ValidationWarning {
                    code: "ROOM_NUMBER_OUT_OF_RANGE".to_string(),
                    message: format!(
                        "房间号码超出范围 {}-{}: {}",
                        rule.room_number_range.0, rule.room_number_range.1, room_num
                    ),
                    relation_id: None,
                    suggestion: Some("请检查房间号码是否正确".to_string()),
                });
            }
        } else if !rule.allow_alpha_room_number {
            result.is_valid = false;
            result.errors.push(ValidationError {
                code: "ALPHA_ROOM_NUMBER_NOT_ALLOWED".to_string(),
                message: format!("项目不允许字母房间号: {}", code.room_number),
                relation_id: None,
                details: HashMap::new(),
            });
        }

        result
    }

    /// 尝试特殊格式转换
    fn try_special_format_conversion(&self, input: &str) -> Option<RoomCode> {
        for (_, rule) in &self.project_rules {
            for special_rule in &rule.special_formats {
                if let Ok(regex) = Regex::new(&special_rule.input_pattern) {
                    if let Some(captures) = regex.captures(input) {
                        return self.apply_special_conversion(captures, special_rule);
                    }
                }
            }
        }
        None
    }

    /// 应用特殊格式转换
    fn apply_special_conversion(
        &self,
        captures: regex::Captures,
        rule: &SpecialFormatRule,
    ) -> Option<RoomCode> {
        match rule.transform_function.as_str() {
            "convert_5digit_to_4digit" => {
                if captures.len() >= 4 {
                    let project = captures.get(0)?.as_str().split('-').next()?;
                    let area = captures.get(1)?.as_str();
                    let first_digit = captures.get(2)?.as_str();
                    let last_digits = captures.get(3)?.as_str();

                    // 转换 A1001 -> A001 格式
                    let room_number = format!("{}{}", first_digit, &last_digits[1..]);
                    let full_code = format!("{}-{}{}", project, area, room_number);

                    RoomCode::parse(&full_code).ok()
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// 批量处理房间代码
    pub fn batch_process(&mut self, inputs: Vec<String>) -> Vec<ProcessingResult> {
        inputs
            .into_iter()
            .map(|input| self.process_room_code(&input))
            .collect()
    }

    /// 获取处理统计信息
    pub fn get_processing_stats(&self) -> ProcessingStats {
        ProcessingStats {
            cache_size: self.code_cache.len(),
            project_rules_count: self.project_rules.len(),
            supported_projects: self.project_rules.keys().cloned().collect(),
        }
    }

    /// 添加项目规则
    pub fn add_project_rule(&mut self, rule: ProjectRoomRule) {
        info!("添加项目规则: {}", rule.project_id);
        self.project_rules.insert(rule.project_id.clone(), rule);
    }

    /// 清理缓存
    pub fn clear_cache(&mut self) {
        self.code_cache.clear();
        info!("房间代码缓存已清理");
    }

    /// 导出项目规则配置
    pub fn export_project_rules(&self) -> anyhow::Result<String> {
        serde_json::to_string_pretty(&self.project_rules)
            .map_err(|e| anyhow::anyhow!("导出项目规则失败: {}", e))
    }

    /// 导入项目规则配置
    pub fn import_project_rules(&mut self, json_config: &str) -> anyhow::Result<()> {
        let rules: HashMap<String, ProjectRoomRule> = serde_json::from_str(json_config)
            .map_err(|e| anyhow::anyhow!("解析项目规则配置失败: {}", e))?;

        for (project_id, rule) in rules {
            self.project_rules.insert(project_id, rule);
        }

        info!("成功导入 {} 个项目规则", self.project_rules.len());
        Ok(())
    }
}

/// 处理统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingStats {
    pub cache_size: usize,
    pub project_rules_count: usize,
    pub supported_projects: Vec<String>,
}

/// 全局房间代码处理器实例
static GLOBAL_PROCESSOR: tokio::sync::OnceCell<tokio::sync::Mutex<RoomCodeProcessor>> =
    tokio::sync::OnceCell::const_new();

/// 获取全局房间代码处理器
pub async fn get_global_processor() -> &'static tokio::sync::Mutex<RoomCodeProcessor> {
    GLOBAL_PROCESSOR
        .get_or_init(|| async { tokio::sync::Mutex::new(RoomCodeProcessor::new()) })
        .await
}

/// 便捷函数：处理单个房间代码
pub async fn process_room_code(input: &str) -> ProcessingResult {
    let processor = get_global_processor().await;
    let mut processor = processor.lock().await;
    processor.process_room_code(input)
}

/// 便捷函数：批量处理房间代码
pub async fn batch_process_room_codes(inputs: Vec<String>) -> Vec<ProcessingResult> {
    let processor = get_global_processor().await;
    let mut processor = processor.lock().await;
    processor.batch_process(inputs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_processor_creation() {
        let processor = RoomCodeProcessor::new();
        assert_eq!(processor.project_rules.len(), 3); // SSC, HD, HH
    }

    #[test]
    fn test_ssc_room_code_processing() {
        let mut processor = RoomCodeProcessor::new();

        // 测试标准格式
        let result = processor.process_room_code("SSC-A001");
        assert!(matches!(result.status, ProcessingStatus::Success));
        assert!(result.standardized_code.is_some());

        // 测试5位格式转换
        let result = processor.process_room_code("SSC-A1001");
        assert!(matches!(result.status, ProcessingStatus::Success));
        if let Some(code) = result.standardized_code {
            assert_eq!(code.room_number, "001");
        }
    }

    #[test]
    fn test_hd_room_code_processing() {
        let mut processor = RoomCodeProcessor::new();

        let result = processor.process_room_code("HD-B123");
        assert!(matches!(result.status, ProcessingStatus::Success));
        assert!(result.standardized_code.is_some());
    }

    #[test]
    fn test_hh_room_code_processing() {
        let mut processor = RoomCodeProcessor::new();

        // HH项目允许任意格式
        let result = processor.process_room_code("HH-ROOM001");
        assert!(matches!(
            result.status,
            ProcessingStatus::Warning | ProcessingStatus::Success
        ));
    }

    #[test]
    fn test_invalid_room_code() {
        let mut processor = RoomCodeProcessor::new();

        let result = processor.process_room_code("INVALID");
        assert!(matches!(result.status, ProcessingStatus::Error));
        assert!(!result.validation.errors.is_empty());
    }

    #[test]
    fn test_preprocessing() {
        let processor = RoomCodeProcessor::new();

        assert_eq!(processor.preprocess_input(" ssc-a001 "), "SSC-A001");
        assert_eq!(processor.preprocess_input("ssc_a001"), "SSC-A001");
        assert_eq!(processor.preprocess_input("ssc.a001"), "SSC-A001");
    }

    #[tokio::test]
    async fn test_global_processor() {
        let result = process_room_code("SSC-A001").await;
        assert!(matches!(result.status, ProcessingStatus::Success));
    }
}
