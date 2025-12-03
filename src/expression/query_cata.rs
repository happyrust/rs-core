use std::collections::BTreeMap;

use crate::expression::resolve::{resolve_axis_params, resolve_gms};
use crate::parsed_data::CateGeomsInfo;
use crate::pdms_data::{AxisParam, GmParam, ScomInfo};
use crate::pdms_types::*;
use crate::{AttrMap, CataContext, NamedAttrValue};

///查询 Axis 参数
pub async fn query_axis_params(refno: RefnoEnum) -> anyhow::Result<BTreeMap<i32, AxisParam>> {
    // 查找ptse
    let mut map = BTreeMap::new();
    // dbg!(refno);
    let children = crate::get_children_named_attmaps(refno).await?;

    for child in children {
        //plin不在收集范围
        if child.get_type_str() == "PLIN" {
            continue;
        }
        // dbg!(&child);
        let number = child.get_i32("NUMB").unwrap_or(-1);
        if let Some(axis) = get_axis_param(&child) {
            map.entry(number).or_insert(axis);
        }
    }
    Ok(map)
}

///对元件库的SCOM Element进行求值计算
pub fn resolve_cata_comp(
    des_att: &NamedAttrMap,
    scom_info: &ScomInfo,
    context: Option<CataContext>,
) -> anyhow::Result<CateGeomsInfo> {
    let des_refno = des_att.get_refno().unwrap_or_default();
    let mut cur_context = context.unwrap_or_default();
    let cat_ref = scom_info.attr_map.get_refno().unwrap_or_default();

    let axis_param_map = resolve_axis_params(des_refno, scom_info, &cur_context);
    let jusl_param = if let Some(plin) = cur_context.get("JUSL") {
        if scom_info.plin_map.contains_key(plin.as_str()) {
            Some(scom_info.plin_map.get(plin.as_str()).unwrap().clone())
        } else {
            None
        }
    } else {
        None
    };

    let na_plin_param = if scom_info.plin_map.contains_key("NA") {
        Some(scom_info.plin_map.get("NA").unwrap().clone())
    } else {
        None
    };

    let geometries = resolve_gms(
        des_refno,
        &scom_info.gm_params,
        &jusl_param,
        &na_plin_param,
        &cur_context,
        &axis_param_map,
    );
    // dbg!((des_refno, &geometries));

    let n_geometries = resolve_gms(
        des_refno,
        &scom_info.ngm_params,
        &jusl_param,
        &na_plin_param,
        &cur_context,
        &axis_param_map,
    );

    Ok(CateGeomsInfo {
        refno: cat_ref,
        geometries,
        n_geometries,
        axis_map: axis_param_map,
    })
}

///获得AxisParam
pub fn get_axis_param(attr_map: &NamedAttrMap) -> Option<AxisParam> {
    let type_name = attr_map.get_as_string("TYPE").unwrap_or_default();
    let pconnect = attr_map.get_as_string("PCON").unwrap_or_default();
    let pbore = attr_map.get_as_string("PBOR").unwrap_or_default();
    let pwidth = attr_map.get_as_string("PWID").unwrap_or_default();
    let pheight = attr_map.get_as_string("PHEI").unwrap_or_default();
    let refno = attr_map.get_refno()?;
    let number = attr_map.get_i32("NUMB").unwrap_or_default();
    let r = match type_name.as_ref() {
        "PTAX" => AxisParam {
            refno,
            type_name,
            number,
            x: "".into(),
            y: "".into(),
            z: "".into(),
            distance: attr_map.get_as_string("PDIS")?,
            direction: attr_map.get_as_string("PAXI")?,
            ref_direction: attr_map.get_as_string("PZAXI").unwrap_or_default(),
            pconnect,
            pbore,
            pwidth,
            pheight,
            pnt_index_str: None,
        },
        "PTCA" => AxisParam {
            refno,
            type_name,
            number,
            x: attr_map.get_as_string("PX")?,
            y: attr_map.get_as_string("PY")?,
            z: attr_map.get_as_string("PZ")?,
            distance: "".into(),
            direction: { attr_map.get_as_string("PTCD").unwrap_or("Y".into()) },
            ref_direction: attr_map.get_as_string("PZAXI").unwrap_or_default(),
            pconnect,
            pbore,
            pwidth,
            pheight,
            pnt_index_str: None,
        },
        "PTMI" => AxisParam {
            refno,
            type_name,
            number,
            x: attr_map.get_as_string("PX")?,
            y: attr_map.get_as_string("PY")?,
            z: attr_map.get_as_string("PZ")?,
            distance: "".into(),
            direction: attr_map.get_as_string("PAXI")?,
            ref_direction: attr_map.get_as_string("PZAXI").unwrap_or_default(),
            pconnect,
            pbore,
            pwidth,
            pheight,
            pnt_index_str: None,
        },
        "PTPOS" => {
            AxisParam {
                //todo need fix " TPOS OF CREF"   " TDIR OF CREF"
                refno,
                type_name,
                number,
                x: "".into(),
                y: "".into(),
                z: "".into(),
                distance: attr_map.get_as_string("PTCP").unwrap_or("0".into()),
                direction: attr_map.get_as_string("PTCD").unwrap_or("Y".into()),
                ref_direction: attr_map.get_as_string("PZAXI").unwrap_or_default(),
                pconnect,
                pbore,
                pwidth,
                pheight,
                pnt_index_str: attr_map.get_as_string("PTCPOS"),
            }
        }
        _ => AxisParam {
            refno,
            type_name,
            number,
            x: "".into(),
            y: "".into(),
            z: "".into(),
            distance: "".into(),
            direction: "".into(),
            ref_direction: "".into(),
            pconnect,
            pbore,
            pwidth,
            pheight,
            pnt_index_str: None,
        },
    };
    Some(r)
}

