use std::collections::{BTreeMap, HashMap, HashSet};
use std::ops::Neg;
use std::panic;
use std::time::Instant;

use crate::expression::resolve_helper::{
    parse_str_axis_to_vec3, resolve_axis_with_cache, resolve_to_cate_geo_params,
};
use crate::parsed_data::geo_params_data::CateGeoParam;
use crate::parsed_data::{CateAxisParam, GmseParamData, ResolvedPlinePoint};
use crate::pdms_data::{AxisParam, GmParam, PlinParam, ScomInfo};
use crate::pdms_types::RefU64;
use crate::shape::pdms_shape::RsVec3;
use crate::tool::db_tool::db1_dehash;
use crate::{
    CataContext, DDANGLE_STR, DDHEIGHT_STR, DDRADIUS_STR, RefnoEnum, eval_str_to_f32_or_default,
};
use dashmap::DashMap;
use glam::{Vec2, Vec3};
use once_cell::sync::Lazy;

pub static SCOM_INFO_MAP: Lazy<DashMap<RefnoEnum, ScomInfo>> = Lazy::new(DashMap::new);

#[derive(Default)]
pub(crate) struct AxisResolveTraceStats {
    pub(crate) axis_calls: usize,
    pub(crate) cache_hit: usize,
    pub(crate) cycle_guard_hit: usize,
    pub(crate) total_ms: u128,
    pub(crate) resolve_axis_core_ms: u128,
    pub(crate) scalar_eval_ms: u128,
    pub(crate) parse_dir_ms: u128,
    pub(crate) parse_ref_dir_ms: u128,
    pub(crate) p_ref_lookup_ms: u128,
    pub(crate) p_ref_hit: usize,
    pub(crate) p_ref_miss: usize,
    pub(crate) ptpos_follow_ms: u128,
    pub(crate) scalar_fast_path_hit: usize,
    pub(crate) scalar_ctx_fast_hit: usize,
    pub(crate) scalar_fast_path_fallback: usize,
    pub(crate) scalar_fallback_expr_hits: HashMap<String, usize>,
}

#[derive(Default)]
pub(crate) struct ResolveEvalCache {
    expr_values: HashMap<(String, String), f32>,
    axis_dirs: HashMap<String, Option<Vec3>>,
    axis_params: HashMap<i32, CateAxisParam>,
    axis_resolving: HashSet<i32>,
    pub(crate) axis_trace: AxisResolveTraceStats,
    pub(crate) axis_trace_enabled: bool,
    pub(crate) axis_trace_refno: Option<RefnoEnum>,
}

