//! dblist 文件解析器
//!
//! 基于 NamedAttrMap 的 PDMS dblist 文件解析器

use crate::dblist_parser::attr_converter::AttrConverter;
use crate::dblist_parser::element::{ElementType, PdmsElement};
use anyhow::Result;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// dblist 文件解析器
pub struct DblistParser {
    /// 当前数据库编号
    current_dbno: i32,
    /// 下一个元素编号
    next_elno: i32,
    /// 元素栈，用于处理嵌套结构
    element_stack: Vec<PdmsElement>,
    /// 当前元素
    current_element: Option<PdmsElement>,
    /// 解析结果
    elements: Vec<PdmsElement>,
    /// 属性转换器
    attr_converter: AttrConverter,
}

impl DblistParser {
    /// 创建新的解析器
    pub fn new() -> Self {
        Self {
            current_dbno: 0,
            next_elno: 1,
            element_stack: Vec::new(),
            current_element: None,
            elements: Vec::new(),
            attr_converter: AttrConverter::default(),
        }
    }

    /// 从文件解析 dblist
    pub fn parse_file<P: AsRef<Path>>(&mut self, file_path: P) -> Result<Vec<PdmsElement>> {
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line?;
            let trimmed = line.trim();

            // 跳过空行和注释
            if trimmed.is_empty() || trimmed.starts_with('!') {
                continue;
            }

            self.process_line(trimmed)?;
        }

        self.finalize()
    }

    /// 处理单行内容
    fn process_line(&mut self, line: &str) -> Result<()> {
        if line.starts_with("NEW") {
            self.start_new_element(line)?;
        } else if line.starts_with("END") {
            self.end_element()?;
        } else if line.starts_with("DBNO") {
            self.set_database_number(line)?;
        } else if let Some((key, value)) = self.parse_attribute(line) {
            self.add_attribute(key, value)?;
        }

        Ok(())
    }

    /// 开始解析新元素
    fn start_new_element(&mut self, line: &str) -> Result<()> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            return Ok(());
        }

        let element_type = ElementType::from_str(parts[1])?;
        let refno = (self.current_dbno, self.next_elno);
        self.next_elno += 1;

        let element = PdmsElement::new(element_type, refno);

        if let Some(current) = self.current_element.take() {
            self.element_stack.push(current);
        }

        self.current_element = Some(element);
        Ok(())
    }

    /// 结束当前元素
    fn end_element(&mut self) -> Result<()> {
        if let Some(element) = self.current_element.take() {
            if let Some(mut parent) = self.element_stack.pop() {
                parent.add_child(element);
                self.current_element = Some(parent);
            } else {
                self.elements.push(element);
            }
        }
        Ok(())
    }

    /// 设置数据库编号
    fn set_database_number(&mut self, line: &str) -> Result<()> {
        if let Some(number_str) = line.split_whitespace().nth(1) {
            if let Ok(dbnum) = number_str.parse::<i32>() {
                self.current_dbno = dbnum;
                self.next_elno = 1;
            }
        }
        Ok(())
    }

    /// 解析属性行
    fn parse_attribute(&self, line: &str) -> Option<(String, String)> {
        // 属性格式: KEY VALUE
        if let Some(space_pos) = line.find(' ') {
            let key = line[..space_pos].trim().to_string();
            let value = line[space_pos..].trim().to_string();
            Some((key, value))
        } else {
            None
        }
    }

    /// 添加属性到当前元素
    fn add_attribute(&mut self, key: String, value: String) -> Result<()> {
        if let Some(ref mut element) = self.current_element {
            // 特殊处理位置属性
            if key == "POSITION" {
                element.position = Some(value.clone());
            }

            // 使用属性转换器转换属性值
            let noun = element.get_noun();
            let mut raw_attrs = BTreeMap::new();
            raw_attrs.insert(key.clone(), value);

            let converted_attrs = self.attr_converter.convert_attributes(noun, &raw_attrs)?;

            // 合并到现有属性中
            for (k, v) in converted_attrs.map {
                element.attributes.map.insert(k, v);
            }
        }
        Ok(())
    }

    /// 完成解析并返回结果
    fn finalize(&mut self) -> Result<Vec<PdmsElement>> {
        // 处理未结束的元素
        while let Some(element) = self.current_element.take() {
            if let Some(mut parent) = self.element_stack.pop() {
                parent.add_child(element);
                self.current_element = Some(parent);
            } else {
                self.elements.push(element);
                break;
            }
        }

        Ok(std::mem::take(&mut self.elements))
    }
}

impl Default for DblistParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_simple_element() {
        // 创建临时测试文件
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "NEW FRMWORK").unwrap();
        writeln!(temp_file, "NAME TestFramework").unwrap();
        writeln!(temp_file, "END").unwrap();
        temp_file.flush().unwrap();

        let mut parser = DblistParser::new();
        let elements = parser.parse_file(temp_file.path()).unwrap();

        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].element_type, ElementType::FrmFramework);
        assert!(elements[0].attributes.map.contains_key("NAME"));
    }

    #[test]
    fn test_nested_elements() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "NEW FRMWORK").unwrap();
        writeln!(temp_file, "NAME Framework").unwrap();
        writeln!(temp_file, "NEW PANEL").unwrap();
        writeln!(temp_file, "NAME Panel").unwrap();
        writeln!(temp_file, "END").unwrap();
        writeln!(temp_file, "END").unwrap();
        temp_file.flush().unwrap();

        let mut parser = DblistParser::new();
        let elements = parser.parse_file(temp_file.path()).unwrap();

        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].children.len(), 1);
        assert_eq!(elements[0].children[0].element_type, ElementType::Panel);
    }
}
