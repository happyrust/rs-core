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
    gmse_raw_paras
        .iter()
        .cloned()
        .filter_map(|g| {
            if g.visible_flag {
                if g.gm_type == "SPRO" && g.verts.is_empty() {
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
                return match r {
                    Ok(v) => Some(v),
                    Err(e) => {
                        // dbg!(g);
                        println!("{}", e);
                        None
                    }
                };
            } else {
                None
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
    let diameters: Vec<f32> = gm
        .diameters
        .iter()
        .map(|exp| {
            let val = eval_str_to_f32_or_default(exp, context, "DIST");

            // 🔍 如果表达式包含 PARAM 2，打印详细信息
            if exp.contains("PARAM 2") || exp.contains("PARAM2") {
                println!(
                    "🔍 [diameter 使用 PARAM 2] refno={:?}, gm_type='{}', expr='{}', value={}",
                    gm.refno, gm.gm_type, exp, val
                );
            } else {
                // 🔍 调试输出：打印表达式和计算结果
                println!(
                    "🔍 [diameter] refno={:?}, expr='{}', value={}",
                    gm.refno, exp, val
                );
            }

            // 如果值异常大，打印 context 中的 PARAM 值
            if val > 10000.0 && exp.contains("PARAM") {
                println!("   ⚠️  异常大的 diameter 值！打印 context 中的 PARAM:");
                for entry in context.context.iter() {
                    let key = entry.key();
                    if key.contains("PARAM") {
                        println!("      {} = {}", key, entry.value());
                    }
                }
            }
            val
        })
        .collect();
    // dbg!(&diameters);

    let distances = gm
        .distances
        .iter()
        .map(|exp| eval_str_to_f32_or_default(exp, context, "DIST"))
        .collect();

    let shears = gm
        .shears
        .iter()
        .map(|exp| eval_str_to_f32_or_default(exp, context, "DIST"))
        .collect();

    let mut verts = vec![];
    for vert in &gm.verts {
        let f0 = eval_str_to_f32_or_default(&vert[0], context, "DIST");
        let f1 = eval_str_to_f32_or_default(&vert[1], context, "DIST");
        let f2 = eval_str_to_f32_or_default(&vert[2].as_str(), context, "DIST");
        {
            verts.push(Vec3::new(f0, f1, f2));
        }
    }

    let phei = eval_str_to_f32_or_default(&gm.phei, context, "DIST");

    // 🔍 如果表达式包含 PARAM 2，打印详细信息
    if gm.phei.contains("PARAM 2") || gm.phei.contains("PARAM2") {
        println!(
            "🔍 [phei 使用 PARAM 2] refno={:?}, gm_type='{}', expr='{}', value={}",
            gm.refno, gm.gm_type, gm.phei, phei
        );
    } else {
        // 🔍 调试输出：打印 phei 表达式和计算结果
        println!(
            "🔍 [phei] refno={:?}, expr='{}', value={}",
            gm.refno, gm.phei, phei
        );
    }
    let offset = eval_str_to_f32_or_default(&gm.offset, context, "DIST");

    let pang = eval_str_to_f32_or_default(&gm.pang, context, "DIST");
    let pwid = eval_str_to_f32_or_default(&gm.pwid, context, "DIST");
    let drad = eval_str_to_f32_or_default(&gm.drad, context, "DIST");
    let dwid = eval_str_to_f32_or_default(&gm.dwid, context, "DIST");

    let mut frads = gm
        .frads
        .iter()
        .map(|exp| eval_str_to_f32_or_default(exp, context, "DIST"))
        .collect();

    let prad = eval_str_to_f32_or_default(&gm.prad, context, "DIST");

    let dxy = gm
        .dxy
        .iter()
        .try_fold::<_, _, anyhow::Result<_>>(vec![], |mut acc, exp| {
            let f0 = eval_str_to_f32_or_default(&exp[0], context, "DIST");
            let f1 = eval_str_to_f32_or_default(&exp[1], context, "DIST");
            acc.push(Vec2::new(f0, f1));
            Ok(acc)
        })?;

    let lengths = gm
        .lengths
        .iter()
        .map(|exp| eval_str_to_f32_or_default(exp, context, "DIST"))
        .collect();

    let xyz = gm
        .xyz
        .iter()
        .map(|exp| eval_str_to_f32_or_default(exp, context, "DIST"))
        .collect();

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