#[inline]
fn normalize_cache_expr(expr: &str) -> String {
    expr.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[inline]
fn try_parse_simple_number(expr: &str) -> Option<f32> {
    let mut s = expr.trim();
    if s.is_empty() {
        return None;
    }
    while s.starts_with('(') && s.ends_with(')') && s.len() > 2 {
        s = s[1..s.len() - 1].trim();
    }
    if s.is_empty() {
        return None;
    }
    let mut has_digit = false;
    for b in s.as_bytes() {
        if b.is_ascii_digit() {
            has_digit = true;
            continue;
        }
        match *b {
            b'+' | b'-' | b'.' | b'e' | b'E' => {}
            _ => return None,
        }
    }
    if !has_digit {
        return None;
    }
    s.parse::<f32>().ok()
}

#[inline]
fn try_eval_context_scalar(expr: &str, context: &CataContext) -> Option<f32> {
    let mut upper = expr.trim().to_uppercase();
    if upper.is_empty() {
        return None;
    }
    if let Some(rest) = upper.strip_prefix("ATTRIB") {
        upper = rest.trim_start().to_string();
    }
    let compact = upper.split_whitespace().collect::<String>();
    if compact.is_empty() {
        return None;
    }
    if compact
        .as_bytes()
        .iter()
        .any(|b| matches!(*b, b'+' | b'-' | b'*' | b'/' | b'(' | b')' | b','))
    {
        return None;
    }

    let mut key = compact.trim_start_matches(':').to_string();
    if let Some(l) = key.find('[')
        && key.ends_with(']')
    {
        let head = &key[..l];
        let idx = key[l + 1..key.len() - 1].trim();
        if idx.is_empty() || !idx.as_bytes().iter().all(|b| b.is_ascii_digit()) {
            return None;
        }
        key = format!("{head}{idx}");
    }
    if !key
        .as_bytes()
        .iter()
        .all(|b| b.is_ascii_alphanumeric() || *b == b'_')
    {
        return None;
    }
    context.get(&key).and_then(|v| v.parse::<f32>().ok())
}

#[inline]
fn eval_str_to_f32_cached(
    expr: &str,
    context: &CataContext,
    dtse_unit: &str,
    cache: &mut ResolveEvalCache,
) -> f32 {
    let key = (dtse_unit.to_string(), normalize_cache_expr(expr));
    if let Some(val) = cache.expr_values.get(&key) {
        return *val;
    }
    let val = if let Some(v) = try_parse_simple_number(&key.1) {
        if cache.axis_trace_enabled {
            cache.axis_trace.scalar_fast_path_hit += 1;
        }
        v
    } else if let Some(v) = try_eval_context_scalar(&key.1, context) {
        if cache.axis_trace_enabled {
            cache.axis_trace.scalar_ctx_fast_hit += 1;
        }
        v
    } else {
        if cache.axis_trace_enabled {
            cache.axis_trace.scalar_fast_path_fallback += 1;
            *cache
                .axis_trace
                .scalar_fallback_expr_hits
                .entry(key.1.clone())
                .or_insert(0) += 1;
        }
        eval_str_to_f32_or_default(expr, context, dtse_unit)
    };
    cache.expr_values.insert(key, val);
    val
}

#[inline]
pub(crate) fn parse_axis_to_vec3_cached(
    axis_expr: &str,
    context: &CataContext,
    cache: &mut ResolveEvalCache,
) -> Option<Vec3> {
    let key = normalize_cache_expr(axis_expr);
    if let Some(v) = cache.axis_dirs.get(&key) {
        return *v;
    }
    let v = parse_str_axis_to_vec3(axis_expr, context).ok();
    cache.axis_dirs.insert(key, v);
    v
}

fn resolve_trace_refno_filter() -> Option<String> {
    std::env::var("AIOS_CATA_P1_TRACE_REFNO")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn should_trace_resolve_detail(desi_refno: RefnoEnum) -> bool {
    let Some(target) = resolve_trace_refno_filter() else {
        return false;
    };
    let target_normalized = target.replace('/', "_");
    target == desi_refno.to_string()
        || target_normalized == desi_refno.to_string()
        || target == desi_refno.to_e3d_id()
}

fn resolve_gm_trace_topn() -> usize {
    std::env::var("AIOS_RESOLVE_GM_TRACE_TOPN")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|&v| v > 0)
        .unwrap_or(5)
}

fn resolve_gm_trace_threshold_ms() -> u128 {
    std::env::var("AIOS_RESOLVE_GM_TRACE_THRESHOLD_MS")
        .ok()
        .and_then(|v| v.parse::<u128>().ok())
        .unwrap_or(200)
}

// #region agent log
fn agent_now_ms() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

fn agent_run_id() -> String {
    std::env::var("AIOS_AGENT_RUNID").unwrap_or_else(|_| "run1".to_string())
}

fn agent_match_refno(refno: RefnoEnum) -> bool {
    let Ok(target) = std::env::var("AIOS_AGENT_DEBUG_GEOM_REFNO") else {
        return false;
    };
    let target = target.trim();
    if target.is_empty() {
        return false;
    }
    let cur = refno.to_string().replace('/', "_");
    cur == target
}

fn agent_log(hypothesis_id: &str, location: &str, message: &str, data: serde_json::Value) {
    if std::env::var_os("AIOS_AGENT_DEBUG").is_none()
        && std::env::var_os("AIOS_AGENT_DEBUG_REFNO").is_none()
        && std::env::var_os("AIOS_AGENT_DEBUG_GEOM_REFNO").is_none()
        && std::env::var_os("AIOS_LOG_FILE").is_none()
    {
        return;
    }
    let payload = serde_json::json!({
        "sessionId": "debug-session",
        "runId": agent_run_id(),
        "hypothesisId": hypothesis_id,
        "location": location,
        "message": message,
        "data": data,
        "timestamp": agent_now_ms(),
    });
    let path = r"d:\work\plant-code\gen_model-dev\.cursor\debug.log";
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(f, "{}", payload.to_string());
    }
}
// #endregion

/// 求解axis的数值
pub fn resolve_axis_params(
    refno: RefnoEnum,
    scom: &ScomInfo,
    context: &CataContext,
) -> BTreeMap<i32, CateAxisParam> {
    let mut cache = ResolveEvalCache::default();
    resolve_axis_params_with_cache(refno, scom, context, &mut cache)
}

pub(crate) fn resolve_axis_params_with_cache(
    refno: RefnoEnum,
    scom: &ScomInfo,
    context: &CataContext,
    cache: &mut ResolveEvalCache,
) -> BTreeMap<i32, CateAxisParam> {
    if should_trace_resolve_detail(refno) {
        cache.axis_trace_enabled = true;
        cache.axis_trace_refno = Some(refno);
    }
    let mut map = BTreeMap::new();
    for i in 0..scom.axis_params.len() {
        let axis = resolve_axis_param_with_cache(&scom.axis_params[i], scom, context, cache);
        map.insert(scom.axis_param_numbers[i], axis);
    }
    if cache.axis_trace_enabled {
        let s = &cache.axis_trace;
        println!(
            "      [axis trace] refno={} calls={} cache_hit={} cycle_guard={} total={}ms core={}ms scalar={}ms parse_dir={}ms parse_ref={}ms p_ref_lookup={}ms p_ref_hit={} p_ref_miss={} ptpos_follow={}ms scalar_fast_hit={} scalar_ctx_hit={} scalar_fallback={}",
            refno,
            s.axis_calls,
            s.cache_hit,
            s.cycle_guard_hit,
            s.total_ms,
            s.resolve_axis_core_ms,
            s.scalar_eval_ms,
            s.parse_dir_ms,
            s.parse_ref_dir_ms,
            s.p_ref_lookup_ms,
            s.p_ref_hit,
            s.p_ref_miss,
            s.ptpos_follow_ms,
            s.scalar_fast_path_hit,
            s.scalar_ctx_fast_hit,
            s.scalar_fast_path_fallback
        );
        if !s.scalar_fallback_expr_hits.is_empty() {
            let mut items = s
                .scalar_fallback_expr_hits
                .iter()
                .map(|(k, v)| (k, *v))
                .collect::<Vec<_>>();
            items.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
            let top = items
                .into_iter()
                .take(8)
                .map(|(k, v)| format!("{k}:{v}"))
                .collect::<Vec<_>>()
                .join(", ");
            println!("      [axis trace expr_top] refno={} {}", refno, top);
        }
    }
    map
}

