use crate::{
    NamedAttrMap, RefnoEnum, SUL_DB, SurrealQueryExt, get_children_refnos, get_default_full_name,
    get_named_attmap, get_ui_named_attmap, query_filter_ancestors, transform,
};
use anyhow::{Context, anyhow};
use glam::{DMat4, DVec3};
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet, VecDeque},
};
use surrealdb::types::SurrealValue;

const DEFAULT_PANEL_SEARCH_RADIUS_MM: f64 = 1500.0;
const DEFAULT_PANEL_SEARCH_Z_WINDOW_MM: f64 = 500.0;
const DEFAULT_BRAN_SEARCH_RADIUS_MM: f64 = 2500.0;

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
    pub match_method: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuppBranMatch {
    pub bran_refno: RefnoEnum,
    pub bran_name: String,
    pub contact_sctn_refno: RefnoEnum,
    pub contact_point_world: DVec3,
    pub match_method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuppSpanResult {
    pub bran_refno: RefnoEnum,
    pub current_anchor: DVec3,
    pub left_suppo_refno: Option<RefnoEnum>,
    pub right_suppo_refno: Option<RefnoEnum>,
    pub left_distance: Option<f64>,
    pub right_distance: Option<f64>,
    pub neighbor_window: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuppDistanceCandidate {
    pub refno: RefnoEnum,
    pub noun: String,
    pub closest_point_world: DVec3,
    pub distance_mm: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuppWallMatch {
    pub anchor_kind: SuppAnchorKind,
    pub anchor_point: DVec3,
    pub target_refno: RefnoEnum,
    pub target_noun: String,
    pub closest_point_world: DVec3,
    pub distance_mm: f64,
    pub candidates: Vec<SuppDistanceCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuppSteelMatch {
    pub anchor_kind: SuppAnchorKind,
    pub anchor_point: DVec3,
    pub steel_refno: RefnoEnum,
    pub steel_noun: String,
    pub closest_point_world: DVec3,
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

#[derive(Debug, Clone, Deserialize, SurrealValue)]
struct NearbyElementRow {
    refno: RefnoEnum,
    noun: String,
    #[serde(default)]
    center: Option<Vec<f64>>,
    #[serde(default)]
    mins: Option<Vec<f64>>,
    #[serde(default)]
    maxs: Option<Vec<f64>>,
}

#[derive(Debug, Clone, Deserialize, SurrealValue)]
struct PanelSizeAabbRow {
    #[serde(default)]
    mins: Option<Vec<f64>>,
    #[serde(default)]
    maxs: Option<Vec<f64>>,
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

fn parse_aabb(values: Option<Vec<f64>>, fallback: DVec3) -> DVec3 {
    parse_center(values).unwrap_or(fallback)
}

fn clamp_point_to_aabb(point: DVec3, mins: DVec3, maxs: DVec3) -> DVec3 {
    DVec3::new(
        point.x.clamp(mins.x, maxs.x),
        point.y.clamp(mins.y, maxs.y),
        point.z.clamp(mins.z, maxs.z),
    )
}

fn same_dbnum(lhs: RefnoEnum, rhs: RefnoEnum) -> bool {
    lhs.refno().get_0() == rhs.refno().get_0()
}

fn quality_rank(tag: Option<&str>) -> usize {
    if is_unset_like(tag) { 1 } else { 0 }
}

fn direct_panel_match_method(vector: DVec3, tolerance: Option<f64>) -> &'static str {
    let z_window = tolerance
        .filter(|value| value.is_finite() && *value > 0.0)
        .map(|value| value.max(50.0))
        .unwrap_or(50.0);
    if vector.z.abs() <= z_window {
        "direct_contact"
    } else {
        "spatial_nearest"
    }
}

fn panel_search_radius(tolerance: Option<f64>) -> f64 {
    tolerance
        .filter(|value| value.is_finite() && *value > 0.0)
        .map(|value| value.max(DEFAULT_PANEL_SEARCH_RADIUS_MM))
        .unwrap_or(DEFAULT_PANEL_SEARCH_RADIUS_MM)
}

fn supp_bran_candidate_nouns() -> Vec<String> {
    [
        "BRAN", "HANG", "SCTN", "GENSEC", "FTUB", "ELBO", "TEE", "BEND", "OFST", "REDU", "CAP",
        "CROS", "COUP", "TRNS", "DUCT", "FLEX", "VALV", "FBLI", "HACC", "GRIL", "DAMP",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
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

async fn resolve_span_fallback_subjects(
    refno: RefnoEnum,
    anchor_point: DVec3,
) -> anyhow::Result<Vec<RefnoEnum>> {
    let root_type = get_named_attmap(refno)
        .await?
        .get_type_str()
        .trim()
        .to_uppercase();
    if root_type != "STRU" {
        return Ok(Vec::new());
    }

    let mut candidates = Vec::<(RefnoEnum, f64)>::new();
    for child in get_children_refnos(refno).await? {
        let child_type = match get_named_attmap(child).await {
            Ok(att) => att.get_type_str().trim().to_uppercase(),
            Err(_) => continue,
        };
        if child_type != "FRMW" {
            continue;
        }
        let child_anchor = match resolve_supp_anchor(child).await {
            Ok(anchor) => anchor.point_world,
            Err(_) => continue,
        };
        let distance = (child_anchor - anchor_point).length();
        if distance <= 1.0 {
            candidates.push((child, distance));
        }
    }
    candidates.sort_by(|a, b| point_cmp(&a.1, &b.1));
    Ok(candidates.into_iter().map(|item| item.0).collect())
}

async fn compute_supp_span_for_subject(
    subject_refno: RefnoEnum,
    current_anchor: &SuppAnchorInfo,
    window: f64,
) -> anyhow::Result<Option<SuppSpanResult>> {
    fn build_span_result(
        current_anchor: DVec3,
        current_bran: &SuppBranMatch,
        neighbor_refs: &[(RefnoEnum, DVec3)],
        window: f64,
    ) -> Option<SuppSpanResult> {
        if neighbor_refs.is_empty() {
            return None;
        }

        let dominant_axis = {
            let mut max_x = 0.0_f64;
            let mut max_y = 0.0_f64;
            for (_, point) in neighbor_refs {
                max_x = max_x.max((point.x - current_anchor.x).abs());
                max_y = max_y.max((point.y - current_anchor.y).abs());
            }
            if max_x >= max_y { 0 } else { 1 }
        };

        let mut left: Option<(RefnoEnum, f64)> = None;
        let mut right: Option<(RefnoEnum, f64)> = None;
        for (support_refno, point) in neighbor_refs {
            let signed = if dominant_axis == 0 {
                point.x - current_anchor.x
            } else {
                point.y - current_anchor.y
            };
            let distance = (*point - current_anchor).length();
            if signed < 0.0 {
                if left.is_none_or(|best| distance < best.1) {
                    left = Some((*support_refno, distance));
                }
            } else if signed > 0.0 {
                if right.is_none_or(|best| distance < best.1) {
                    right = Some((*support_refno, distance));
                }
            }
        }

        Some(SuppSpanResult {
            bran_refno: current_bran.bran_refno,
            current_anchor,
            left_suppo_refno: left.map(|item| item.0),
            right_suppo_refno: right.map(|item| item.0),
            left_distance: left.map(|item| item.1),
            right_distance: right.map(|item| item.1),
            neighbor_window: window,
        })
    }

    let debug_span = matches!(
        subject_refno.refno().to_slash_string().as_str(),
        "24383/86525" | "24383/86526"
    );
    let current_bran_matches = resolve_supp_bran(subject_refno, None).await?;
    if current_bran_matches.is_empty() {
        return Ok(None);
    }

    let root_type = get_named_attmap(subject_refno)
        .await?
        .get_type_str()
        .trim()
        .to_uppercase();
    let nearby = query_nearby_world_elements_filtered(
        subject_refno,
        current_anchor.point_world,
        window,
        &["SCTN", "PNOD"]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>(),
        400,
        true,
    )
    .await?;
    let mut support_roots = BTreeMap::<String, (RefnoEnum, f64)>::new();

    for row in nearby {
        let ancestor_types = vec![root_type.as_str()];
        let root_candidates = query_filter_ancestors(row.refno, &ancestor_types).await?;
        let Some(root_refno) = root_candidates.last().copied() else {
            continue;
        };
        if root_refno == subject_refno {
            continue;
        }
        let point = parse_center(row.center.clone()).unwrap_or(current_anchor.point_world);
        let distance = (point - current_anchor.point_world).length();
        let key = root_refno.refno().to_slash_string();
        match support_roots.get(&key) {
            Some((_, best_distance)) if *best_distance <= distance => {}
            _ => {
                support_roots.insert(key, (root_refno, distance));
            }
        }
    }

    let mut support_items = support_roots.into_values().collect::<Vec<_>>();
    support_items.sort_by(|a, b| point_cmp(&a.1, &b.1));
    let support_items = support_items
        .into_iter()
        .take(32)
        .map(|item| item.0)
        .collect::<Vec<_>>();
    if debug_span {
        eprintln!(
            "[supp-span-debug] subject={} support_items={:?}",
            subject_refno.refno().to_slash_string(),
            support_items
                .iter()
                .map(|item| item.refno().to_slash_string())
                .collect::<Vec<_>>()
        );
    }

    let mut support_match_cache = BTreeMap::<String, Vec<SuppBranMatch>>::new();
    let mut support_anchor_cache = BTreeMap::<String, DVec3>::new();
    for support_refno in &support_items {
        let key = support_refno.refno().to_slash_string();
        let bran_matches = match resolve_supp_bran(*support_refno, None).await {
            Ok(matches) => matches,
            Err(_) => continue,
        };
        let anchor = match resolve_supp_anchor(*support_refno).await {
            Ok(anchor) => anchor.point_world,
            Err(_) => continue,
        };
        support_match_cache.insert(key.clone(), bran_matches);
        support_anchor_cache.insert(key, anchor);
    }

    let mut bran_candidates = current_bran_matches;
    bran_candidates.sort_by(|a, b| {
        a.match_method.cmp(&b.match_method).then_with(|| {
            a.bran_refno
                .refno()
                .to_slash_string()
                .cmp(&b.bran_refno.refno().to_slash_string())
        })
    });

    let mut best_partial: Option<SuppSpanResult> = None;
    for current_bran in &bran_candidates {
        let mut neighbor_refs = Vec::<(RefnoEnum, DVec3)>::new();
        for support_refno in &support_items {
            let key = support_refno.refno().to_slash_string();
            let Some(bran_matches) = support_match_cache.get(&key) else {
                continue;
            };
            if !bran_matches
                .iter()
                .any(|item| item.bran_refno == current_bran.bran_refno)
            {
                continue;
            }
            let Some(anchor_point) = support_anchor_cache.get(&key) else {
                continue;
            };
            neighbor_refs.push((*support_refno, *anchor_point));
        }
        if debug_span {
            eprintln!(
                "[supp-span-debug] subject={} bran={} neighbors={:?}",
                subject_refno.refno().to_slash_string(),
                current_bran.bran_refno.refno().to_slash_string(),
                neighbor_refs
                    .iter()
                    .map(|(item, point)| {
                        (item.refno().to_slash_string(), [point.x, point.y, point.z])
                    })
                    .collect::<Vec<_>>()
            );
        }

        let Some(result) = build_span_result(
            current_anchor.point_world,
            current_bran,
            &neighbor_refs,
            window,
        ) else {
            continue;
        };

        if result.left_suppo_refno.is_some() && result.right_suppo_refno.is_some() {
            return Ok(Some(result));
        }
        if best_partial.is_none() {
            best_partial = Some(result);
        }
    }

    Ok(best_partial)
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

async fn query_nearby_world_elements(
    anchor_point: DVec3,
    radius_mm: f64,
) -> anyhow::Result<Vec<NearbyElementRow>> {
    let radius_mm = radius_mm.max(200.0);
    let sql = format!(
        r#"
        SELECT
            id as refno,
            noun,
            type::record("pe_transform", record::id(id)).world_trans.d.translation as center,
            type::record("inst_relate_aabb", record::id(id)).aabb_id.d.mins as mins,
            type::record("inst_relate_aabb", record::id(id)).aabb_id.d.maxs as maxs,
            (
                math::abs(type::record("pe_transform", record::id(id)).world_trans.d.translation[0] - ({x})) +
                math::abs(type::record("pe_transform", record::id(id)).world_trans.d.translation[1] - ({y})) +
                math::abs(type::record("pe_transform", record::id(id)).world_trans.d.translation[2] - ({z}))
            ) as distance_sort
        FROM pe
        WHERE type::record("pe_transform", record::id(id)).world_trans.d.translation != NONE
          AND math::abs(type::record("pe_transform", record::id(id)).world_trans.d.translation[0] - ({x})) <= {radius}
          AND math::abs(type::record("pe_transform", record::id(id)).world_trans.d.translation[1] - ({y})) <= {radius}
          AND math::abs(type::record("pe_transform", record::id(id)).world_trans.d.translation[2] - ({z})) <= {radius}
        ORDER BY distance_sort
        LIMIT 400;
        "#,
        x = anchor_point.x,
        y = anchor_point.y,
        z = anchor_point.z,
        radius = radius_mm
    );
    SUL_DB.query_take(&sql, 0).await
}

async fn query_nearby_world_elements_filtered(
    source_refno: RefnoEnum,
    anchor_point: DVec3,
    radius_mm: f64,
    nouns: &[String],
    limit: usize,
    same_db_only: bool,
) -> anyhow::Result<Vec<NearbyElementRow>> {
    let limit = limit.clamp(1, 500);
    let noun_list = nouns
        .iter()
        .map(|value| format!("'{}'", value.trim().to_uppercase().replace('\'', "\\'")))
        .collect::<Vec<_>>();
    if noun_list.is_empty() {
        return Ok(Vec::new());
    }
    let radius_mm = radius_mm.max(200.0);
    let sql = format!(
        r#"
        SELECT
            id as refno,
            noun,
            type::record("pe_transform", record::id(id)).world_trans.d.translation as center,
            type::record("inst_relate_aabb", record::id(id)).aabb_id.d.mins as mins,
            type::record("inst_relate_aabb", record::id(id)).aabb_id.d.maxs as maxs,
            (
                math::abs(type::record("pe_transform", record::id(id)).world_trans.d.translation[0] - ({x})) +
                math::abs(type::record("pe_transform", record::id(id)).world_trans.d.translation[1] - ({y})) +
                math::abs(type::record("pe_transform", record::id(id)).world_trans.d.translation[2] - ({z}))
            ) as distance_sort
        FROM pe
        WHERE noun IN [{nouns}]
          AND type::record("pe_transform", record::id(id)).world_trans.d.translation != NONE
          AND math::abs(type::record("pe_transform", record::id(id)).world_trans.d.translation[0] - ({x})) <= {radius}
          AND math::abs(type::record("pe_transform", record::id(id)).world_trans.d.translation[1] - ({y})) <= {radius}
          AND math::abs(type::record("pe_transform", record::id(id)).world_trans.d.translation[2] - ({z})) <= {radius}
        ORDER BY distance_sort
        LIMIT {limit};
        "#,
        nouns = noun_list.join(", "),
        x = anchor_point.x,
        y = anchor_point.y,
        z = anchor_point.z,
        radius = radius_mm,
        limit = limit
    );
    let rows: Vec<NearbyElementRow> = SUL_DB.query_take(&sql, 0).await?;
    if same_db_only {
        Ok(rows
            .into_iter()
            .filter(|row| same_dbnum(row.refno, source_refno))
            .collect())
    } else {
        Ok(rows)
    }
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
            type::record("pe_transform", record::id(id)).world_trans.d.translation as center,
            (
                math::abs(type::record("pe_transform", record::id(id)).world_trans.d.translation[0] - ({x})) +
                math::abs(type::record("pe_transform", record::id(id)).world_trans.d.translation[1] - ({y})) +
                math::abs(type::record("pe_transform", record::id(id)).world_trans.d.translation[2] - ({z}))
            ) as distance_sort
        FROM pe
        WHERE noun IN ['PANE', 'PANEL']
          AND type::record("pe_transform", record::id(id)).world_trans.d.translation != NONE
          AND math::abs(type::record("pe_transform", record::id(id)).world_trans.d.translation[0] - ({x})) <= {xy_radius}
          AND math::abs(type::record("pe_transform", record::id(id)).world_trans.d.translation[1] - ({y})) <= {xy_radius}
          AND math::abs(type::record("pe_transform", record::id(id)).world_trans.d.translation[2] - ({z})) <= {z_radius}
        ORDER BY distance_sort
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
    let match_method =
        direct_panel_match_method(panel_center_world - anchor.point_world, tolerance);

    Ok(Some(SuppPanelMatch {
        panel_refno: row.refno,
        panel_name: full_name_to_panel_name(&panel_name),
        panel_center_world,
        match_method: match_method.to_string(),
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

pub async fn resolve_supp_bran(
    refno: RefnoEnum,
    tolerance: Option<f64>,
) -> anyhow::Result<Vec<SuppBranMatch>> {
    let anchor = resolve_supp_anchor(refno).await?;
    let search_radius = tolerance
        .filter(|value| value.is_finite() && *value > 0.0)
        .map(|value| value.max(DEFAULT_BRAN_SEARCH_RADIUS_MM))
        .unwrap_or(DEFAULT_BRAN_SEARCH_RADIUS_MM);
    let nearby = query_nearby_world_elements_filtered(
        refno,
        anchor.point_world,
        search_radius,
        &supp_bran_candidate_nouns(),
        200,
        false,
    )
    .await?;
    let mut best_by_bran = BTreeMap::<String, (SuppBranMatch, f64, f64)>::new();
    let direct_gap = tolerance
        .filter(|value| value.is_finite() && *value > 0.0)
        .map(|value| value.max(80.0))
        .unwrap_or(80.0);

    for row in nearby {
        let Some(center) = parse_center(row.center.clone()) else {
            continue;
        };
        let row_noun = row.noun.trim().to_uppercase();
        let bran_refno = if matches!(row_noun.as_str(), "BRAN" | "HANG") {
            row.refno
        } else {
            let bran_ancestors = query_filter_ancestors(row.refno, &["BRAN", "HANG"]).await?;
            if bran_ancestors.is_empty() {
                continue;
            }
            let mut bran_refno = *bran_ancestors.last().unwrap();
            for candidate in &bran_ancestors {
                if let Ok(att) = get_named_attmap(*candidate).await
                    && att.get_type_str().eq_ignore_ascii_case("BRAN")
                {
                    bran_refno = *candidate;
                    break;
                }
            }
            bran_refno
        };
        let bran_key = bran_refno.refno().to_slash_string();
        let bran_name = get_default_full_name(bran_refno)
            .await
            .map(|value| full_name_to_panel_name(&value))
            .unwrap_or_else(|_| bran_key.clone());
        let mins = parse_aabb(row.mins.clone(), center);
        let maxs = parse_aabb(row.maxs.clone(), center);
        let contact_point_world = clamp_point_to_aabb(anchor.point_world, mins, maxs);
        let vector = contact_point_world - anchor.point_world;
        let horizontal = DVec3::new(vector.x, vector.y, 0.0).length();
        let vertical = vector.z.abs();
        let match_method = if vertical <= direct_gap {
            "direct_contact"
        } else {
            "spatial_nearest"
        };
        let candidate = SuppBranMatch {
            bran_refno,
            bran_name,
            contact_sctn_refno: row.refno,
            contact_point_world,
            match_method: match_method.to_string(),
        };
        match best_by_bran.get(&bran_key) {
            Some((_, best_horizontal, best_vertical))
                if *best_horizontal < horizontal
                    || (*best_horizontal == horizontal && *best_vertical <= vertical) => {}
            _ => {
                best_by_bran.insert(bran_key, (candidate, horizontal, vertical));
            }
        }
    }

    let mut matches = best_by_bran
        .into_values()
        .map(|(candidate, horizontal, vertical)| (candidate, horizontal, vertical))
        .collect::<Vec<_>>();
    matches.sort_by(|a, b| {
        a.0.match_method
            .cmp(&b.0.match_method)
            .then_with(|| point_cmp(&a.1, &b.1))
            .then_with(|| point_cmp(&a.2, &b.2))
            .then_with(|| a.0.bran_name.cmp(&b.0.bran_name))
    });
    Ok(matches
        .into_iter()
        .map(|(candidate, _, _)| candidate)
        .collect())
}

// https://gitee.com/happydpc/rs-server/issues/IB8RUF
/// 支吊架下的sctn在空间上找到支撑的bran
pub async fn get_supp_bran(refno: RefnoEnum) -> anyhow::Result<Vec<String>> {
    let matches = resolve_supp_bran(refno, None).await?;
    if matches.is_empty() {
        return Err(anyhow!(
            "未找到支架 {} 对应的 BRAN/HANG",
            refno.refno().to_slash_string()
        ));
    }
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for item in matches {
        if seen.insert(item.bran_name.clone()) {
            out.push(item.bran_name);
        }
    }
    Ok(out)
}

// https://gitee.com/happydpc/rs-server/issues/IB8SNG
/// 通过输入支吊架找到支撑的bran，然后找到支吊架旁边两个支架，且着三个支架支撑的都是同一个bran，分别求这个支架与旁边两个支架的距离
pub async fn compute_supp_span(
    refno: RefnoEnum,
    neighbor_window: Option<f64>,
) -> anyhow::Result<Option<SuppSpanResult>> {
    let current_anchor = resolve_supp_anchor(refno).await?;
    let window = neighbor_window
        .filter(|value| value.is_finite() && *value > 0.0)
        .map(|value| value.max(500.0))
        .unwrap_or(5000.0);

    if let Some(result) = compute_supp_span_for_subject(refno, &current_anchor, window).await? {
        return Ok(Some(result));
    }

    for subject_refno in resolve_span_fallback_subjects(refno, current_anchor.point_world).await? {
        if let Some(result) =
            compute_supp_span_for_subject(subject_refno, &current_anchor, window).await?
        {
            return Ok(Some(result));
        }
    }

    Ok(None)
}

pub async fn get_supp_span(refno: RefnoEnum) -> anyhow::Result<[f32; 2]> {
    let Some(result) = compute_supp_span(refno, None).await? else {
        return Err(anyhow!(
            "未找到支架 {} 左右两侧的有效跨度",
            refno.refno().to_slash_string()
        ));
    };
    match (result.left_distance, result.right_distance) {
        (Some(left), Some(right)) => Ok([left as f32, right as f32]),
        _ => Err(anyhow!(
            "未找到支架 {} 左右两侧的有效跨度",
            refno.refno().to_slash_string()
        )),
    }
}

pub async fn resolve_supp_wall(
    refno: RefnoEnum,
    search_radius: Option<f64>,
    target_nouns: &[String],
) -> anyhow::Result<Option<SuppWallMatch>> {
    let anchor = resolve_supp_anchor(refno).await?;
    let radius = search_radius
        .filter(|value| value.is_finite() && *value > 0.0)
        .map(|value| value.max(500.0))
        .unwrap_or(5000.0);
    let rows = query_nearby_world_elements_filtered(
        refno,
        anchor.point_world,
        radius,
        target_nouns,
        200,
        false,
    )
    .await?;
    let mut candidates = Vec::<SuppDistanceCandidate>::new();
    for row in rows {
        let center = parse_center(row.center.clone()).unwrap_or(anchor.point_world);
        let mins = parse_aabb(row.mins.clone(), center);
        let maxs = parse_aabb(row.maxs.clone(), center);
        let closest_point_world = clamp_point_to_aabb(anchor.point_world, mins, maxs);
        let distance_mm = (closest_point_world - anchor.point_world).length();
        candidates.push(SuppDistanceCandidate {
            refno: row.refno,
            noun: row.noun.trim().to_uppercase(),
            closest_point_world,
            distance_mm,
        });
    }
    candidates.sort_by(|a, b| {
        point_cmp(&a.distance_mm, &b.distance_mm)
            .then_with(|| a.refno.refno().get_0().cmp(&b.refno.refno().get_0()))
            .then_with(|| a.refno.refno().get_1().cmp(&b.refno.refno().get_1()))
    });
    let Some(target) = candidates.first().cloned() else {
        return Ok(None);
    };
    Ok(Some(SuppWallMatch {
        anchor_kind: anchor.kind,
        anchor_point: anchor.point_world,
        target_refno: target.refno,
        target_noun: target.noun.clone(),
        closest_point_world: target.closest_point_world,
        distance_mm: target.distance_mm,
        candidates,
    }))
}

pub async fn resolve_supp_steel(
    refno: RefnoEnum,
    search_radius: Option<f64>,
    excluded_refnos: &[RefnoEnum],
) -> anyhow::Result<Option<SuppSteelMatch>> {
    let anchor = resolve_supp_anchor(refno).await?;
    let radius = search_radius
        .filter(|value| value.is_finite() && *value > 0.0)
        .map(|value| value.max(500.0))
        .unwrap_or(3000.0);
    let excluded = excluded_refnos
        .iter()
        .map(|item| item.refno().to_slash_string())
        .collect::<BTreeSet<_>>();
    let rows = query_nearby_world_elements_filtered(
        refno,
        anchor.point_world,
        radius,
        &["SCTN", "GENSEC", "STWALL", "STRU", "FRMW"]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>(),
        200,
        true,
    )
    .await?;
    let mut candidates = Vec::<SuppDistanceCandidate>::new();
    for row in rows {
        if excluded.contains(&row.refno.refno().to_slash_string()) {
            continue;
        }
        let center = parse_center(row.center.clone()).unwrap_or(anchor.point_world);
        let mins = parse_aabb(row.mins.clone(), center);
        let maxs = parse_aabb(row.maxs.clone(), center);
        let closest_point_world = clamp_point_to_aabb(anchor.point_world, mins, maxs);
        let distance_mm = (closest_point_world - anchor.point_world).length();
        candidates.push(SuppDistanceCandidate {
            refno: row.refno,
            noun: row.noun.trim().to_uppercase(),
            closest_point_world,
            distance_mm,
        });
    }
    candidates.sort_by(|a, b| {
        point_cmp(&a.distance_mm, &b.distance_mm)
            .then_with(|| a.refno.refno().get_0().cmp(&b.refno.refno().get_0()))
            .then_with(|| a.refno.refno().get_1().cmp(&b.refno.refno().get_1()))
    });
    let Some(target) = candidates.first().cloned() else {
        return Ok(None);
    };
    let vector = target.closest_point_world - anchor.point_world;
    Ok(Some(SuppSteelMatch {
        anchor_kind: anchor.kind,
        anchor_point: anchor.point_world,
        steel_refno: target.refno,
        steel_noun: target.noun,
        closest_point_world: target.closest_point_world,
        vector,
        length: vector.length(),
    }))
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
    for keys in [
        ("XLEN", "YLEN"),
        ("XLE", "YLE"),
        ("LX", "LY"),
        ("WIDTH", "HEIGHT"),
        ("DX", "DY"),
    ] {
        if let (Some(a), Some(b)) = (panel.get_f32(keys.0), panel.get_f32(keys.1))
            && a.is_finite()
            && b.is_finite()
            && a > 0.0
            && b > 0.0
        {
            return Ok([a, b]);
        }
    }

    let sql = format!(
        r#"
        SELECT
            type::record("inst_relate_aabb", record::id({pe_key})).aabb_id.d.mins as mins,
            type::record("inst_relate_aabb", record::id({pe_key})).aabb_id.d.maxs as maxs
        ;
        "#,
        pe_key = refno.to_pe_key()
    );
    let row: Option<PanelSizeAabbRow> = SUL_DB.query_take(&sql, 0).await?;
    if let Some(row) = row
        && let (Some(mins), Some(maxs)) = (row.mins, row.maxs)
        && mins.len() >= 3
        && maxs.len() >= 3
    {
        let mut dims = [
            (maxs[0] - mins[0]).abs() as f32,
            (maxs[1] - mins[1]).abs() as f32,
            (maxs[2] - mins[2]).abs() as f32,
        ];
        dims.sort_by(|a, b| b.partial_cmp(a).unwrap_or(Ordering::Equal));
        if dims[0].is_finite() && dims[1].is_finite() && dims[0] > 0.0 && dims[1] > 0.0 {
            return Ok([dims[0], dims[1]]);
        }
    }

    Err(anyhow!(
        "暂未识别 PANEL/PANE {} 的长宽",
        refno.refno().to_slash_string()
    ))
}