///获得gmse的params
pub async fn query_gm_param(att: &NamedAttrMap, is_spro: bool) -> Option<GmParam> {
    // dbg!(a);
    let mut paxises = att.get_attr_strings_without_default(&["PAXI", "PAAX", "PBAX", "PCAX"]);
    if let Some(val) = att.get_val("PTS") {
        match val {
            NamedAttrValue::IntArrayType(v) => {
                for s in v {
                    paxises.push(s.to_string().into());
                }
            }
            _ => {}
        }
    }
    if let Some(v) = att.get_as_string("PLAX") {
        paxises.push((v));
    }
    let centre_line_flag = att.get_bool("CLFL").unwrap_or(false);
    // TUFL 控制几何体在管道视图中的可见性，默认为 true（可见）
    let tube_flag = att.get_bool("TUFL").unwrap_or(true);
    let mut verts = vec![];
    let mut frads = vec![];
    let mut dxy = vec![];
    let refno = att.get_refno().unwrap_or_default();
    let type_name = att.get_type_str();

    // 🔍 调试：记录从数据库读取的几何体信息
    crate::debug_model_debug!("📦 query_gm_param: 几何体 {} ({})", refno, type_name);
    if type_name == "SEXT" || type_name == "NSEX" || type_name == "SREV" || type_name == "NSRE" {
        //先暂时不考虑负实体
        let children = crate::get_children_named_attmaps(refno).await.ok()?;
        for child in children {
            if let Some(r) = child.get_refno()
                && child.get_type_str() == "SLOO"
            {
                let vert_atts = crate::get_children_named_attmaps(r)
                    .await
                    .unwrap_or_default();
                // dbg!(&vert_atts);
                for a in vert_atts {
                    verts.push([
                        (a.get_as_string("PX").unwrap_or_default()),
                        (a.get_as_string("PY").unwrap_or_default()),
                        (a.get_as_string("PZ").unwrap_or_default()),
                    ]);
                    frads.push((a.get_as_string("PRAD").unwrap_or_default()));
                }
            }
        }
    } else {
        let cur_type = crate::get_type_name(refno).await.unwrap_or_default();
        if is_spro && cur_type.as_str() == "SPRO" {
            for a in crate::get_children_named_attmaps(refno)
                .await
                .ok()
                .unwrap_or_default()
            {
                verts.push([
                    (a.get_as_string("PX").unwrap_or_default()),
                    (a.get_as_string("PY").unwrap_or_default()),
                    (a.get_as_string("PZ").unwrap_or_default()),
                ]);
                frads.push((a.get_as_string("PRAD").unwrap_or_default()));
                dxy.push([
                    (a.get_as_string("DX").unwrap_or_default()),
                    (a.get_as_string("DY").unwrap_or_default()),
                ]);
            }
        } else {
            verts.push([
                (att.get_as_string("PX").unwrap_or_default()),
                (att.get_as_string("PY").unwrap_or_default()),
                (att.get_as_string("PZ").unwrap_or_default()),
            ]);
            frads.push((att.get_as_string("PRAD").unwrap_or_default()));
            dxy.push([
                (att.get_as_string("DX").unwrap_or_default()),
                (att.get_as_string("DY").unwrap_or_default()),
            ]);
        }
    }

    let gm_param = GmParam {
        refno: att.get_refno().unwrap_or_default(),
        gm_type: att.get_type_str().to_owned(),
        prad: (att.get_as_string("PRAD").unwrap_or_default()),
        pang: (att.get_as_string("PANG").unwrap_or_default()),
        pwid: (att.get_as_string("PWID").unwrap_or_default()),
        diameters: att.get_attr_strings(&["PDIA", "PBDM", "PTDM", "DIAM"]),
        distances: att.get_attr_strings(&["PDIS", "PBDI", "PTDI"]),
        shears: att.get_attr_strings(&["PXTS", "PYTS", "PXBS", "PYBS"]),
        phei: (att.get_as_string("PHEI").unwrap_or_default()),
        offset: (att.get_as_string("POFF").unwrap_or_default()),
        lengths: att.get_attr_strings(&["PXLE", "PYLE", "PZLE"]),
        xyz: att.get_attr_strings(&[
            "PX", "PY", "PZ", "PBBT", "PCBT", "PBTP", "PCTP", "PBOF", "PCOF",
        ]),
        verts,
        frads,
        dxy,
        drad: (att.get_as_string("DRAD").unwrap_or_default()),
        dwid: (att.get_as_string("DWID").unwrap_or_default()),
        paxises, // 先pa_axis, 后pb_axis
        centre_line_flag,
        visible_flag: tube_flag,
        plax: att.get_as_string("PLAX"),
    };

    // 🔍 调试：记录提取的表达式（只记录非空的）
    // 特别关注包含 "ATTRIB RPRO" 或 "RPRO" 的表达式
    let mut has_rpro = false;
    let mut rpro_attrs = vec![];

    if !gm_param.prad.is_empty() {
        if gm_param.prad.contains("RPRO") || gm_param.prad.contains("ATTRIB") {
            has_rpro = true;
            rpro_attrs.push(format!("PRAD: {}", gm_param.prad));
        }
    }
    if !gm_param.phei.is_empty() {
        if gm_param.phei.contains("RPRO") || gm_param.phei.contains("ATTRIB") {
            has_rpro = true;
            rpro_attrs.push(format!("PHEI: {}", gm_param.phei));
        }
    }
    if !gm_param.pang.is_empty() {
        if gm_param.pang.contains("RPRO") || gm_param.pang.contains("ATTRIB") {
            has_rpro = true;
            rpro_attrs.push(format!("PANG: {}", gm_param.pang));
        }
    }
    if !gm_param.pwid.is_empty() {
        if gm_param.pwid.contains("RPRO") || gm_param.pwid.contains("ATTRIB") {
            has_rpro = true;
            rpro_attrs.push(format!("PWID: {}", gm_param.pwid));
        }
    }
    if !gm_param.drad.is_empty() {
        if gm_param.drad.contains("RPRO") || gm_param.drad.contains("ATTRIB") {
            has_rpro = true;
            rpro_attrs.push(format!("DRAD: {}", gm_param.drad));
        }
    }
    if !gm_param.dwid.is_empty() {
        if gm_param.dwid.contains("RPRO") || gm_param.dwid.contains("ATTRIB") {
            has_rpro = true;
            rpro_attrs.push(format!("DWID: {}", gm_param.dwid));
        }
    }
    if !gm_param.offset.is_empty() {
        if gm_param.offset.contains("RPRO") || gm_param.offset.contains("ATTRIB") {
            has_rpro = true;
            rpro_attrs.push(format!("POFF: {}", gm_param.offset));
        }
    }

    // 检查数组属性
    for (i, val) in gm_param.diameters.iter().enumerate() {
        if val.contains("RPRO") || val.contains("ATTRIB") {
            has_rpro = true;
            rpro_attrs.push(format!("DIAMETERS[{}]: {}", i, val));
        }
    }
    for (i, val) in gm_param.distances.iter().enumerate() {
        if val.contains("RPRO") || val.contains("ATTRIB") {
            has_rpro = true;
            rpro_attrs.push(format!("DISTANCES[{}]: {}", i, val));
        }
    }
    for (i, val) in gm_param.shears.iter().enumerate() {
        if val.contains("RPRO") || val.contains("ATTRIB") {
            has_rpro = true;
            rpro_attrs.push(format!("SHEARS[{}]: {}", i, val));
        }
    }
    for (i, val) in gm_param.lengths.iter().enumerate() {
        if val.contains("RPRO") || val.contains("ATTRIB") {
            has_rpro = true;
            rpro_attrs.push(format!("LENGTHS[{}]: {}", i, val));
        }
    }
    for (i, val) in gm_param.xyz.iter().enumerate() {
        if val.contains("RPRO") || val.contains("ATTRIB") {
            has_rpro = true;
            rpro_attrs.push(format!("XYZ[{}]: {}", i, val));
        }
    }

    // 如果包含 RPRO 或 ATTRIB，打印详细信息
    if has_rpro {
        crate::debug_model_debug!("   ⚠️  发现包含 RPRO/ATTRIB 的属性:");
        for attr in rpro_attrs {
            crate::debug_model_debug!("     {}", attr);
        }
    }

    Some(gm_param)
}