///求解几何体，允许出错的情况，出错的需要跳过
pub fn resolve_gms(
    des_refno: RefnoEnum,
    gmse_raw_paras: &[GmParam],
    jusl_param: &Option<PlinParam>,
    na_plin_param: &Option<PlinParam>,
    context: &CataContext,
    axis_param_map: &BTreeMap<i32, CateAxisParam>,
    gm_group: &str,
) -> Vec<CateGeoParam> {
    let mut cache = ResolveEvalCache::default();
    resolve_gms_with_cache(
        des_refno,
        gmse_raw_paras,
        jusl_param,
        na_plin_param,
        context,
        axis_param_map,
        gm_group,
        &mut cache,
    )
}

pub(crate) fn resolve_gms_with_cache(
    des_refno: RefnoEnum,
    gmse_raw_paras: &[GmParam],
    jusl_param: &Option<PlinParam>,
    na_plin_param: &Option<PlinParam>,
    context: &CataContext,
    axis_param_map: &BTreeMap<i32, CateAxisParam>,
    gm_group: &str,
    cache: &mut ResolveEvalCache,
) -> Vec<CateGeoParam> {
    // NOTE:
    // - 默认不按 TUFL 硬过滤（用于完整实体生成）。
    // - 若需要“管道视图”导出/显示，可通过环境变量显式开启过滤：AIOS_RESPECT_TUFL=1。
    let respect_tufl = std::env::var_os("AIOS_RESPECT_TUFL").is_some();

    let trace_detail = should_trace_resolve_detail(des_refno);
    let trace_topn = resolve_gm_trace_topn();
    let trace_threshold_ms = resolve_gm_trace_threshold_ms();
    let t_group_total = Instant::now();
    let mut filtered_empty_spro = 0usize;
    let mut filtered_tufl = 0usize;
    let mut fail_cnt = 0usize;
    let mut timing_samples: Vec<(u128, RefnoEnum, String, bool)> = Vec::new();
    let mut out = Vec::new();

    for g in gmse_raw_paras.iter() {
        // NOTE:
        // - g.visible_flag 目前来源于 GMSE 的 TUFL（“管道视图可见性”），不应作为“是否生成几何”的硬过滤条件；
        //   否则会导致某些元件（例如阀门）缺失本体几何（表现为“少一截”）。
        // - TUFL 的语义应留给上层“视图/过滤”逻辑使用，而不是在解析阶段直接丢弃几何。
        if g.gm_type == "SPRO" && g.verts.is_empty() {
            filtered_empty_spro += 1;
            continue;
        }

        // TUFL 过滤（管道视图语义）：开启时直接丢弃 TUFL=false 的几何
        if respect_tufl && !g.visible_flag {
            // #region agent log
            agent_log(
                "H_TUFL",
                "rs-core/src/expression/resolve.rs:resolve_gms",
                "filtered_by_tufl",
                serde_json::json!({
                    "design_refno": des_refno.to_string().replace('/', "_"),
                    "geom_refno": g.refno.to_string().replace('/', "_"),
                    "gm_type": g.gm_type,
                    "visible_flag": g.visible_flag,
                    "centre_line_flag": g.centre_line_flag,
                    "diameters_expr_len": g.diameters.len(),
                    "distances_expr_len": g.distances.len(),
                    "xyz_expr_len": g.xyz.len(),
                }),
            );
            // #endregion
            filtered_tufl += 1;
            continue;
        }

        let t_item = Instant::now();
        let r = resolve_paragon_gm_params_with_cache(
            des_refno,
            g,
            jusl_param,
            na_plin_param,
            context,
            axis_param_map,
            cache,
        );
        let elapsed = t_item.elapsed().as_millis();
        if trace_detail {
            timing_samples.push((elapsed, g.refno, g.gm_type.clone(), r.is_ok()));
        }
        match r {
            Ok(v) => out.push(v),
            Err(e) => {
                fail_cnt += 1;
                println!("{}", e);
            }
        }
    }

    if trace_detail && !timing_samples.is_empty() {
        timing_samples.sort_by(|a, b| b.0.cmp(&a.0));
        let topn = trace_topn.min(timing_samples.len());
        let slow_cnt = timing_samples
            .iter()
            .filter(|(elapsed, _, _, _)| *elapsed >= trace_threshold_ms)
            .count();
        println!(
            "      [resolve_gms trace] refno={} group={} in={} out={} fail={} filtered_spro={} filtered_tufl={} total={}ms slow_count={}/{}(threshold={}ms)",
            des_refno,
            gm_group,
            gmse_raw_paras.len(),
            out.len(),
            fail_cnt,
            filtered_empty_spro,
            filtered_tufl,
            t_group_total.elapsed().as_millis(),
            slow_cnt,
            timing_samples.len(),
            trace_threshold_ms
        );
        for (idx, (elapsed, gm_refno, gm_type, ok)) in timing_samples.iter().take(topn).enumerate()
        {
            println!(
                "        [resolve_gms slow #{:02}] {} ms | status={} | gm_refno={} | gm_type={}",
                idx + 1,
                elapsed,
                if *ok { "ok" } else { "fail" },
                gm_refno,
                gm_type
            );
        }
    }

    out
}

