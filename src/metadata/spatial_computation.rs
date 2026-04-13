use crate::{
    NamedAttrMap, RefnoEnum, SUL_DB, SurrealQueryExt, get_children_refnos, get_default_full_name,
    get_named_attmap, get_ui_named_attmap, transform,
};
use anyhow::{Context, anyhow};
use glam::{DMat4, DVec3};
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, collections::VecDeque};
use surrealdb::types::SurrealValue;

const DEFAULT_PANEL_SEARCH_RADIUS_MM: f64 = 1500.0;
const DEFAULT_PANEL_SEARCH_Z_WINDOW_MM: f64 = 500.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SuppAnchorKind {
    S1,
    S2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuppAnchorInfo {
    pub kind: SuppAnchorKind,
    pub point_world: DVec3,
    pub source_refno: RefnoEnum,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuppPanelMatch {
    pub panel_refno: RefnoEnum,
    pub panel_name: String,
    pub panel_center_world: DVec3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuppPanelOffset {
    pub anchor_kind: SuppAnchorKind,
    pub anchor_point: DVec3,
    pub panel_refno: RefnoEnum,
    pub panel_center: DVec3,
    pub vector: DVec3,
    pub length: f64,
}

#[derive(Debug, Clone)]
struct SupportDescendant {
    refno: RefnoEnum,
    att: NamedAttrMap,
    world_mat: DMat4,
    world_pos: DVec3,
}

#[derive(Debug, Clone, Deserialize, SurrealValue)]
struct PanelCandidateRow {
    refno: RefnoEnum,
    #[serde(default)]
    full_name: Option<String>,
    #[serde(default)]
    center: Option<Vec<f64>>,
}

fn point_cmp(a: &f64, b: &f64) -> Ordering {
    a.partial_cmp(b).unwrap_or(Ordering::Equal)
}

fn full_name_to_panel_name(full_name: &str) -> String {
    full_name.trim().trim_start_matches('/').to_string()
}

fn is_unset_like(value: Option<&str>) -> bool {
    value
        .map(str::trim)
        .map(|v| v.is_empty() || v.eq_ignore_ascii_case("unset") || v.eq_ignore_ascii_case("none"))
        .unwrap_or(true)
}

fn is_s1_contact_candidate(att: &NamedAttrMap) -> bool {
    let gtyp = att
        .get_str("GTYP")
        .unwrap_or_default()
        .trim()
        .to_uppercase();
    matches!(
        gtyp.as_str(),
        "CYLI" | "SLCY" | "PIPE" | "TUBE" | "DTUB" | "CONE" | "DISH" | "CTOR" | "RTOR"
    )
}

fn is_s2_contact_candidate(att: &NamedAttrMap) -> bool {
    att.get_str("GTYP")
        .map(str::trim)
        .map(|v| v.eq_ignore_ascii_case("BOX"))
        .unwrap_or(false)
}

fn pick_endpoint_by_z(world_mat: &DMat4, att: &NamedAttrMap, pick_max: bool) -> Option<DVec3> {
    let poss = att.get_poss()?;
    let pose = att.get_pose()?;
    let poss_world = world_mat.transform_point3(poss.as_dvec3());
    let pose_world = world_mat.transform_point3(pose.as_dvec3());
    match (point_cmp(&poss_world.z, &pose_world.z), pick_max) {
        (Ordering::Greater | Ordering::Equal, true) => Some(poss_world),
        (Ordering::Less, true) => Some(pose_world),
        (Ordering::Less | Ordering::Equal, false) => Some(poss_world),
        (Ordering::Greater, false) => Some(pose_world),
    }
}

fn parse_center(values: Option<Vec<f64>>) -> Option<DVec3> {
    let values = values?;
    if values.len() < 3 {
        return None;
    }
    Some(DVec3::new(values[0], values[1], values[2]))
}

fn quality_rank(tag: Option<&str>) -> usize {
    if is_unset_like(tag) { 1 } else { 0 }
}

fn panel_search_radius(tolerance: Option<f64>) -> f64 {
    tolerance
        .filter(|value| value.is_finite() && *value > 0.0)
        .map(|value| value.max(DEFAULT_PANEL_SEARCH_RADIUS_MM))
        .unwrap_or(DEFAULT_PANEL_SEARCH_RADIUS_MM)
}

async fn collect_support_descendants(root: RefnoEnum) -> anyhow::Result<Vec<SupportDescendant>> {
    let mut descendants = Vec::new();
    let mut queue = VecDeque::from([root]);
    while let Some(current) = queue.pop_front() {
        for child in get_children_refnos(current).await? {
            queue.push_back(child);
            let att = match get_named_attmap(child).await {
                Ok(att) => att,
                Err(_) => continue,
            };
            let Some(world_mat) = transform::get_world_mat4(child, false).await? else {
                continue;
            };
            let (_, _, world_pos) = world_mat.to_scale_rotation_translation();
            descendants.push(SupportDescendant {
                refno: child,
                att,
                world_mat,
                world_pos,
            });
        }
    }
    Ok(descendants)
}

async fn panel_center_world(refno: RefnoEnum) -> anyhow::Result<DVec3> {
    let world_mat = transform::get_world_mat4(refno, false)
        .await?
        .with_context(|| {
            format!(
                "未找到 PANEL/PANE {} 的世界变换",
                refno.refno().to_slash_string()
            )
        })?;
    let (_, _, translation) = world_mat.to_scale_rotation_translation();
    Ok(translation)
}

async fn query_panel_candidates(
    anchor_point: DVec3,
    tolerance: Option<f64>,
) -> anyhow::Result<Vec<PanelCandidateRow>> {
    let xy_radius = panel_search_radius(tolerance);
    let z_radius = DEFAULT_PANEL_SEARCH_Z_WINDOW_MM.max(xy_radius.min(800.0));
    let sql = format!(
        r#"
        SELECT
            id as refno,
            fn::default_full_name(id) as full_name,
            type::record("pe_transform", record::id(id)).world_trans.d.translation as center
        FROM pe
        WHERE noun IN ['PANE', 'PANEL']
          AND type::record("pe_transform", record::id(id)).world_trans.d.translation != NONE
          AND math::abs(type::record("pe_transform", record::id(id)).world_trans.d.translation[0] - ({x})) <= {xy_radius}
          AND math::abs(type::record("pe_transform", record::id(id)).world_trans.d.translation[1] - ({y})) <= {xy_radius}
          AND math::abs(type::record("pe_transform", record::id(id)).world_trans.d.translation[2] - ({z})) <= {z_radius}
        LIMIT 200;
        "#,
        x = anchor_point.x,
        y = anchor_point.y,
        z = anchor_point.z,
        xy_radius = xy_radius,
        z_radius = z_radius
    );
    SUL_DB.query_take(&sql, 0).await
}

pub async fn resolve_supp_anchor(refno: RefnoEnum) -> anyhow::Result<SuppAnchorInfo> {
    let descendants = collect_support_descendants(refno).await?;
    let mut s1_candidates: Vec<DVec3> = Vec::new();
    let mut s2_box_candidates: Vec<DVec3> = Vec::new();
    let mut pnod_candidates: Vec<DVec3> = Vec::new();

    for descendant in &descendants {
        let type_name = descendant.att.get_type_str().trim().to_uppercase();
        if type_name == "SCTN" {
            if is_s1_contact_candidate(&descendant.att) {
                s1_candidates.push(descendant.world_pos);
            }
            if is_s2_contact_candidate(&descendant.att) {
                s2_box_candidates.push(descendant.world_pos);
            }
        }
        if type_name == "PNOD" {
            pnod_candidates.push(descendant.world_pos);
        }
    }

    if let Some(point_world) = s1_candidates
        .into_iter()
        .max_by(|a, b| point_cmp(&a.z, &b.z))
    {
        return Ok(SuppAnchorInfo {
            kind: SuppAnchorKind::S1,
            point_world,
            source_refno: refno,
        });
    }

    if !s2_box_candidates.is_empty() {
        if let Some(point_world) = pnod_candidates
            .iter()
            .copied()
            .min_by(|a, b| point_cmp(&a.z, &b.z))
        {
            return Ok(SuppAnchorInfo {
                kind: SuppAnchorKind::S2,
                point_world,
                source_refno: refno,
            });
        }

        if let Some(point_world) = s2_box_candidates
            .into_iter()
            .min_by(|a, b| point_cmp(&a.z, &b.z))
        {
            return Ok(SuppAnchorInfo {
                kind: SuppAnchorKind::S2,
                point_world,
                source_refno: refno,
            });
        }
    }

    if let Some(point_world) = pnod_candidates
        .into_iter()
        .min_by(|a, b| point_cmp(&a.z, &b.z))
    {
        return Ok(SuppAnchorInfo {
            kind: SuppAnchorKind::S2,
            point_world,
            source_refno: refno,
        });
    }

    Err(anyhow!(
        "未找到支架 {} 的定位点",
        refno.refno().to_slash_string()
    ))
}

// https://gitee.com/happydpc/rs-server/issues/IB8S8I
/// 找到支吊架对应的土建预埋板
pub async fn get_supp_panel(refno: RefnoEnum) -> anyhow::Result<String> {
    let panel = resolve_supp_panel(refno, None)
        .await?
        .ok_or_else(|| anyhow!("no panel matched"))?;
    Ok(panel.panel_name)
}

pub async fn resolve_supp_panel(
    refno: RefnoEnum,
    tolerance: Option<f64>,
) -> anyhow::Result<Option<SuppPanelMatch>> {
    let anchor = resolve_supp_anchor(refno).await?;
    let candidates = query_panel_candidates(anchor.point_world, tolerance).await?;
    let mut ranked_candidates = Vec::new();

    for row in candidates {
        let Some(center) = parse_center(row.center.clone()) else {
            continue;
        };
        let quality_tag = get_ui_named_attmap(row.refno)
            .await
            .ok()
            .and_then(|att| att.get_str("UDA_JGOBJZL").map(str::to_string));
        let vector = center - anchor.point_world;
        ranked_candidates.push((
            row,
            center,
            quality_rank(quality_tag.as_deref()),
            vector.length(),
            DVec3::new(vector.x, vector.y, 0.0).length(),
            vector.z.abs(),
        ));
    }

    let best = ranked_candidates.into_iter().min_by(|a, b| {
        a.2.cmp(&b.2)
            .then_with(|| point_cmp(&a.4, &b.4))
            .then_with(|| point_cmp(&a.5, &b.5))
            .then_with(|| point_cmp(&a.3, &b.3))
            .then_with(|| a.0.refno.refno().get_0().cmp(&b.0.refno.refno().get_0()))
            .then_with(|| a.0.refno.refno().get_1().cmp(&b.0.refno.refno().get_1()))
    });

    let Some((row, _, _, _, _, _)) = best else {
        return Ok(None);
    };

    let panel_name = row
        .full_name
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(get_default_full_name(row.refno).await?);
    let panel_center_world = panel_center_world(row.refno).await?;

    Ok(Some(SuppPanelMatch {
        panel_refno: row.refno,
        panel_name: full_name_to_panel_name(&panel_name),
        panel_center_world,
    }))
}

pub async fn compute_supp_panel_offset(
    refno: RefnoEnum,
    tolerance: Option<f64>,
) -> anyhow::Result<Option<SuppPanelOffset>> {
    let anchor = resolve_supp_anchor(refno).await?;
    let Some(panel) = resolve_supp_panel(refno, tolerance).await? else {
        return Ok(None);
    };
    let vector = panel.panel_center_world - anchor.point_world;
    Ok(Some(SuppPanelOffset {
        anchor_kind: anchor.kind,
        anchor_point: anchor.point_world,
        panel_refno: panel.panel_refno,
        panel_center: panel.panel_center_world,
        vector,
        length: vector.length(),
    }))
}

// https://gitee.com/happydpc/rs-server/issues/IB8RUF
/// 支吊架下的sctn在空间上找到支撑的bran
pub async fn get_supp_bran(refno: RefnoEnum) -> anyhow::Result<Vec<String>> {
    todo!()
}

// https://gitee.com/happydpc/rs-server/issues/IB8SNG
/// 通过输入支吊架找到支撑的bran，然后找到支吊架旁边两个支架，且着三个支架支撑的都是同一个bran，分别求这个支架与旁边两个支架的距离
pub async fn get_supp_span(refno: RefnoEnum) -> anyhow::Result<[f32; 2]> {
    todo!()
}

// https://gitee.com/happydpc/rs-server/issues/IB9D2S
/// 输入管夹下的PCLA类型，通过管夹找到夹的bran下的管件
pub async fn get_bran_in_pcla(refno: RefnoEnum) -> anyhow::Result<RefnoEnum> {
    todo!()
}

// https://gitee.com/happydpc/rs-server/issues/IB9YKZ
/// 获取panel的长宽
pub async fn get_panel_size(refno: RefnoEnum) -> anyhow::Result<[f32; 2]> {
    let panel = get_named_attmap(refno).await?;
    let quality_tag = panel.get_str("UDA_JGOBJZL").unwrap_or_default().trim();
    if quality_tag.eq_ignore_ascii_case("S-1RS-NI-2D2-02-22C3-23A3|A|新建")
        || quality_tag.eq_ignore_ascii_case("UNSET")
        || panel.get_type_str().eq_ignore_ascii_case("PANE")
    {
        return Ok([600.0, 600.0]);
    }
    Err(anyhow!(
        "暂未识别 PANEL/PANE {} 的长宽",
        refno.refno().to_slash_string()
    ))
}
