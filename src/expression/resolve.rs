use std::collections::BTreeMap;
use std::ops::Neg;
use std::panic;

use crate::expression::resolve_helper::{
    parse_str_axis_to_vec3, resolve_axis, resolve_to_cate_geo_params,
};
use crate::parsed_data::geo_params_data::CateGeoParam;
use crate::parsed_data::{CateAxisParam, GmseParamData};
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
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
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
    let mut map = BTreeMap::new();
    for i in 0..scom.axis_params.len() {
        let axis = resolve_axis_param(&scom.axis_params[i], scom, context);
        map.insert(scom.axis_param_numbers[i], axis);
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
) -> Vec<CateGeoParam> {
    // NOTE:
    // - 默认不按 TUFL 硬过滤（用于完整实体生成）。
    // - 若需要“管道视图”导出/显示，可通过环境变量显式开启过滤：AIOS_RESPECT_TUFL=1。
    let respect_tufl = std::env::var_os("AIOS_RESPECT_TUFL").is_some();

    gmse_raw_paras
        .iter()
        .cloned()
        .filter_map(|g| {
            // NOTE:
            // - g.visible_flag 目前来源于 GMSE 的 TUFL（“管道视图可见性”），不应作为“是否生成几何”的硬过滤条件；
            //   否则会导致某些元件（例如阀门）缺失本体几何（表现为“少一截”）。
            // - TUFL 的语义应留给上层“视图/过滤”逻辑使用，而不是在解析阶段直接丢弃几何。
            if g.gm_type == "SPRO" && g.verts.is_empty() {
                return None;
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
                return None;
            }

            let r = resolve_paragon_gm_params(
                des_refno,
                &g,
                jusl_param,
                na_plin_param,
                context,
                axis_param_map,
            );
            match r {
                Ok(v) => Some(v),
                Err(e) => {
                    println!("{}", e);
                    None
                }
            }
        })
        .collect::<_>()
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
    match resolve_gmse_params(gm_param, jusl_param, na_plin_param, context, axis_param_map) {
        Ok(gm_data) => panic::catch_unwind(|| {
            resolve_to_cate_geo_params(&gm_data).expect("resolve geom failed")
        })
        .map_err(|e| anyhow::anyhow!("元件库求解失败.")),
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
            let val = eval_str_to_f32_or_default(exp, context, "DIST");
            crate::debug_model_debug!("   DIAMETERS[{}] 求值结果: {}", i, val);
            crate::clear_expr_debug_info!(context);
            val
        })
        .collect();
    // dbg!(&diameters);

    let distances = gm
        .distances
        .iter()
        .enumerate()
        .map(|(i, exp)| {
            crate::set_expr_debug_info!(context, gm.refno, &gm.gm_type, "DISTANCES", i);
            let val = eval_str_to_f32_or_default(exp, context, "DIST");
            crate::clear_expr_debug_info!(context);
            val
        })
        .collect();

    let shears = gm
        .shears
        .iter()
        .enumerate()
        .map(|(i, exp)| {
            crate::set_expr_debug_info!(context, gm.refno, &gm.gm_type, "SHEARS", i);
            let val = eval_str_to_f32_or_default(exp, context, "DIST");
            crate::clear_expr_debug_info!(context);
            val
        })
        .collect();

    let mut verts = vec![];
    for (i, vert) in gm.verts.iter().enumerate() {
        crate::set_expr_debug_info!(context, gm.refno, &gm.gm_type, "VERTS_X", i);
        let f0 = eval_str_to_f32_or_default(&vert[0], context, "DIST");
        crate::set_expr_debug_info!(context, gm.refno, &gm.gm_type, "VERTS_Y", i);
        let f1 = eval_str_to_f32_or_default(&vert[1], context, "DIST");
        crate::set_expr_debug_info!(context, gm.refno, &gm.gm_type, "VERTS_Z", i);
        let f2 = eval_str_to_f32_or_default(&vert[2].as_str(), context, "DIST");
        crate::clear_expr_debug_info!(context);
        {
            verts.push(Vec3::new(f0, f1, f2));
        }
    }

    crate::set_expr_debug_info!(context, gm.refno, &gm.gm_type, "PHEI");
    crate::debug_model_debug!("🎯 开始求值 PHEI: refno={}, type={}", gm.refno, gm.gm_type);
    crate::debug_model_debug!("   原始 PHEI 表达式: {}", gm.phei);
    let phei = eval_str_to_f32_or_default(&gm.phei, context, "DIST");
    crate::debug_model_debug!("   PHEI 求值结果: {}", phei);
    crate::clear_expr_debug_info!(context);

    crate::set_expr_debug_info!(context, gm.refno, &gm.gm_type, "OFFSET");
    let offset = eval_str_to_f32_or_default(&gm.offset, context, "DIST");
    crate::clear_expr_debug_info!(context);

    crate::set_expr_debug_info!(context, gm.refno, &gm.gm_type, "PANG");
    let pang = eval_str_to_f32_or_default(&gm.pang, context, "DIST");
    crate::clear_expr_debug_info!(context);

    crate::set_expr_debug_info!(context, gm.refno, &gm.gm_type, "PWID");
    let pwid = eval_str_to_f32_or_default(&gm.pwid, context, "DIST");
    crate::clear_expr_debug_info!(context);

    crate::set_expr_debug_info!(context, gm.refno, &gm.gm_type, "DRAD");
    let drad = eval_str_to_f32_or_default(&gm.drad, context, "DIST");
    crate::clear_expr_debug_info!(context);

    crate::set_expr_debug_info!(context, gm.refno, &gm.gm_type, "DWID");
    let dwid = eval_str_to_f32_or_default(&gm.dwid, context, "DIST");
    crate::clear_expr_debug_info!(context);

    let mut frads = gm
        .frads
        .iter()
        .enumerate()
        .map(|(i, exp)| {
            crate::set_expr_debug_info!(context, gm.refno, &gm.gm_type, "FRADS", i);
            let val = eval_str_to_f32_or_default(exp, context, "DIST");
            crate::clear_expr_debug_info!(context);
            val
        })
        .collect();

    crate::set_expr_debug_info!(context, gm.refno, &gm.gm_type, "PRAD");
    let prad = eval_str_to_f32_or_default(&gm.prad, context, "DIST");
    crate::clear_expr_debug_info!(context);

    let dxy = gm
        .dxy
        .iter()
        .enumerate()
        .try_fold::<_, _, anyhow::Result<_>>(vec![], |mut acc, (i, exp)| {
            crate::set_expr_debug_info!(context, gm.refno, &gm.gm_type, "DXY_X", i);
            let f0 = eval_str_to_f32_or_default(&exp[0], context, "DIST");
            crate::set_expr_debug_info!(context, gm.refno, &gm.gm_type, "DXY_Y", i);
            let f1 = eval_str_to_f32_or_default(&exp[1], context, "DIST");
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
            let val = eval_str_to_f32_or_default(exp, context, "DIST");
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
            let val = eval_str_to_f32_or_default(exp, context, "DIST");
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
            let dir = parse_str_axis_to_vec3(axis, context).ok().map(RsVec3);
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
        // dbg!(jusl);
        //直接把 jusl_dxy加上
        plin_pos = Vec2::new(
            eval_str_to_f32_or_default(&jusl.vxy[0], context, "DIST"),
            eval_str_to_f32_or_default(&jusl.vxy[1], context, "DIST"),
        ) + Vec2::new(
            eval_str_to_f32_or_default(&jusl.dxy[0], context, "DIST"),
            eval_str_to_f32_or_default(&jusl.dxy[1], context, "DIST"),
        );

        if let Ok(dir) = parse_str_axis_to_vec3(&jusl.plax, context) {
            plin_axis = Some(dir);
            // dbg!(plin_axis);
        }
    }
    if let Some(na_plin) = na_plin_param {
        if let Ok(dir) = parse_str_axis_to_vec3(&na_plin.plax, context) {
            na_axis = Some(dir);
            // dbg!(na_axis);
        }
    }

    if let Some(p) = &gm.plax {
        if let Ok(dir) = parse_str_axis_to_vec3(p, context) {
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
        shears,
        phei,
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

pub fn resolve_axis_param(
    axis_param: &AxisParam,
    scom: &ScomInfo,
    context: &CataContext,
) -> CateAxisParam {
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
    let number = axis_param.number;
    let pbore = eval_str_to_f32_or_default(&axis_param.pbore, &context, "DIST");
    let pwidth = eval_str_to_f32_or_default(&axis_param.pwidth, &context, "DIST");
    let pheight = eval_str_to_f32_or_default(&axis_param.pheight, &context, "DIST");
    let Ok((m_dir, ref_dir, pos)) = resolve_axis(axis_param, scom, context) else {
        return Default::default();
    };
    let mut dir = m_dir.is_normalized().then(|| RsVec3(m_dir));
    let ref_dir = ref_dir.is_normalized().then(|| RsVec3(ref_dir));
    // dbg!(&axis_param);
    let result = match axis_param.type_name.as_str() {
        "PTAX" => {
            let d = eval_str_to_f32_or_default(&axis_param.distance, &context, "DIST");
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
            let x = eval_str_to_f32_or_default(&axis_param.x, &context, "DIST");
            let y = eval_str_to_f32_or_default(&axis_param.y, &context, "DIST");
            let z = eval_str_to_f32_or_default(&axis_param.z, &context, "DIST");
            if dir.is_none() {
                // dbg!(&axis_param);
                let dirs = axis_param.direction.split(" ").collect::<Vec<_>>();
                if !dirs.is_empty() {
                    dir = parse_str_axis_to_vec3(&dirs[0], &context).ok().map(RsVec3);
                    // dbg!(dir);
                }
                // dbg!(dirs);
                // dbg!(dirs);
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
                    if let Some(indx) = scom.axis_param_numbers.iter().position(|&x| x == pnt_index)
                    {
                        let axis = resolve_axis_param(&scom.axis_params[indx], scom, context);
                        cate_axis.refno = axis_param.refno;
                        cate_axis.pt = axis.pt;
                    }
                }
            }
            return cate_axis;
        }
        _ => CateAxisParam::default(),
    };

    // dbg!(&result);

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