/// 解析gmes的参数
pub fn resolve_paragon_gm_params(
    des_refno: RefnoEnum,
    gm_param: &GmParam,
    jusl_param: &Option<PlinParam>,
    na_plin_param: &Option<PlinParam>,
    context: &CataContext,
    axis_param_map: &BTreeMap<i32, CateAxisParam>,
) -> anyhow::Result<CateGeoParam> {
    let mut cache = ResolveEvalCache::default();
    resolve_paragon_gm_params_with_cache(
        des_refno,
        gm_param,
        jusl_param,
        na_plin_param,
        context,
        axis_param_map,
        &mut cache,
    )
}

pub(crate) fn resolve_paragon_gm_params_with_cache(
    des_refno: RefnoEnum,
    gm_param: &GmParam,
    jusl_param: &Option<PlinParam>,
    na_plin_param: &Option<PlinParam>,
    context: &CataContext,
    axis_param_map: &BTreeMap<i32, CateAxisParam>,
    cache: &mut ResolveEvalCache,
) -> anyhow::Result<CateGeoParam> {
    let trace_detail = should_trace_resolve_detail(des_refno);
    let trace_threshold_ms = resolve_gm_trace_threshold_ms();
    let t_total = Instant::now();
    let t_parse = Instant::now();

    match resolve_gmse_params_with_cache(
        gm_param,
        jusl_param,
        na_plin_param,
        context,
        axis_param_map,
        cache,
    ) {
        Ok(gm_data) => {
            let parse_ms = t_parse.elapsed().as_millis();
            let t_convert = Instant::now();
            let convert_result = panic::catch_unwind(|| {
                resolve_to_cate_geo_params(&gm_data).expect("resolve geom failed")
            })
            .map_err(|_| anyhow::anyhow!("元件库求解失败."));
            let convert_ms = t_convert.elapsed().as_millis();
            let total_ms = t_total.elapsed().as_millis();
            if trace_detail && total_ms >= trace_threshold_ms {
                println!(
                    "        [resolve_gm trace] des_refno={} gm_refno={} gm_type={} parse={}ms convert={}ms total={}ms status={}",
                    des_refno,
                    gm_param.refno,
                    gm_param.gm_type,
                    parse_ms,
                    convert_ms,
                    total_ms,
                    if convert_result.is_ok() { "ok" } else { "fail" }
                );
            }
            convert_result
        }
        Err(e) => Err(anyhow::anyhow!(format!(
            "几何数据解析失败: {:?}, 原因：{}",
            des_refno.to_string(),
            &e
        ))),
    }
}

pub fn resolve_gmse_params(
    gm: &GmParam,
    jusl_param: &Option<PlinParam>,
    na_plin_param: &Option<PlinParam>,
    context: &CataContext,
    axis_param_map: &BTreeMap<i32, CateAxisParam>,
) -> anyhow::Result<GmseParamData> {
    let mut cache = ResolveEvalCache::default();
    resolve_gmse_params_with_cache(
        gm,
        jusl_param,
        na_plin_param,
        context,
        axis_param_map,
        &mut cache,
    )
}

