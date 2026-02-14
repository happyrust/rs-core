use std::cell::RefCell;
use std::collections::HashSet;
use std::{collections::HashMap, str::FromStr};

use crate::RefnoEnum;
use crate::pdms_types::PdmsGenericType;
use crate::{
    NamedAttrMap, NamedAttrValue, RefU64, SUL_DB, SurrealQueryExt, math::polish_notation::Stack,
    tiny_expr::expr_eval::interp, tool::float_tool::f64_round_3,
};
use dashmap::DashMap;
use derive_more::{Deref, DerefMut};
use once_cell::sync::Lazy;
use regex::{Captures, Regex};
use tokio::sync::RwLock;

//生成模型的中间过程中产生的伪属性，需要保存下来
//使用once_cell, 初始化一个dashmap, 后面去修改用这个dashmap来保存NamedAttMap
//加上tokio的读写锁，保证线程安全
pub static HASH_PSEUDO_ATT_MAPS: Lazy<RwLock<HashMap<String, NamedAttrMap>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

// 表达式求值错误去重：避免同一条错误在批量 BRAN/HANG 生成时刷屏，导致 I/O 拖慢整体生成。
pub static EXPR_EVAL_ERROR_ONCE: Lazy<DashMap<String, ()>> = Lazy::new(DashMap::new);

// 解析定位开关（读取一次环境变量，避免 eval 热路径反复读取 env）。
static EXPR_TRACE_ALL: Lazy<bool> = Lazy::new(|| {
    std::env::var("RS_EXPR_TRACE_ALL")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty() && v != "0")
        .is_some()
});
static EXPR_TRACE_BT: Lazy<bool> = Lazy::new(|| {
    std::env::var("RS_EXPR_TRACE_BT")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty() && v != "0")
        .is_some()
});

static COMPATIBLE_UNIT_MAP: Lazy<HashMap<&'static str, HashSet<&'static str>>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("INT", ["DIST"].into());
    m.insert("DIST", ["INT"].into());
    m
});

//todo 收集所有的种类，不在这里面的为NONE
#[inline]
pub fn check_unit_compatible(unit_a: &str, unit_b: &str) -> bool {
    unit_a == unit_b
        || (unit_a == "REAL" || unit_b == "REAL")
        || (unit_a == "NUME" || unit_b == "NUME")
        || (unit_a == "DATA" || unit_b == "DATA")
        || COMPATIBLE_UNIT_MAP
            .get(unit_a)
            .map(|x| x.contains(unit_b))
            .unwrap_or(false)
}

pub const INTERNAL_PDMS_EXPRESS: [&'static str; 28] = [
    "MAX", "MIN", "COS", "SIN", "LOG", "ABS", "POW", "SQR", "SQRT", "NOT", "AND", "OR", "ATAN",
    "ACOS", "ATAN2", "ASIN", "INT", "OF", "MOD", "NEGATE", "SUM", "TANF", "TAN", "TIMES", "MULT",
    "DIV", "ADD", "MINUS",
];

/// 元件库表达式相关的参数
#[derive(Debug, Clone, Deref, DerefMut)]
pub struct CataContext {
    #[deref]
    #[deref_mut]
    pub context: DashMap<String, String>,
    pub is_tubi: bool,
    /// 是否将保温层参数（IPARAM）代入几何计算。
    /// - false（默认）：IPARAM 全部为 0，生成物理几何模型
    /// - true：IPARAM 使用实际保温层厚度，生成含保温层的模型
    pub with_insulation: bool,

    // 调试信息字段（使用 RefCell 实现内部可变性，仅在 debug_model 开启时使用）
    pub debug_geo_refno: RefCell<Option<String>>, // 当前几何体参考号
    pub debug_geo_type: RefCell<Option<String>>,  // 几何体类型 (SCYL, SBOX等)
    pub debug_attr_name: RefCell<Option<String>>, // 当前属性名 (PRAD, PHEI等)
    pub debug_attr_index: RefCell<Option<usize>>, // 数组属性的索引
}

impl Default for CataContext {
    fn default() -> Self {
        Self {
            context: DashMap::new(),
            is_tubi: false,
            with_insulation: false,
            debug_geo_refno: RefCell::new(None),
            debug_geo_type: RefCell::new(None),
            debug_attr_name: RefCell::new(None),
            debug_attr_index: RefCell::new(None),
        }
    }
}

impl CataContext {
    pub fn insert(&self, key: impl Into<String>, value: impl Into<String>) {
        self.context.insert(key.into(), value.into());
    }
    pub fn get(&self, key: impl AsRef<str>) -> Option<String> {
        self.context.get(key.as_ref()).map(|x| x.value().clone())
    }
    pub fn contains_key(&self, key: impl AsRef<str>) -> bool {
        self.context.contains_key(key.as_ref())
    }

    #[inline]
    pub fn is_tubi(&self) -> bool {
        self.is_tubi
    }
}

pub const DDHEIGHT_STR: &'static str = "DDHEIGHT";
pub const DDRADIUS_STR: &'static str = "DDRADIUS";
pub const DDANGLE_STR: &'static str = "DDANGLE";

