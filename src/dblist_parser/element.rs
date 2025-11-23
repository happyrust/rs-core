//! PDMS 元素数据结构

use crate::types::named_attmap::NamedAttrMap;
use crate::RefnoEnum;
use std::collections::VecDeque;

/// PDMS 元素
#[derive(Debug, Clone)]
pub struct PdmsElement {
    /// 元素类型
    pub element_type: ElementType,
    /// 元素编号 (dbno, elno)
    pub refno: (i32, i32),
    /// 属性映射（使用 NamedAttrMap）
    pub attributes: NamedAttrMap,
    /// 位置信息
    pub position: Option<String>,
    /// 子元素
    pub children: Vec<PdmsElement>,
}

/// PDMS 元素类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElementType {
    FrmFramework,
    Panel,
    Gensec,
    Spine,
    Poinsp,
    Pavert,
    Ploop,
    Jldatum,
    Pldatum,
    Fixing,
    Rladdr,
    Handra,
    Rpath,
    Pointr,
    // 可以继续添加更多类型
}

impl PdmsElement {
    /// 创建新的 PDMS 元素
    pub fn new(element_type: ElementType, refno: (i32, i32)) -> Self {
        Self {
            element_type,
            refno,
            attributes: NamedAttrMap::default(),
            position: None,
            children: Vec::new(),
        }
    }

    /// 添加子元素
    pub fn add_child(&mut self, child: PdmsElement) {
        self.children.push(child);
    }

    /// 获取元素名称
    pub fn get_noun(&self) -> &'static str {
        self.element_type.to_noun()
    }

    /// 获取完整的 refno 字符串
    pub fn get_refno_string(&self) -> String {
        format!("{}_{}", self.refno.0, self.refno.1)
    }
}

impl ElementType {
    /// 从字符串解析元素类型
    pub fn from_str(s: &str) -> anyhow::Result<Self> {
        match s.to_uppercase().as_str() {
            "FRMWORK" => Ok(ElementType::FrmFramework),
            "PANEL" => Ok(ElementType::Panel),
            "GENSEC" => Ok(ElementType::Gensec),
            "SPINE" => Ok(ElementType::Spine),
            "POINSP" => Ok(ElementType::Poinsp),
            "PAVERT" => Ok(ElementType::Pavert),
            "PLOOP" => Ok(ElementType::Ploop),
            "JLDATUM" => Ok(ElementType::Jldatum),
            "PLDATUM" => Ok(ElementType::Pldatum),
            "FIXING" => Ok(ElementType::Fixing),
            "RLADDR" => Ok(ElementType::Rladdr),
            "HANDRA" => Ok(ElementType::Handra),
            "RPATH" => Ok(ElementType::Rpath),
            "POINTR" => Ok(ElementType::Pointr),
            _ => Err(anyhow::anyhow!("未知的元素类型: {}", s)),
        }
    }

    /// 转换为名词字符串
    pub fn to_noun(&self) -> &'static str {
        match self {
            ElementType::FrmFramework => "FRMWORK",
            ElementType::Panel => "PANEL",
            ElementType::Gensec => "GENSEC",
            ElementType::Spine => "SPINE",
            ElementType::Poinsp => "POINSP",
            ElementType::Pavert => "PAVERT",
            ElementType::Ploop => "PLOOP",
            ElementType::Jldatum => "JLDATUM",
            ElementType::Pldatum => "PLDATUM",
            ElementType::Fixing => "FIXING",
            ElementType::Rladdr => "RLADDR",
            ElementType::Handra => "HANDRA",
            ElementType::Rpath => "RPATH",
            ElementType::Pointr => "POINTR",
        }
    }
}