pub(crate) fn resolve_gmse_params_with_cache(
    gm: &GmParam,
    jusl_param: &Option<PlinParam>,
    na_plin_param: &Option<PlinParam>,
    context: &CataContext,
    axis_param_map: &BTreeMap<i32, CateAxisParam>,
    cache: &mut ResolveEvalCache,
) -> anyhow::Result<GmseParamData> {
    let angle = context
        .get(DDANGLE_STR)
        .unwrap()
        .parse::<f32>()
        .unwrap_or(0.0)
        .to_radians();
    let radius = context
        .get(DDRADIUS_STR)
        .unwrap()
        .parse::<f32>()
        .unwrap_or(0.0);
    let height = context
        .get(DDHEIGHT_STR)
        .unwrap()
        .parse::<f32>()
        .unwrap_or(0.0);
    // dbg!(&gm.diameters);
    crate::debug_model_debug!(
        "🎯 开始求值 DIAMETERS: refno={}, type={}, count={}",
        gm.refno,
        gm.gm_type,
        gm.diameters.len()
    );
    let diameters: Vec<f32> = gm
        .diameters
        .iter()
        .enumerate()
        .map(|(i, exp)| {
            crate::debug_model_debug!("   DIAMETERS[{}]: {}", i, exp);
            crate::set_expr_debug_info!(context, gm.refno, &gm.gm_type, "DIAMETERS", i);
            let val = eval_str_to_f32_cached(exp, context, "DIST", cache);
            crate::debug_model_debug!("   DIAMETERS[{}] 求值结果: {}", i, val);
            crate::clear_expr_debug_info!(context);
            val
        })
        .collect();
    // dbg!(&diameters);

    crate::debug_model_debug!(
        "🎯 开始求值 DISTANCES: refno={}, type={}, count={}",
        gm.refno,
        gm.gm_type,
        gm.distances.len()
    );
    let distances = gm
        .distances
        .iter()
        .enumerate()
        .map(|(i, exp)| {
            crate::debug_model_debug!("   DISTANCES[{}]: {}", i, exp);
            crate::set_expr_debug_info!(context, gm.refno, &gm.gm_type, "DISTANCES", i);
            let val = eval_str_to_f32_cached(exp, context, "DIST", cache);
            crate::debug_model_debug!("   DISTANCES[{}] 求值结果: {}", i, val);
            crate::clear_expr_debug_info!(context);
            val
        })
        .collect();

    let distances_specified: Vec<bool> = gm
        .distances
        .iter()
        .map(|exp| !exp.trim().is_empty())
        .collect();

    let shears = gm
        .shears
        .iter()
        .enumerate()
        .map(|(i, exp)| {
            crate::set_expr_debug_info!(context, gm.refno, &gm.gm_type, "SHEARS", i);
            let val = eval_str_to_f32_cached(exp, context, "DIST", cache);
            crate::clear_expr_debug_info!(context);
            val
        })
        .collect();

    let mut verts = vec![];
    for (i, vert) in gm.verts.iter().enumerate() {
        crate::set_expr_debug_info!(context, gm.refno, &gm.gm_type, "VERTS_X", i);
        let f0 = eval_str_to_f32_cached(&vert[0], context, "DIST", cache);
        crate::set_expr_debug_info!(context, gm.refno, &gm.gm_type, "VERTS_Y", i);
        let f1 = eval_str_to_f32_cached(&vert[1], context, "DIST", cache);
        crate::set_expr_debug_info!(context, gm.refno, &gm.gm_type, "VERTS_Z", i);
        let f2 = eval_str_to_f32_cached(&vert[2], context, "DIST", cache);
        crate::clear_expr_debug_info!(context);
        {
            verts.push(Vec3::new(f0, f1, f2));
        }
    }

    crate::set_expr_debug_info!(context, gm.refno, &gm.gm_type, "PHEI");
    crate::debug_model_debug!("🎯 开始求值 PHEI: refno={}, type={}", gm.refno, gm.gm_type);
    crate::debug_model_debug!("   原始 PHEI 表达式: {}", gm.phei);
    let phei = eval_str_to_f32_cached(&gm.phei, context, "DIST", cache);
    let phei_specified = !gm.phei.trim().is_empty();
    crate::debug_model_debug!("   PHEI 求值结果: {}", phei);
    crate::clear_expr_debug_info!(context);

    crate::set_expr_debug_info!(context, gm.refno, &gm.gm_type, "OFFSET");
    let offset = eval_str_to_f32_cached(&gm.offset, context, "DIST", cache);
    crate::clear_expr_debug_info!(context);

    crate::set_expr_debug_info!(context, gm.refno, &gm.gm_type, "PANG");
    let pang = eval_str_to_f32_cached(&gm.pang, context, "DIST", cache);
    crate::clear_expr_debug_info!(context);

    crate::set_expr_debug_info!(context, gm.refno, &gm.gm_type, "PWID");
    let pwid = eval_str_to_f32_cached(&gm.pwid, context, "DIST", cache);
    crate::clear_expr_debug_info!(context);

    crate::set_expr_debug_info!(context, gm.refno, &gm.gm_type, "DRAD");
    let drad = eval_str_to_f32_cached(&gm.drad, context, "DIST", cache);
    crate::clear_expr_debug_info!(context);

    crate::set_expr_debug_info!(context, gm.refno, &gm.gm_type, "DWID");
    let dwid = eval_str_to_f32_cached(&gm.dwid, context, "DIST", cache);
    crate::clear_expr_debug_info!(context);

    let mut frads = gm
        .frads
        .iter()
        .enumerate()
        .map(|(i, exp)| {
            crate::set_expr_debug_info!(context, gm.refno, &gm.gm_type, "FRADS", i);
            let val = eval_str_to_f32_cached(exp, context, "DIST", cache);
            crate::clear_expr_debug_info!(context);
            val
        })
        .collect();

    crate::set_expr_debug_info!(context, gm.refno, &gm.gm_type, "PRAD");
    let prad = eval_str_to_f32_cached(&gm.prad, context, "DIST", cache);
    crate::clear_expr_debug_info!(context);

    let dxy = gm
        .dxy
        .iter()
        .enumerate()
        .try_fold::<_, _, anyhow::Result<_>>(vec![], |mut acc, (i, exp)| {
            crate::set_expr_debug_info!(context, gm.refno, &gm.gm_type, "DXY_X", i);
            let f0 = eval_str_to_f32_cached(&exp[0], context, "DIST", cache);
            crate::set_expr_debug_info!(context, gm.refno, &gm.gm_type, "DXY_Y", i);
            let f1 = eval_str_to_f32_cached(&exp[1], context, "DIST", cache);
            crate::clear_expr_debug_info!(context);
            acc.push(Vec2::new(f0, f1));
            Ok(acc)
        })?;

    let lengths = gm
        .lengths
        .iter()
        .enumerate()
        .map(|(i, exp)| {
            crate::set_expr_debug_info!(context, gm.refno, &gm.gm_type, "LENGTHS", i);
            let val = eval_str_to_f32_cached(exp, context, "DIST", cache);
            crate::clear_expr_debug_info!(context);
            val
        })
        .collect();

    crate::debug_model_debug!(
        "🎯 开始求值 XYZ: refno={}, type={}, count={}",
        gm.refno,
        gm.gm_type,
        gm.xyz.len()
    );
    let xyz = gm
        .xyz
        .iter()
        .enumerate()
        .map(|(i, exp)| {
            crate::debug_model_debug!("   XYZ[{}]: {}", i, exp);
            crate::set_expr_debug_info!(context, gm.refno, &gm.gm_type, "XYZ", i);
            let val = eval_str_to_f32_cached(exp, context, "DIST", cache);
            crate::debug_model_debug!("   XYZ[{}] 求值结果: {}", i, val);
            crate::clear_expr_debug_info!(context);
            val
        })
        .collect();

    // agent：抓取目标 LPYR/NLPY 的原始表达式与求值结果，定位是否存在 /2、单位换算等导致“少一截”
    if (gm.gm_type == "LPYR" || gm.gm_type == "NLPY") && agent_match_refno(gm.refno) {
        agent_log(
            "H8",
            "rs-core/expression/resolve.rs:resolve_gmse_params",
            "gm_expr_snapshot",
            serde_json::json!({
                "gm_refno": gm.refno.to_string().replace('/', "_"),
                "gm_type": gm.gm_type,
                "visible_flag": gm.visible_flag,
                "distances_raw": gm.distances,
                "distances_eval": distances,
                "xyz_raw": gm.xyz,
                "xyz_eval": xyz,
            }),
        );
    }

    let mut paxises: Vec<Option<CateAxisParam>> = Vec::new();
    for axis_str in gm.paxises.iter() {
        let mut axis = axis_str.trim();
        if axis.is_empty() {
            continue;
        }
        let p_axis = axis.starts_with("P");
        let p_axis_neg = axis.starts_with("-P");
        //针对P方向
        if p_axis || p_axis_neg {
            if p_axis_neg {
                axis = &axis[1..];
            }
            if let Ok(index) = axis[1..].parse::<i32>() {
                if axis_param_map.contains_key(&index) {
                    paxises.push(Some(if p_axis_neg {
                        axis_param_map[&index].clone().neg()
                    } else {
                        axis_param_map[&index].clone()
                    }));
                } else {
                    paxises.push(None);
                    // dbg!(&gm);
                    #[cfg(feature = "debug")]
                    println!("Axis: '{axis_str}' index not exist");
                }
            }
        } else {
            let dir = parse_axis_to_vec3_cached(axis, context, cache).map(RsVec3);
            let axis = CateAxisParam {
                refno: Default::default(),
                number: 0,
                pt: Default::default(),
                dir,
                ..Default::default()
            };
            paxises.push(Some(axis));
        }
    }
    let mut plin_pos = Vec2::ZERO;
    let mut plin_axis = None;
    let mut plax = None;
    let mut na_axis = None;
    if let Some(jusl) = jusl_param {
        // 与 rs_surreal::spatial::query_pline 保持一致：
        // pt = vxy + dxy * plax（按分量缩放，2D 取 plax.xy）
        let vxy = Vec2::new(
            eval_str_to_f32_cached(&jusl.vxy[0], context, "DIST", cache),
            eval_str_to_f32_cached(&jusl.vxy[1], context, "DIST", cache),
        );
        let dxy = Vec2::new(
            eval_str_to_f32_cached(&jusl.dxy[0], context, "DIST", cache),
            eval_str_to_f32_cached(&jusl.dxy[1], context, "DIST", cache),
        );
        let plax_dir = parse_axis_to_vec3_cached(&jusl.plax, context, cache).unwrap_or(Vec3::Y);
        plin_pos = calc_plin_pos(vxy, dxy, plax_dir);
        plin_axis = Some(plax_dir);
    }
    if let Some(na_plin) = na_plin_param {
        if let Some(dir) = parse_axis_to_vec3_cached(&na_plin.plax, context, cache) {
            na_axis = Some(dir);
            // dbg!(na_axis);
        }
    }

    if let Some(p) = &gm.plax {
        if let Some(dir) = parse_axis_to_vec3_cached(p, context, cache) {
            plax = Some(dir);
            // dbg!(plax);
        }
    }
    let type_name = gm.gm_type.clone();
    Ok(GmseParamData {
        refno: gm.refno,
        type_name,
        radius,
        angle,
        height,
        pwid,
        prad,
        plin_pos,
        frads,
        pang,
        diameters,
        distances,
        distances_specified,
        shears,
        phei,
        phei_specified,
        offset,
        verts,
        dxy,
        drad,
        dwid,
        lengths,
        xyz,
        paxises,
        centre_line_flag: gm.centre_line_flag,
        tube_flag: gm.visible_flag,
        plin_axis,
        plax,
        na_axis,
    })
}