///创建desi参考号的元件库计算上下文
pub async fn get_or_create_cata_context(
    desi_refno: RefnoEnum,
    is_tubi: bool,
) -> anyhow::Result<CataContext> {
    let desi_att = crate::get_named_attmap(desi_refno).await?;
    let mut context = CataContext::default();
    context.is_tubi = is_tubi;
    if let Some(v) = desi_att.get_as_string("JUSL") {
        context.insert("JUSL".to_string(), v);
    }
    context.insert("DESI_REFNO".to_string(), desi_refno.to_string());
    let desp = desi_att.get_f32_vec("DESP").unwrap_or_default();
    crate::debug_model_debug!(
        "🔍 [DESP] desi_refno={}, DESP array: {:?}, raw_val: {:?}",
        desi_refno,
        desp,
        desi_att.get_val("DESP")
    );
    for i in 0..desp.len() {
        context.insert(format!("DESI{}", i + 1), desp[i].to_string());
        context.insert(format!("DESP{}", i + 1), desp[i].to_string());
    }
    let ddesp = desi_att.get_ddesp().unwrap_or_default();
    // dbg!(&ddesp);
    for i in 0..ddesp.len() {
        context.insert(format!("DDES{}", i + 1), ddesp[i].to_string());
    }

    let height = desi_att.get_as_string("HEIG").unwrap_or("0.0".into());
    context.insert(DDHEIGHT_STR.to_string(), height.clone());
    let angle = desi_att.get_as_string("ANGL").unwrap_or("0.0".into());
    context.insert(DDANGLE_STR.to_string(), angle.clone());
    let radi = desi_att.get_as_string("RADI").unwrap_or("0.0".into());
    context.insert(DDRADIUS_STR.to_string(), radi.clone());

    for (str, v) in &desi_att.map {
        let is_uda = str.starts_with(":");
        let n = str.to_uppercase();
        match v {
            NamedAttrValue::F32Type(d) => {
                if is_uda {
                    dbg!((&n, d));
                }
                context.insert(n, d.to_string());
            }
            NamedAttrValue::F32VecType(ds) => {
                for (i, d) in ds.into_iter().enumerate() {
                    context.insert(format!("{}{}", &n, i + 1), d.to_string());
                }
            }
            _ => {}
        }
    }

    //todo 保温层厚度参数
    // let iparams = self.query_ipara_from_ele(desi_refno).unwrap_or_default();
    // for i in 0..iparams.len() {
    //     context.insert(format!("IPAR{}", i + 1), iparams[i].to_string());
    //     context.insert(format!("IPARM{}", i + 1), iparams[i].to_string());
    // }

    context.insert("RS_DES_REFNO".to_string(), desi_refno.to_string());
    // dbg!(&desi_refno);
    //添加cata的信息
    crate::debug_model_debug!(
        "🔍 get_or_create_cata_context for desi_refno: {}",
        desi_refno
    );

    // 先尝试获取元件库参考号
    let cat_refno_opt = crate::get_cat_refno(desi_refno).await.ok().flatten();
    crate::debug_model_debug!("   元件库参考号: {:?}", cat_refno_opt);

    // 🔧 修复：如果有元件库参考号，直接使用它获取属性
    if let Some(cat_refno) = cat_refno_opt {
        crate::debug_model_debug!("   使用元件库参考号: {}", cat_refno);

        // 直接获取元件库的属性映射
        if let Ok(cata_attmap) = crate::get_named_attmap(cat_refno).await {
            crate::debug_model_debug!(
                "   ✅ 成功获取元件库 attmap, type: {}",
                cata_attmap.get_type_str()
            );

            // dbg!(&cata_attmap);
            context.insert(
                "RS_CATR_REFNO".to_string(),
                cata_attmap.get_refno_or_default().to_string(),
            );
            // dbg!(&cata_attmap);
            let params = cata_attmap.get_f32_vec("PARA").unwrap_or_default();

            // 🔍 调试输出：打印 PARA 数组
            crate::debug_model_debug!(
                "🔍 [PARA] desi_refno={:?}, PARA array: {:?}",
                desi_refno,
                params
            );

            for i in 0..params.len() {
                context.insert(format!("CPAR{}", i + 1), params[i].to_string());
                context.insert(format!("PARA{}", i + 1), params[i].to_string());
                context.insert(format!("PARAM{}", i + 1), params[i].to_string());
                context.insert(format!("IPARA{}", i + 1), "0".to_string());
                context.insert(format!("IPAR{}", i + 1), "0".to_string());
            }
            let mut owner_ref = desi_att.get_owner();
            //todo 需要换掉
            let mut owner_att = crate::get_named_attmap(owner_ref).await?;
            //todo use a single query to get all the ancestors' attmap
            while !owner_att.contains_key("GTYP") {
                if owner_att.get_refno().is_none() || owner_att.get_type_str() == "ZONE" {
                    break;
                }
                owner_ref = owner_att.get_owner();
                // owner_att = crate::get_named_attmap(owner_ref).await.unwrap_or_default();
                owner_att = crate::get_named_attmap(owner_ref).await?;
            }

            //dtse 的信息处理
            let dtre_refno = cata_attmap.get_foreign_refno("DTRE").unwrap_or_default();
            crate::debug_model_debug!("🔍 DTRE refno: {}", dtre_refno);
            let children = crate::get_children_named_attmaps(dtre_refno).await?;
            crate::debug_model_debug!("🔍 DTRE children count: {}", children.len());
            //如果只查部分数据，可以改一下接口
            for child in children {
                if let Some(k) = child.get_as_string("DKEY") {
                    let key = format!("RPRO_{}", &k);
                    let exp = child.get_as_string("PPRO").unwrap_or_default();
                    let default_key = format!("{}_default_expr", key);
                    let default_expr = child.get_as_string("DPRO").unwrap_or_default();
                    let type_key = format!("{}_default_type", key);
                    let type_value = child.get_as_string("PTYP").unwrap_or_default();
                    // crate::debug_model_debug!("🔍 添加 RPRO 键: {} = {}", key, exp);
                    context.insert(key, exp);
                    context.insert(default_key, default_expr);
                    context.insert(type_key, type_value);
                }
            }

            let desp = owner_att.get_f32_vec("DESP").unwrap_or_default();
            for i in 0..desp.len() {
                context.insert(format!("ODES{}", i + 1), desp[i].to_string());
            }
            //找到owner 参考号，再找到它的元件库params
            if let Ok(parent_cat_am) = crate::get_cat_attmap(owner_ref).await {
                let params = parent_cat_am.get_f32_vec("PARA").unwrap_or_default();
                for i in 0..params.len() {
                    context.insert(format!("OPAR{}", i + 1), params[i].to_string());
                }
            }
            let cref = desi_att.get_foreign_refno("CREF");
            if cref.is_some()
                && let Ok(c_att) = crate::get_named_attmap(cref.unwrap()).await
            {
                let desp = c_att.get_f32_vec("DESP").unwrap_or_default();
                for i in 0..desp.len() {
                    context.insert(format!("ADES{}", i + 1), desp[i].to_string());
                }
                let c_refno = c_att.get_refno().unwrap_or_default();

                if let Ok(attach_cat_am) = crate::get_cat_attmap(c_refno).await {
                    let params = attach_cat_am.get_f32_vec("PARA").unwrap_or_default();
                    for i in 0..params.len() {
                        context.insert(format!("APAR{}", i + 1), params[i].to_string());
                    }
                }
            }
        } else {
            crate::debug_model_debug!("   ❌ 无法获取元件库 attmap for cat_refno: {}", cat_refno);
        }
    } else {
        crate::debug_model_debug!("   ❌ 没有元件库参考号 for desi_refno: {}", desi_refno);
    }
    // dbg!(&context);
    Ok(context)
}

fn replace_all_result<E>(
    re: &Regex,
    haystack: &str,
    replacement: impl Fn(&Captures) -> Result<String, E>,
) -> Result<String, E> {
    let mut new = String::with_capacity(haystack.len());
    let mut last_match = 0;
    for caps in re.captures_iter(haystack) {
        let m = caps.get(0).unwrap();
        new.push_str(&haystack[last_match..m.start()]);
        new.push_str(&replacement(&caps)?);
        last_match = m.end();
    }
    new.push_str(&haystack[last_match..]);
    Ok(new)
}