#[inline]
fn calc_plin_pos(vxy: Vec2, dxy: Vec2, plax: Vec3) -> Vec2 {
    vxy + dxy * Vec2::new(plax.x, plax.y)
}

pub(crate) fn resolve_plin_points_with_cache(
    scom: &ScomInfo,
    context: &CataContext,
    cache: &mut ResolveEvalCache,
) -> Vec<ResolvedPlinePoint> {
    let mut points = scom
        .plin_map
        .iter()
        .map(|(pkey, plin)| {
            let vxy = Vec2::new(
                eval_str_to_f32_cached(&plin.vxy[0], context, "DIST", cache),
                eval_str_to_f32_cached(&plin.vxy[1], context, "DIST", cache),
            );
            let dxy = Vec2::new(
                eval_str_to_f32_cached(&plin.dxy[0], context, "DIST", cache),
                eval_str_to_f32_cached(&plin.dxy[1], context, "DIST", cache),
            );
            let plax = parse_axis_to_vec3_cached(&plin.plax, context, cache).unwrap_or(Vec3::Y);
            ResolvedPlinePoint {
                pkey: pkey.clone(),
                position: calc_plin_pos(vxy, dxy, plax),
            }
        })
        .collect::<Vec<_>>();
    points.sort_by(|a, b| a.pkey.cmp(&b.pkey));
    points
}