pub fn prepare_eval_str(input: &str) -> String {
    // 🔧 修复：先处理 ATTRIB RPRO 组合，再删除 ATTRIB
    // 将 "ATTRIB RPRO LENG" 转换为 "RPRO_LENG"
    let attrib_rpro_re = Regex::new(r"ATTRIB\s+RPRO\s+([a-zA-Z0-9_]+)").unwrap();
    let step1 = attrib_rpro_re.replace_all(input, "RPRO_$1").to_string();

    step1
        .replace("IFTRUE", "if")
        .replace(" LT ", "<")
        .replace(" GT ", ">")
        .replace(" LE ", "<=")
        .replace(" GE ", ">=")
        .replace(" EQ ", "==")
        .replace("ATTRIB", "")
        .replace("DESIGN PARAM", "DESP")
        .replace("DESIGN PARA", "DESP")
}

fn rewrite_mat_trim_str_iftrue(input: &str) -> String {
    fn is_ws(ch: char) -> bool {
        ch.is_whitespace()
    }

    fn starts_with_keyword(chars: &[char], pos: usize, keyword: &str) -> bool {
        let end = pos + keyword.len();
        if end > chars.len() {
            return false;
        }
        chars[pos..end]
            .iter()
            .zip(keyword.chars())
            .all(|(c, k)| c.to_ascii_uppercase() == k)
    }

    fn skip_ws(chars: &[char], pos: &mut usize) {
        while *pos < chars.len() && is_ws(chars[*pos]) {
            *pos += 1;
        }
    }

    fn consume_keyword(chars: &[char], pos: &mut usize, keyword: &str) -> bool {
        skip_ws(chars, pos);
        if starts_with_keyword(chars, *pos, keyword) {
            *pos += keyword.len();
            true
        } else {
            false
        }
    }

    fn consume_char(chars: &[char], pos: &mut usize, expected: char) -> bool {
        skip_ws(chars, pos);
        if *pos < chars.len() && chars[*pos] == expected {
            *pos += 1;
            true
        } else {
            false
        }
    }

    fn consume_quoted_true(chars: &[char], pos: &mut usize) -> bool {
        skip_ws(chars, pos);
        if *pos >= chars.len() || chars[*pos] != '\'' {
            return false;
        }
        *pos += 1;
        let start = *pos;
        while *pos < chars.len() && chars[*pos] != '\'' {
            *pos += 1;
        }
        if *pos >= chars.len() {
            return false;
        }
        let value: String = chars[start..*pos].iter().collect();
        *pos += 1;
        value.eq_ignore_ascii_case("TRUE")
    }

    fn parse_mat_trim_str(chars: &[char], start: usize) -> Option<(String, usize)> {
        let mut pos = start;
        if !consume_keyword(chars, &mut pos, "MAT") {
            return None;
        }
        if !consume_char(chars, &mut pos, '(') {
            return None;
        }
        if !consume_keyword(chars, &mut pos, "TRIM") {
            return None;
        }
        if !consume_char(chars, &mut pos, '(') {
            return None;
        }
        if !consume_keyword(chars, &mut pos, "STR") {
            return None;
        }
        if !consume_char(chars, &mut pos, '(') {
            return None;
        }

        let inner_start = pos;
        let mut depth = 1usize;
        while pos < chars.len() {
            match chars[pos] {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            pos += 1;
        }
        if depth != 0 {
            return None;
        }

        let inner_expr: String = chars[inner_start..pos].iter().collect();
        pos += 1;

        if !consume_char(chars, &mut pos, ')') {
            return None;
        }
        if !consume_char(chars, &mut pos, ',') {
            return None;
        }
        if !consume_quoted_true(chars, &mut pos) {
            return None;
        }
        if !consume_char(chars, &mut pos, ')') {
            return None;
        }

        Some((format!("IFTRUE({},1,0)", inner_expr), pos))
    }

    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    let mut pos = 0usize;
    while pos < chars.len() {
        if starts_with_keyword(&chars, pos, "MAT") {
            if let Some((replacement, next_pos)) = parse_mat_trim_str(&chars, pos) {
                out.push_str(&replacement);
                pos = next_pos;
                continue;
            }
        }
        out.push(chars[pos]);
        pos += 1;
    }

    out
}

///评估表达式的值
pub fn eval_str_to_f64(
    input_expr: &str,
    context: &CataContext,
    dtse_unit: &str,
) -> anyhow::Result<f64> {
    // ---------------------------------------------------------------------
    // 解析定位：缺右括号问题（例如 "( ATTRIB PARA[10 ] / 2"）
    //
    // 背景：在 dbnum=7999 的 BRAN 生成中，观测到传入 eval 的子表达式会缺失尾部 ')'
    // （看起来像 take_until(')') 之类的抽取逻辑截断了字符串）。
    //
    // 这里做“最小侵入”的定位：
    // - 仅在括号不平衡时输出一次（去重），避免刷屏
    // - 默认只关注 PARA[10（可通过 RS_EXPR_TRACE_ALL=1 扩大范围）
    // - 可通过 RS_EXPR_TRACE_BT=1 输出 Rust backtrace，用于定位调用栈
    // ---------------------------------------------------------------------
    {
        // 默认仅跟踪 PARA[10；若要扩大范围，设置 RS_EXPR_TRACE_ALL=1。
        let should_trace = *EXPR_TRACE_ALL || *EXPR_TRACE_BT || input_expr.contains("PARA[10");
        if should_trace {
            let left = input_expr.matches('(').count();
            let right = input_expr.matches(')').count();
            if left != right && (*EXPR_TRACE_ALL || input_expr.contains("PARA[10")) {
                let des_refno_str: String = context.get("RS_DES_REFNO").unwrap_or_default();
                let cata_refno_str: String = context.get("RS_CATR_REFNO").unwrap_or_default();
                let geo_refno_str = context.debug_geo_refno.borrow().clone().unwrap_or_default();
                let geo_type_str = context.debug_geo_type.borrow().clone().unwrap_or_default();
                let attr_name_str = context.debug_attr_name.borrow().clone().unwrap_or_default();
                let attr_index_str = context
                    .debug_attr_index
                    .borrow()
                    .map(|i| format!("[{}]", i))
                    .unwrap_or_default();

                let dedup_key = format!(
                    "expr_trace_paren|{}|{}|{}|{}|{}",
                    des_refno_str, cata_refno_str, geo_refno_str, attr_name_str, input_expr
                );
                if EXPR_EVAL_ERROR_ONCE.insert(dedup_key, ()).is_none() {
                    eprintln!(
                        "[expr_trace] paren_mismatch left={} right={} dtse_unit={} des={} cata={} geo={}({}) attr={}{} expr={}",
                        left,
                        right,
                        dtse_unit,
                        des_refno_str,
                        cata_refno_str,
                        geo_refno_str,
                        geo_type_str,
                        attr_name_str,
                        attr_index_str,
                        input_expr
                    );
                    if *EXPR_TRACE_BT {
                        eprintln!(
                            "[expr_trace] backtrace:\n{}",
                            std::backtrace::Backtrace::force_capture()
                        );
                    } else {
                        eprintln!("[expr_trace] (set RS_EXPR_TRACE_BT=1 to print backtrace)");
                    }
                }
            }
        }
    }

    // 🔍 调试：记录输入的表达式（特别是包含 RPRO 的）
    if crate::debug_macros::is_debug_model_enabled()
        && (input_expr.contains("RPRO") || input_expr.contains("ATTRIB"))
    {
        // crate::debug_model_debug!("🔍 eval_str_to_f64 输入表达式: {}", input_expr);

        // 如果表达式缺少右括号，打印警告
        let left_count = input_expr.matches('(').count();
        let right_count = input_expr.matches(')').count();
        if left_count != right_count {
            crate::debug_model_debug!(
                "   ⚠️  括号不匹配！左括号: {}, 右括号: {}",
                left_count,
                right_count
            );
        }

        // 打印 context 中所有包含 RPRO 的键
        crate::debug_model_debug!("   Context 中的 RPRO 相关键:");
        let mut found_rpro = false;
        for entry in context.context.iter() {
            let key = entry.key();
            if key.contains("RPRO") {
                // crate::debug_model_debug!("     {} = {}", key, entry.value());
                found_rpro = true;
            }
        }
        if !found_rpro {
            crate::debug_model_debug!("     (没有找到 RPRO 相关的键)");
        }
    }

    if input_expr.is_empty() || input_expr == "UNSET" {
        return Ok(0.0);
    }
    #[cfg(feature = "debug_expr")]
    {
        dbg!(&input_expr);
        // 如果表达式包含 PARAM，打印 context 中的所有 PARAM 键
        if input_expr.contains("PARAM") {
            println!("Context PARAM keys:");
            for entry in context.context.iter() {
                let key = entry.key();
                if key.contains("PARAM") {
                    println!("  {} = {}", key, entry.value());
                }
            }
        }
    }
    let refno = context
        .get("RS_DES_REFNO")
        .and_then(|x| Some(RefnoEnum::from(x.as_str())))
        .unwrap_or_default();
    //处理引用的情况 OF 的情况, 如果需要获取 att value，还是需要用数据库去获取值
    let rewritten_expr = rewrite_mat_trim_str_iftrue(input_expr);
    // 兜底：对明显的“缺右括号”做温和修复，避免简单表达式（如 "( PARA[10]/2"）直接求值失败。
    // 仅在：左括号更多、差值不大、且不含引号（避免破坏字符串函数）时补齐。
    let rewritten_expr = {
        let s = rewritten_expr.trim().to_string();
        let left = s.matches('(').count();
        let right = s.matches(')').count();
        if left > right && (left - right) <= 4 && !s.contains('\'') && !s.contains('\"') {
            let mut fixed = s;
            fixed.push_str(&")".repeat(left - right));
            fixed
        } else {
            s
        }
    };
    if crate::debug_macros::is_debug_model_enabled() && rewritten_expr != input_expr {
        crate::debug_model_debug!(
            "   MAT/TRIM/STR rewrite: {} -> {}",
            input_expr,
            rewritten_expr
        );
    }
    let mut new_exp = prepare_eval_str(&rewritten_expr);

    // 🔍 调试：记录 prepare_eval_str 后的表达式
    if crate::debug_macros::is_debug_model_enabled()
        && (input_expr.contains("RPRO") || input_expr.contains("ATTRIB"))
    {
        crate::debug_model_debug!("   📝 prepare_eval_str 后: {}", new_exp);
    }

    #[cfg(feature = "debug_expr")]
    dbg!(&new_exp);
    if new_exp.contains(" OF ") {
        let re = Regex::new(r"([A-Z\s]+) OF (PREV|NEXT|\d+/\d+)").unwrap();
        for caps in re.captures_iter(&new_exp.clone()) {
            let s = &caps[0];
            let c1 = caps.get(1).map_or("", |m| m.as_str().trim());
            let c2 = caps.get(2).map_or("", |m| m.as_str().trim());
            let is_tubi = context.is_tubi();
            #[cfg(not(target_arch = "wasm32"))]
            {
                let expr_val = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async move {
                        //如果是直段，直接取当前的参考号
                        let target_refno = match c2 {
                            "PREV" => {
                                if is_tubi {
                                    refno
                                } else {
                                    crate::get_next_prev(refno, false).await.unwrap_or_default()
                                }
                            }
                            "NEXT" => crate::get_next_prev(refno, true).await.unwrap_or_default(),
                            _ => c2.into(),
                        };
                        // dbg!(target_refno);
                        let inst_info_key = {
                            let sql = format!(
                                "select value record::id(out) from {}->inst_relate limit 1;",
                                target_refno.to_pe_key()
                            );
                            SUL_DB
                                .query_take::<Option<String>>(&sql, 0)
                                .await
                                .unwrap_or(None)
                        };
                        let pseudo_map = HASH_PSEUDO_ATT_MAPS.read().await;
                        // #[cfg(feature = "debug_expr")]
                        // dbg!(&pseudo_map);
                        //判断target_refno是否在pseudo_map，如果有，取出这里的值
                        if let Some(inst_info_key) = inst_info_key.as_deref() {
                            if let Some(am) = pseudo_map.get(inst_info_key) {
                                if let Some(v) = am.map.get(c1) {
                                    return v.get_val_as_string();
                                }
                            }
                        }
                        "0".to_owned()
                    })
                });
                new_exp = new_exp.replace(s, expr_val.as_str());
            }
        }
    }

    #[cfg(feature = "debug_expr")]
    dbg!(&new_exp);

    //说明：匹配带小数的情况 PARA[1.1]，支持 UDA 属性名（以冒号开头，如 :HXYS[1]）
    // 修复：原 (:? 是非捕获组语法，改为 (:)? 匹配可选冒号
    let re =
        Regex::new(r"(:)?([A-Z_]+[0-9]*)(\s*\[?\s*(([1-9]\d*\.?\d*)|(0\.\d*[1-9]\s*))\s*\]?)?")
            .unwrap();
    // 将NEXT PREV 的值统一换成参考号，然后 context_params 要存储 参考号对应的 attr，要是它这个值没有求解，
    // 相当于要递归去求值
    // 匹配 "RPRO GANG" 或 "RPRO_GANG" 两种格式，修复单位验证被绕过的问题
    let rpro_re = Regex::new(r"(RPRO)[_\s]+([a-zA-Z0-9]+)").unwrap();
    if new_exp.contains("RPRO") {
        new_exp = replace_all_result(&rpro_re, &new_exp, |caps: &Captures| {
            // 无论匹配到的是空格还是下划线，都统一生成 RPRO_KEY 格式
            let key: String = format!("RPRO_{}", &caps[2]).into();
            let default_key: String = format!("RPRO_{}_default_expr", &caps[2]).into();
            let key_type: String = format!("RPRO_{}_default_type", &caps[2]).into();
            let unit_type = context.get(&key_type).unwrap_or_default();
            if (!unit_type.is_empty() && unit_type != dtse_unit)
                && !check_unit_compatible(dtse_unit, &unit_type)
            {
                #[cfg(feature = "debug_expr")]
                dbg!((&new_exp, &unit_type, dtse_unit));
                return Err(anyhow::anyhow!(
                    "DTSE 表达式 {new_exp} 有问题，可能单位不一致"
                ));
            } else {
                #[cfg(feature = "debug_expr")]
                dbg!((&new_exp, &unit_type, dtse_unit));
                let v = context
                    .get(&key)
                    .map(|x| x.to_string())
                    .unwrap_or("0".to_string());
                context.insert(format!("EXPR_HAS_DEFAULT"), "true");
                #[cfg(feature = "debug_expr")]
                dbg!(&v);
                if let Ok(t) = eval_str_to_f64(&v, &context, "DIST") {
                    #[cfg(feature = "debug_expr")]
                    dbg!(t);
                    Ok(t.to_string())
                } else {
                    context.context.remove("EXPR_HAS_DEFAULT");
                    Ok(context
                        .get(&default_key)
                        .map(|x| x.to_string())
                        .unwrap_or("0".to_string()))
                }
            }
        })?
        .trim()
        .to_string();
        #[cfg(feature = "debug_expr")]
        dbg!(&new_exp);
        if let Ok(s) = new_exp.parse::<f64>() {
            // dbg!(s);
            return Ok(s);
        }
    }
    let mut result_exp = new_exp.clone();
    //默认两次
    let mut found_replaced = false;
    let para_name_re =
        Regex::new(r"(DESI(GN)?\s+)?([I|C|O|A)]?PARA?M?)|DESP|(O|A|W|D)DESP?").unwrap();
    let mut uda_context_added = false;
    let mut uda_context = HashMap::new();

    // 🔍 调试：记录循环开始
    let is_debug_rpro = crate::debug_macros::is_debug_model_enabled()
        && (input_expr.contains("RPRO") || input_expr.contains("ATTRIB"));
    if is_debug_rpro {
        crate::debug_model_debug!("   🔁 开始替换循环，初始表达式: {}", result_exp);
    }

    const MAX_SUBSTITUTION_LOOPS: usize = 30;
    for loop_idx in 0..MAX_SUBSTITUTION_LOOPS {
        for caps in re.captures_iter(&new_exp) {
            let Some(m0) = caps.get(0) else {
                continue;
            };
            let s = m0.as_str().trim();
            if INTERNAL_PDMS_EXPRESS.contains(&s) {
                continue;
            }
            // 重要：re 允许“空格 + 数字”作为可选索引，这会把前缀函数误匹配成变量，
            // 例如在替换完 PARAM 后，"SUM 26.700 0" 可能被捕获为 "SUM 26.700"，
            // 进而被当作未知变量替换为 0，导致表达式最终变成 "0 0"。
            //
            // 这里做一个最小侵入的兜底：若本次匹配包含空格，且首 token 是内置函数名，则跳过替换。
            // （带括号的函数调用已由下方 starts_with('(') 规则处理，这里专门覆盖前缀函数风格）
            if s.contains(char::is_whitespace) {
                if let Some(first) = s.split_whitespace().next() {
                    if INTERNAL_PDMS_EXPRESS.contains(&first) {
                        continue;
                    }
                }
            }
            // 兜底：若该 token 后面直接跟着 '('，更像函数调用而非属性引用。
            // 这能避免“内置函数未入白名单”时被误替换（如 SQRT(...)）。
            if new_exp[m0.end()..].trim_start().starts_with('(') {
                continue;
            }
            // 捕获组: (1)冒号前缀 (2)属性名 (3)数组部分 (4)索引值
            let colon_prefix = caps.get(1).map_or("", |m| m.as_str());
            let mut para_name = caps.get(2).map_or("", |m| m.as_str());
            let c3 = caps.get(3).map_or("", |m| m.as_str());
            let c4 = caps.get(4).map_or("", |m| m.as_str());
            //处理掉PARA 和 PARAM的区别
            let is_some_param = para_name_re.is_match(para_name);
            if is_some_param {
                if para_name.ends_with("M") {
                    para_name = &para_name[0..para_name.len() - 1];
                }
            }
            // 小数向下取整，构造完整 key（包含可选的冒号前缀）
            let k: String = format!(
                "{}{}{}",
                colon_prefix,
                para_name,
                c4.parse::<f32>()
                    .map(|x| x.floor().to_string())
                    .unwrap_or_default()
            )
            .into();
            let is_uda = k.starts_with(":");
            if is_uda && !uda_context_added {
                let refno_str = context.get("RS_DES_REFNO").unwrap();
                // dbg!(&refno_str);
                let refno: RefnoEnum = refno_str.as_str().into();
                // dbg!(&k);
                let uda_map = NamedAttrMap::default();
                #[cfg(not(target_arch = "wasm32"))]
                let uda_map = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async move {
                        crate::get_named_attmap_with_uda(refno)
                            .await
                            .unwrap_or_default()
                    })
                });
                for (kk, vv) in uda_map.map {
                    let mut kk = kk.to_uppercase();
                    if let Some(stripped) = kk.strip_prefix("UDA_") {
                        kk = format!(":{}", stripped);
                    }
                    if kk.starts_with({ ":" }) {
                        match vv {
                            NamedAttrValue::F32Type(d) => {
                                let short_name = if kk.len() >= 5 {
                                    kk[..5].to_uppercase()
                                } else {
                                    kk.to_uppercase()
                                };
                                uda_context.insert(short_name, d.to_string());
                                uda_context.insert(kk, d.to_string());
                            }
                            NamedAttrValue::F32VecType(ds) => {
                                let short_name = if kk.len() >= 5 {
                                    kk[..5].to_uppercase()
                                } else {
                                    kk.to_uppercase()
                                };
                                for (i, d) in ds.into_iter().enumerate() {
                                    // dbg!(format!("{} ->{}{}", kk, &short_name, i+1));
                                    uda_context
                                        .insert(format!("{}{}", &short_name, i + 1), d.to_string());
                                    uda_context.insert(format!("{}{}", &kk, i + 1), d.to_string());
                                }
                            }
                            _ => {}
                        }
                    }
                }
                uda_context_added = true;
            }

            #[cfg(feature = "debug_expr")]
            if input_expr.contains("PARAM") {
                println!(
                    "Checking key: k='{}', s='{}', is_some_param={}, contains_key={}",
                    &k,
                    s,
                    is_some_param,
                    context.contains_key(&k)
                );
            }

            if context.contains_key(&k) {
                result_exp = result_exp.replace(s, &context.get(&k).unwrap());
                found_replaced = true;
            } else if is_uda && uda_context.contains_key(&k) {
                result_exp = result_exp.replace(s, &uda_context.get(&k).unwrap());
                found_replaced = true;
            } else if is_some_param {
                // 匹配到没有别的嵌套，比如 cos(DESP[1])，这种应该cos(DESP[1])整体结果为 0
                // dbg!(&result_exp);
                let hash_fallback_value =
                    context.get("EXPR_HAS_DEFAULT").unwrap_or_default() == "true";
                if dtse_unit == "DIST" && (!hash_fallback_value) {
                    result_exp = result_exp.replace(s, "NaN");
                    let re = Regex::new(r"\w+\(NaN\)").unwrap();
                    result_exp = re.replace_all(&result_exp, "0.0").to_string();
                    result_exp = result_exp.replace("NaN", " 0");
                    // println!("{input_expr}： {} not found, use {}.", &k, &result_exp);
                    //
                    found_replaced = true;
                } else {
                    return Err(anyhow::anyhow!(format!(
                        "{input_expr}:： {} not found.",
                        &k
                    )));
                }
            } else if !is_uda {
                // ⚠️ DTSE/RPRO 表达式里常出现裸属性名（如 HEIG），其实际来源仍可能在 RPRO 表中。
                // 若 context 无该裸属性，则尝试用 RPRO_{ATTR} 递归求值；否则统一回退 0，避免表达式残留未定义符号。
                //
                // 例：RPRO_TLEN = MIN(HEIG, PARA[3])，当 HEIG 缺失但存在 RPRO_HEIG 时，应继续展开并求值。
                let mut rpro_keys = Vec::new();
                rpro_keys.push(format!("RPRO_{}", para_name));
                if k != para_name {
                    rpro_keys.push(format!("RPRO_{}", &k));
                }

                let mut resolved: Option<String> = None;
                for rpro_key in rpro_keys {
                    if !context.contains_key(&rpro_key) {
                        continue;
                    }

                    let guard_key = format!("__RPRO_GUARD__{}", &rpro_key);
                    if context.contains_key(&guard_key) {
                        resolved = Some("0".to_string());
                        break;
                    }
                    context.insert(&guard_key, "1");

                    let default_key = format!("{}_default_expr", &rpro_key);
                    let expr = context
                        .get(&rpro_key)
                        .or_else(|| context.get(&default_key))
                        .unwrap_or_else(|| "0".to_string());

                    let val = match eval_str_to_f64(&expr, &context, dtse_unit) {
                        Ok(v) => v,
                        Err(_) => context
                            .get(&default_key)
                            .and_then(|d| eval_str_to_f64(&d, &context, dtse_unit).ok())
                            .unwrap_or(0.0),
                    };

                    context.context.remove(&guard_key);
                    resolved = Some(val.to_string());
                    break;
                }

                if let Some(v) = resolved {
                    result_exp = result_exp.replace(s, &v);
                    found_replaced = true;
                } else if dtse_unit == "DIST" {
                    result_exp = result_exp.replace(s, "0");
                    found_replaced = true;
                }
            }
        }

        // 先处理 ATTRIB RPRO 组合（在删除 ATTRIB 之前）
        // 将 "ATTRIB RPRO LENG" 转换为 "RPRO_LENG"
        let attrib_rpro_re = Regex::new(r"ATTRIB\s+RPRO\s+([a-zA-Z0-9_]+)").unwrap();
        if result_exp.contains("ATTRIB") && result_exp.contains("RPRO") {
            crate::debug_model_debug!("   🔄 替换前: {}", result_exp);
            result_exp = attrib_rpro_re
                .replace_all(&result_exp, "RPRO_$1")
                .to_string();
            crate::debug_model_debug!("   🔄 替换 ATTRIB RPRO 后: {}", result_exp);
            found_replaced = true;
        }

        //如果有RPRO 需要执行两次处理
        result_exp = result_exp.replace("ATTRIB", "");
        crate::debug_model_debug!("   🔄 删除 ATTRIB 后: {}", result_exp);

        if result_exp.contains("RPRO") {
            crate::debug_model_debug!("   🔄 开始替换 RPRO 引用");
            result_exp = rpro_re
                .replace_all(&result_exp, |caps: &Captures| {
                    // 统一使用 RPRO_KEY 格式
                    let key: String = format!("RPRO_{}", &caps[2]).into();
                    let default_key: String = format!("RPRO_{}_default_expr", &caps[2]).into();

                    let value = context.get(&key).map(|x| x.to_string()).unwrap_or(
                        context
                            .get(&default_key)
                            .map(|x| x.to_string())
                            .unwrap_or("0".to_string()),
                    );
                    crate::debug_model_debug!("     {} -> {}", key, value);
                    value
                })
                .trim()
                .to_string();
            crate::debug_model_debug!("   🔄 替换 RPRO 后: {}", result_exp);
            found_replaced = true;
        }
        // dbg!(&result_exp);
        new_exp = result_exp.clone();

        // 🔍 调试：记录每次循环后的结果
        if is_debug_rpro {
            crate::debug_model_debug!("   🔁 循环 {} 结束，当前表达式: {}", loop_idx, result_exp);
        }

        if !found_replaced {
            if is_debug_rpro {
                crate::debug_model_debug!("   ✅ 替换循环结束（没有更多替换）");
            }
            break;
        }
        found_replaced = false;
    }
    let seg_strs: Vec<String> = result_exp
        .split_whitespace()
        .map(|x| x.trim().into())
        .collect::<Vec<_>>();
    if seg_strs.len() == 0 {
        return Ok(0.0);
    }
    let mut result_string = String::new();
    let mut p_vals = vec![];
    for s in seg_strs {
        let upper_s = s.to_uppercase();
        match upper_s.as_str() {
            "TIMES" | "MULT" => p_vals.push("*".to_string()),
            "DIV" => p_vals.push("/".to_string()),
            "ADD" => p_vals.push("+".to_string()),
            "SUBTRACT" => p_vals.push("-".to_string()),
            "DDHEIGHT" => p_vals.push(context.get("DDHEIGHT").unwrap().to_string()),
            "DDRADIUS" => p_vals.push(context.get("DDRADIUS").unwrap().to_string()),
            "DDANGLE" => p_vals.push(context.get("DDANGLE").unwrap().to_string()),
            _ => {
                if upper_s.ends_with("mm") {
                    p_vals.push(upper_s[..upper_s.len() - 2].to_string());
                } else {
                    p_vals.push(upper_s.to_string())
                }
            }
        }
    }
    let mut i = 0;
    let mut new_vals = vec![];
    while i < p_vals.len() {
        if p_vals[i] == "TWICE" {
            if i + 1 < p_vals.len() {
                if let Ok(val) = p_vals[i + 1].parse::<f64>() {
                    let v = val * 2.0f64;
                    new_vals.push(v.to_string());
                }
            }
            i += 2;
        } else if p_vals[i] == "TANF" {
            if i + 2 < p_vals.len() {
                if let Ok(val) = p_vals[i + 1].parse::<f64>() {
                    if let Ok(angle) = p_vals[i + 2].parse::<f64>() {
                        {
                            let v = val * ((angle / 2.0).to_radians() as f64).tan();
                            new_vals.push(v.to_string());
                        }
                    }
                }
            }
            i += 3;
        } else {
            new_vals.push(p_vals[i].clone());
            i += 1;
        }
    }
    let mut i = 0;
    while i < new_vals.len() {
        if (new_vals[i] == "SUM" || new_vals[i] == "DIFFERENCE") && i < new_vals.len() - 2 {
            if new_vals[i] == "SUM" {
                result_string.push_str(&format!(
                    "({} {} {})",
                    new_vals[i + 1],
                    "+",
                    new_vals[i + 2]
                ));
            } else {
                result_string.push_str(&format!(
                    "({} {} {})",
                    new_vals[i + 1],
                    "-",
                    new_vals[i + 2]
                ));
            }
            i += 3;
        } else {
            result_string.push_str(new_vals[i].as_str());
            i += 1;
        }
        result_string.push_str(" ");
    }
    //排除两个连续的负号的情况
    let final_expr = result_string.trim().to_lowercase().replace("--", "");
    #[cfg(feature = "debug_expr")]
    dbg!(&final_expr);
    match interp(&final_expr) {
        Ok(val) => Ok(f64_round_3(val).into()),
        Err(_) => {
            return if let Ok(mut val) = evalexpr::eval(&final_expr) {
                return Ok(f64_round_3(val.as_float()?).into());
            } else if let Ok(mut stack) = Stack::init(&final_expr) {
                stack.eval().ok_or(anyhow::anyhow!(format!(
                    "后缀表达式求解失败 {}",
                    &input_expr
                )))
            } else {
                // #[cfg(debug_assertions)]
                let des_refno_str: String = context.get("RS_DES_REFNO").unwrap_or_default();
                let cata_refno_str: String = context.get("RS_CATR_REFNO").unwrap_or_default();

                // 获取调试信息（仅在 debug_model 开启时有值）
                let geo_refno_str = context.debug_geo_refno.borrow().clone().unwrap_or_default();
                let geo_type_str = context.debug_geo_type.borrow().clone().unwrap_or_default();
                let attr_name_str = context.debug_attr_name.borrow().clone().unwrap_or_default();
                let attr_index_str = context
                    .debug_attr_index
                    .borrow()
                    .map(|i| format!("[{}]", i))
                    .unwrap_or_default();

                // 默认不刷屏：仅在显式调试场景打印。
                // - debug_model 开启时：打印（便于定位具体几何/属性/表达式）
                // - 或设置环境变量 AIOS_PRINT_EXPR_EVAL_ERROR=1：打印（用于回归/诊断）
                let print_enabled = crate::is_debug_model_enabled()
                    || std::env::var_os("AIOS_PRINT_EXPR_EVAL_ERROR").is_some();
                if print_enabled {
                    // 根据是否有调试信息，输出不同格式的错误（去重避免刷屏）
                    let dedup_key = format!("{}|{}|{}", des_refno_str, cata_refno_str, &input_expr);
                    let should_print = EXPR_EVAL_ERROR_ONCE.insert(dedup_key, ()).is_none();
                    if should_print {
                        if !geo_refno_str.is_empty() && !attr_name_str.is_empty() {
                            // 有完整调试信息
                            println!(
                                "处理{}时，元件库{}里的输入表达式有误:\n  几何体: {} ({})\n  属性: {}{}\n  表达式: {}",
                                des_refno_str,
                                cata_refno_str,
                                geo_refno_str,
                                geo_type_str,
                                attr_name_str,
                                attr_index_str,
                                &input_expr
                            );
                        } else {
                            // 没有调试信息，使用原有格式
                            println!(
                                "处理{}时，{}元件库里的输入表达式有误 : {}",
                                des_refno_str, cata_refno_str, &input_expr
                            );
                        }
                    }
                }

                Err(anyhow::anyhow!(format!("求解失败 {}", &input_expr)))
            };
        }
    }
}