pub fn resolve_axis_param(
    axis_param: &AxisParam,
    scom: &ScomInfo,
    context: &CataContext,
) -> CateAxisParam {
    let mut cache = ResolveEvalCache::default();
    resolve_axis_param_with_cache(axis_param, scom, context, &mut cache)
}

pub(crate) fn resolve_axis_param_with_cache(
    axis_param: &AxisParam,
    scom: &ScomInfo,
    context: &CataContext,
    cache: &mut ResolveEvalCache,
) -> CateAxisParam {
    let t_axis_total = Instant::now();
    let number = axis_param.number;
    if number != 0 {
        if let Some(axis) = cache.axis_params.get(&number) {
            if cache.axis_trace_enabled {
                cache.axis_trace.cache_hit += 1;
            }
            return axis.clone();
        }
        if !cache.axis_resolving.insert(number) {
            if cache.axis_trace_enabled {
                cache.axis_trace.cycle_guard_hit += 1;
            }
            return Default::default();
        }
    }

    let key: String = axis_param
        .pconnect
        .replace("\n", "")
        .replace(" ", "")
        .into();
    let pconnect = if context.contains_key(&key) {
        let tmp = context.get(&key).unwrap().parse::<u32>().unwrap_or(0u32);
        db1_dehash(tmp)
    } else {
        key.clone()
    };
    let t_scalar = Instant::now();
    let pbore = eval_str_to_f32_cached(&axis_param.pbore, context, "DIST", cache);
    let pwidth = eval_str_to_f32_cached(&axis_param.pwidth, context, "DIST", cache);
    let pheight = eval_str_to_f32_cached(&axis_param.pheight, context, "DIST", cache);
    if cache.axis_trace_enabled {
        cache.axis_trace.scalar_eval_ms += t_scalar.elapsed().as_millis();
    }
    let t_core = Instant::now();
    let result = if let Ok((m_dir, ref_dir, pos)) =
        resolve_axis_with_cache(axis_param, scom, context, cache)
    {
        if cache.axis_trace_enabled {
            cache.axis_trace.resolve_axis_core_ms += t_core.elapsed().as_millis();
        }
        let mut dir = m_dir.is_normalized().then(|| RsVec3(m_dir));
        let ref_dir = ref_dir.is_normalized().then(|| RsVec3(ref_dir));
        match axis_param.type_name.as_str() {
            "PTAX" => {
                let t_scalar = Instant::now();
                let d = eval_str_to_f32_cached(&axis_param.distance, context, "DIST", cache);
                if cache.axis_trace_enabled {
                    cache.axis_trace.scalar_eval_ms += t_scalar.elapsed().as_millis();
                }
                CateAxisParam {
                    refno: axis_param.refno,
                    number,
                    pt: RsVec3(d * m_dir + pos),
                    dir,
                    ref_dir,
                    pconnect,
                    pbore,
                    pwidth,
                    pheight,
                    ..Default::default()
                }
            }
            "PTCA" | "PTMI" => {
                let t_scalar = Instant::now();
                let x = eval_str_to_f32_cached(&axis_param.x, context, "DIST", cache);
                let y = eval_str_to_f32_cached(&axis_param.y, context, "DIST", cache);
                let z = eval_str_to_f32_cached(&axis_param.z, context, "DIST", cache);
                if cache.axis_trace_enabled {
                    cache.axis_trace.scalar_eval_ms += t_scalar.elapsed().as_millis();
                }
                if dir.is_none() {
                    let dirs = axis_param.direction.split(" ").collect::<Vec<_>>();
                    if !dirs.is_empty() {
                        dir = parse_axis_to_vec3_cached(dirs[0], context, cache).map(RsVec3);
                    }
                }
                CateAxisParam {
                    refno: axis_param.refno,
                    number,
                    pt: RsVec3(pos + Vec3::new(x, y, z)),
                    dir,
                    ref_dir,
                    pconnect,
                    pbore,
                    pwidth,
                    pheight,
                    ..Default::default()
                }
            }
            "PTPOS" => {
                let mut cate_axis = CateAxisParam {
                    number,
                    dir,
                    ref_dir,
                    pconnect,
                    pbore,
                    pwidth,
                    pheight,
                    ..Default::default()
                };
                if let Some(pnt_index_str) = axis_param.pnt_index_str.as_ref() {
                    let paras = pnt_index_str
                        .split_whitespace()
                        .map(|x| x.trim().to_owned())
                        .collect::<Vec<_>>();
                    if paras.len() == 2 {
                        let pnt_index = paras[1].parse::<i32>().unwrap_or(i32::MAX);
                        if let Some(indx) =
                            scom.axis_param_numbers.iter().position(|&x| x == pnt_index)
                        {
                            let t_follow = Instant::now();
                            let axis = resolve_axis_param_with_cache(
                                &scom.axis_params[indx],
                                scom,
                                context,
                                cache,
                            );
                            if cache.axis_trace_enabled {
                                cache.axis_trace.ptpos_follow_ms += t_follow.elapsed().as_millis();
                            }
                            cate_axis.refno = axis_param.refno;
                            cate_axis.pt = axis.pt;
                        }
                    }
                }
                cate_axis
            }
            _ => CateAxisParam::default(),
        }
    } else {
        if cache.axis_trace_enabled {
            cache.axis_trace.resolve_axis_core_ms += t_core.elapsed().as_millis();
        }
        Default::default()
    };

    if number != 0 {
        cache.axis_resolving.remove(&number);
        cache.axis_params.insert(number, result.clone());
    }
    if cache.axis_trace_enabled {
        cache.axis_trace.axis_calls += 1;
        cache.axis_trace.total_ms += t_axis_total.elapsed().as_millis();
    }

    result
}

#[inline]
pub fn parse_to_u16(input: &[u8]) -> u16 {
    u16::from_be_bytes(input.try_into().unwrap())
}

#[inline]
pub fn parse_to_i16(input: &[u8]) -> i16 {
    i16::from_be_bytes(input.try_into().unwrap())
}

#[inline]
pub fn parse_to_i32(input: &[u8]) -> i32 {
    i32::from_be_bytes(input.try_into().unwrap())
}

#[inline]
pub fn parse_to_u32(input: &[u8]) -> u32 {
    u32::from_be_bytes(input.try_into().unwrap())
}

#[inline]
pub fn parse_to_u64(input: &[u8]) -> u64 {
    u64::from_be_bytes(input.try_into().unwrap())
}

#[inline]
pub fn parse_to_i64(input: &[u8]) -> i64 {
    i64::from_be_bytes(input.try_into().unwrap())
}

#[inline]
pub fn parse_to_f32(input: &[u8]) -> f32 {
    (f32::from_be_bytes(input.try_into().unwrap()) * 100.0).round() / 100.0
}

#[inline]
pub fn parse_to_f64(input: &[u8]) -> f64 {
    return if let [a, b, c, d, e, f, g, h] = input[..8] {
        (f64::from_be_bytes([e, f, g, h, a, b, c, d]) * 100.0).round() / 100.0
    } else {
        0.0
    };
}

#[inline]
pub fn convert_u32_to_noun(input: &[u8]) -> String {
    db1_dehash(parse_to_u32(input.try_into().unwrap())).into()
}

#[inline]
pub fn parse_to_f64_arr(input: &[u8]) -> [f64; 3] {
    let mut data = [0f64; 3];
    for i in 0..3 {
        data[i] = parse_to_f64(&input[i * 8..i * 8 + 8]);
    }
    data
}

#[inline]
pub fn parse_to_f32_arr(input: &[u8]) -> [f64; 3] {
    let mut data = [0f64; 3];
    for i in 0..3 {
        data[i] = parse_to_f32(&input[i * 4..i * 4 + 4]) as f64;
    }
    data
}

#[cfg(test)]
mod tests {
    use super::{ResolveEvalCache, calc_plin_pos, resolve_plin_points_with_cache};
    use crate::CataContext;
    use crate::pdms_data::{PlinParam, ScomInfo};
    use glam::{Vec2, Vec3};

    #[test]
    fn test_calc_plin_pos_when_plax_is_y() {
        let vxy = Vec2::new(10.0, 20.0);
        let dxy = Vec2::new(3.0, 4.0);
        let plax = Vec3::Y;
        let p = calc_plin_pos(vxy, dxy, plax);
        assert!((p.x - 10.0).abs() < 1e-6, "x={}", p.x);
        assert!((p.y - 24.0).abs() < 1e-6, "y={}", p.y);
    }

    #[test]
    fn test_calc_plin_pos_when_plax_is_x() {
        let vxy = Vec2::new(10.0, 20.0);
        let dxy = Vec2::new(3.0, 4.0);
        let plax = Vec3::X;
        let p = calc_plin_pos(vxy, dxy, plax);
        assert!((p.x - 13.0).abs() < 1e-6, "x={}", p.x);
        assert!((p.y - 20.0).abs() < 1e-6, "y={}", p.y);
    }

    #[test]
    fn test_calc_plin_pos_when_plax_has_mixed_sign() {
        let vxy = Vec2::new(1.0, 2.0);
        let dxy = Vec2::new(5.0, 7.0);
        let plax = Vec3::new(0.5, -1.0, 0.0);
        let p = calc_plin_pos(vxy, dxy, plax);
        assert!((p.x - 3.5).abs() < 1e-6, "x={}", p.x);
        assert!((p.y + 5.0).abs() < 1e-6, "y={}", p.y);
    }

    #[test]
    fn resolves_all_plines_in_stable_pkey_order() {
        let mut scom = ScomInfo::default();
        scom.plin_map.insert(
            "TOS".into(),
            PlinParam {
                vxy: ["10".into(), "20".into()],
                dxy: ["3".into(), "4".into()],
                plax: "Y".into(),
            },
        );
        scom.plin_map.insert(
            "BOS".into(),
            PlinParam {
                vxy: ["1".into(), "2".into()],
                dxy: ["5".into(), "7".into()],
                plax: "X".into(),
            },
        );

        let points = resolve_plin_points_with_cache(
            &scom,
            &CataContext::default(),
            &mut ResolveEvalCache::default(),
        );
        assert_eq!(
            points.iter().map(|p| p.pkey.as_str()).collect::<Vec<_>>(),
            ["BOS", "TOS"]
        );
        assert_eq!(points[0].position, Vec2::new(6.0, 2.0));
        assert_eq!(points[1].position, Vec2::new(10.0, 24.0));
    }
}