pub async fn resolve_expression(
    expr: &str,
    desi_refno: RefnoEnum,
    is_tubi: bool,
) -> anyhow::Result<f64> {
    let context = get_or_create_cata_context(desi_refno, is_tubi).await?;
    eval_str_to_f64(expr, &context, "DIST")
}

/// 通用的解析表达式的方法, 解析desi参考号下的 表达式值
/// 如果 desi_refno 为空，代表design的数据不需要参与计算
pub async fn resolve_expression_to_f32(
    expr: &str,
    desi_refno: RefnoEnum,
    is_tubi: bool,
) -> anyhow::Result<f32> {
    let context = get_or_create_cata_context(desi_refno, is_tubi).await?;
    eval_str_to_f32(expr, &context, "DIST")
}

pub fn eval_str_to_f32(
    input_expr: impl AsRef<str>,
    context: &CataContext,
    dtse_unit: &str,
) -> anyhow::Result<f32> {
    let input_expr = input_expr.as_ref().trim().to_uppercase();
    eval_str_to_f64(&input_expr, context, dtse_unit).map(|x| x as f32)
}

pub fn eval_str_to_f32_or_default(
    input_expr: impl AsRef<str>,
    context: &CataContext,
    dtse_unit: &str,
) -> f32 {
    eval_str_to_f32(input_expr, context, dtse_unit).unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;

    /// 测试 UDA 属性正则表达式匹配（RUS-116 修复）
    #[test]
    fn test_uda_attribute_regex() {
        // 修复后的正则：支持可选冒号前缀
        let re =
            Regex::new(r"(:)?([A-Z_]+[0-9]*)(\s*\[?\s*(([1-9]\d*\.?\d*)|(0\.\d*[1-9]\s*))\s*\]?)?")
                .unwrap();

        // 测试用例：普通属性
        let caps = re.captures("DESP[1]").unwrap();
        assert_eq!(caps.get(1).map(|m| m.as_str()), None); // 无冒号
        assert_eq!(caps.get(2).map(|m| m.as_str()), Some("DESP"));
        assert_eq!(caps.get(4).map(|m| m.as_str()), Some("1"));

        // 测试用例：UDA 属性（以冒号开头）- RUS-116 核心场景
        let caps = re.captures(":HXYS[1]").unwrap();
        assert_eq!(caps.get(1).map(|m| m.as_str()), Some(":")); // 有冒号
        assert_eq!(caps.get(2).map(|m| m.as_str()), Some("HXYS"));
        assert_eq!(caps.get(4).map(|m| m.as_str()), Some("1"));

        // 测试用例：UDA 属性索引 2
        let caps = re.captures(":HXYS[2]").unwrap();
        assert_eq!(caps.get(1).map(|m| m.as_str()), Some(":"));
        assert_eq!(caps.get(2).map(|m| m.as_str()), Some("HXYS"));
        assert_eq!(caps.get(4).map(|m| m.as_str()), Some("2"));

        // 测试用例：无数组索引的属性
        let caps = re.captures("PARA").unwrap();
        assert_eq!(caps.get(1).map(|m| m.as_str()), None);
        assert_eq!(caps.get(2).map(|m| m.as_str()), Some("PARA"));
        assert_eq!(caps.get(4).map(|m| m.as_str()), None);

        // 测试用例：无数组索引的 UDA 属性
        let caps = re.captures(":HXYS").unwrap();
        assert_eq!(caps.get(1).map(|m| m.as_str()), Some(":"));
        assert_eq!(caps.get(2).map(|m| m.as_str()), Some("HXYS"));
    }

    /// 测试 prepare_eval_str 处理 ATTRIB 关键字
    #[test]
    fn test_prepare_eval_str_attrib() {
        // ATTRIB :HXYS[1] -> :HXYS[1]
        let result = prepare_eval_str("ATTRIB :HXYS[1]");
        assert!(result.contains(":HXYS[1]"));
        assert!(!result.contains("ATTRIB"));

        // ( ATTRIB :HXYS[1] + ATTRIB :HXYS[2] ) -> ( :HXYS[1] + :HXYS[2] )
        let result = prepare_eval_str("( ATTRIB :HXYS[1] + ATTRIB :HXYS[2] )");
        assert!(result.contains(":HXYS[1]"));
        assert!(result.contains(":HXYS[2]"));
        assert!(!result.contains("ATTRIB"));
    }

    #[test]
    fn test_rewrite_mat_trim_str_iftrue() {
        let input = "( 2 * MAT( TRIM( STR( ( ATTRIB DESP[6 ] / 1 ) GT ( 50 * 1 ) ) ), 'TRUE' ) )";
        let output = rewrite_mat_trim_str_iftrue(input);
        assert!(output.contains("IFTRUE("));
        assert!(!output.contains("MAT("));
        assert!(!output.contains("TRIM("));
        assert!(!output.contains("STR("));
        assert!(output.contains("ATTRIB DESP[6 ]"));
        assert!(output.contains("GT"));

        let non_match = "MAT( STR( 1 ), 'TRUE' )";
        let unchanged = rewrite_mat_trim_str_iftrue(non_match);
        assert_eq!(unchanged, non_match);
    }

    #[test]
    fn test_rpro_nested_bare_attr_resolves_via_rpro_key() {
        let context = CataContext::default();
        context.insert("RPRO_TLEN", "( MIN ( HEIG , PARA[3 ] ) )");
        context.insert("RPRO_TLEN_default_expr", "0");
        context.insert("RPRO_HEIG", "100");
        context.insert("RPRO_HEIG_default_expr", "0");
        context.insert("PARA3", "50");

        let v = eval_str_to_f64("RPRO TLEN", &context, "DIST").unwrap();
        assert_eq!(v, 50.0);
    }

    #[test]
    fn test_eval_sqrt_is_treated_as_function_not_attr() {
        // 复现 dbnum=7999 日志中高频表达式形态：
        // ( ( SQRT( 3 ) * ATTRIB PARA[12 ] ) / 2 )
        //
        // 关键点：SQRT 必须被识别为内置函数（而不是被当作“裸属性名”替换成 0）。
        let context = CataContext::default();
        context.insert("PARA12", "4");

        let v =
            eval_str_to_f64("( ( SQRT( 3 ) * ATTRIB PARA[12 ] ) / 2 )", &context, "DIST").unwrap();

        assert!((v - 3.464).abs() < 1e-6, "v={}", v);
    }

    #[test]
    fn test_prefix_sum_not_misparsed_as_attr_with_index() {
        // 回归：在替换完 PARAM/IPARAM 后，表达式会变成 "SUM 26.700 0"，
        // 旧逻辑可能把 "SUM 26.700" 误当作“属性 SUM 的索引=26.700”，进而替换为 0，导致结果错误。
        let context = CataContext::default();
        context.insert("PARA4", "26.700");
        context.insert("IPARA1", "0");

        let v = eval_str_to_f64("SUM PARAM 4 IPARAM 1", &context, "DIST").unwrap();
        assert!((v - 26.7).abs() < 1e-6, "v={}", v);
    }

    #[test]
    fn test_iparam_should_resolve_to_ipara_even_if_iparam_key_exists() {
        // 回归：gen_model-dev 会把查询到的 IPARAM 写入 context（IPARAM1/IPARAM 1 等）。
        // 解析器必须仍然把 "IPARAM 1" 规整为 "IPARA1"（去掉末尾 'M'），否则会误用 IPARAM1 的值。
        //
        // 典型现象：SCTO 的 DIAMETERS[0] = SUM PARAM 2 IPARAM 1，
        // 若 IPARAM1 被误用为 bore=100，则 114 + 100 = 214（错误）。
        let context = CataContext::default();
        context.insert("PARA2", "114.0");
        context.insert("IPARA1", "0");

        // 模拟错误来源：DB 查询结果把 IPARAM1 写成了 100（例如误取 bore/ID）。
        context.insert("IPARAM1", "100");

        let v = eval_str_to_f64("SUM PARAM 2 IPARAM 1", &context, "DIST").unwrap();
        assert!((v - 114.0).abs() < 1e-6, "v={}", v);
    }
}
