//! P4: putIntoIsoLine — assign dimension/material/weld objects into isoline bags.
//! Spec + fixture: `teach/fixtures/p4-put-into-isoline.json` · lessons 3 / 47 / 48 / 69 / 70.
//!
//! Teaching implementation, NOT wired into any real extract path. It only decides
//! each object's `goodx` (target isoline index), the write order inside a bag
//! (`insert(1)` for slope/adjust-weld/mbd-angle, else append), and the optional
//! ISOWELDTEXT `movedis` offset. It never opens / merges isolines.

use glam::Vec3;
use std::collections::HashMap;

/// PML thresholds (degrees). U direction is the "horizontal reference" for the
/// non-weldtext tie-break ("更接近水平").
#[derive(Debug, Clone, Copy)]
pub struct PutThresholds {
    pub leg_parallel_max_deg: f64,
    pub samedirs_adir_ldir_min_deg: f64,
    pub weldtext_flip_showdir_deg: f64,
    pub u_dir: Vec3,
}

impl Default for PutThresholds {
    fn default() -> Self {
        Self {
            leg_parallel_max_deg: 1.0,
            samedirs_adir_ldir_min_deg: 179.0,
            weldtext_flip_showdir_deg: 90.0,
            u_dir: Vec3::Z,
        }
    }
}

/// A P1-split isoline segment with its member name list.
#[derive(Debug, Clone)]
pub struct IsoGeom {
    pub index: usize,
    pub mems: Vec<String>,
    pub p0: Vec3,
    pub p1: Vec3,
    pub pipedir: Vec3,
    pub showdir: Vec3,
}

/// An object waiting to be placed into an isoline bag.
#[derive(Debug, Clone)]
pub struct DimObject {
    pub id: String,
    pub objecttype: String,
    pub name: Option<String>,
    pub name_is_atta: bool,
    pub elboname: Option<String>,
    pub ori_zdir: Option<Vec3>,
    pub attapos: Option<Vec3>,
    pub pos: Vec3,
    pub member_adir: Option<Vec3>,
    pub member_ldir: Option<Vec3>,
    pub movedir: Option<Vec3>,
    pub movedis: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteMode {
    Append,
    InsertFront,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Placement {
    pub obj_id: String,
    /// Target isoline index, or `None` when skipped.
    pub goodx: Option<usize>,
    pub write: WriteMode,
    /// Final position (after ISOWELDTEXT movedis offset); equals `pos` otherwise.
    pub final_pos: Vec3,
    /// Set when skipped or otherwise noteworthy.
    pub reason: Option<String>,
}

const INSERT_FRONT_TYPES: [&str; 3] = ["ISOSLOPE", "ISOADJUSTWELD", "ISOMBDANGLE"];

fn angle_deg(a: Vec3, b: Vec3) -> f64 {
    let a = a.normalize_or_zero();
    let b = b.normalize_or_zero();
    if a.length_squared() < 1e-12 || b.length_squared() < 1e-12 {
        return 0.0;
    }
    let cos = a.dot(b).clamp(-1.0, 1.0);
    (cos.acos() as f64).to_degrees()
}

/// Fold an angle into `[0, 90]` (direction sign-agnostic).
fn fold90(a: f64) -> f64 {
    a.min(180.0 - a)
}

/// Projection parameter of `p` onto the infinite line through the segment.
fn project_t(p: Vec3, seg: &IsoGeom) -> f64 {
    let d = seg.p1 - seg.p0;
    let len2 = d.length_squared();
    if len2 < 1e-9 {
        return 0.0;
    }
    ((p - seg.p0).dot(d) / len2) as f64
}

fn within_segment(t: f64) -> bool {
    (-1e-6..=1.0 + 1e-6).contains(&t)
}

/// Perpendicular distance from `p` to the segment's infinite line.
fn line_distance(p: Vec3, seg: &IsoGeom) -> f64 {
    let d = seg.p1 - seg.p0;
    let t = project_t(p, seg) as f32;
    let foot = seg.p0 + d * t;
    p.distance(foot) as f64
}

/// "More horizontal" score: 0 when pipedir ⟂ U (fully horizontal), 90 when ∥ U.
fn horizontality_score(pipedir: Vec3, u: Vec3) -> f64 {
    (angle_deg(pipedir, u) - 90.0).abs()
}

fn is_head_or_tail(name: &str) -> bool {
    name.eq_ignore_ascii_case("HEAD") || name.eq_ignore_ascii_case("TAIL")
}

/// PML `samedirs`: true ⇒ break at first name hit (straight-through member);
/// false ⇒ collect every hit (bridging elbow), so multi-candidate tie-break runs.
fn samedirs(obj: &DimObject, th: &PutThresholds) -> bool {
    if let Some(name) = &obj.name {
        if is_head_or_tail(name) {
            return true;
        }
    }
    match (obj.member_adir, obj.member_ldir) {
        (Some(adir), Some(ldir)) => angle_deg(adir, ldir) >= th.samedirs_adir_ldir_min_deg,
        _ => true,
    }
}

/// Pick the min-distance candidate (by `line_distance` to `p`).
fn nearest_by_distance(cands: &[usize], isolines: &[IsoGeom], p: Vec3) -> Option<usize> {
    cands
        .iter()
        .copied()
        .min_by(|&a, &b| {
            line_distance(p, &isolines[a])
                .partial_cmp(&line_distance(p, &isolines[b]))
                .unwrap()
        })
}

fn tie_break(obj: &DimObject, cands: &[usize], isolines: &[IsoGeom], th: &PutThresholds) -> Option<usize> {
    match cands.len() {
        0 => None,
        1 => Some(cands[0]),
        _ => {
            if obj.objecttype == "ISOWELDTEXT" {
                nearest_by_distance(cands, isolines, obj.pos)
            } else {
                cands.iter().copied().min_by(|&a, &b| {
                    horizontality_score(isolines[a].pipedir, th.u_dir)
                        .partial_cmp(&horizontality_score(isolines[b].pipedir, th.u_dir))
                        .unwrap()
                })
            }
        }
    }
}

/// Resolve `goodx` for one object. `None` ⇒ skip.
fn resolve_goodx(obj: &DimObject, isolines: &[IsoGeom], th: &PutThresholds) -> Option<usize> {
    // Path A — leg (ISOLEG with elboname): near-parallel pipedir + attapos on segment.
    if obj.objecttype == "ISOLEG" && obj.elboname.is_some() {
        let (Some(zdir), Some(attapos)) = (obj.ori_zdir, obj.attapos) else {
            return None;
        };
        let cands: Vec<usize> = isolines
            .iter()
            .filter(|iso| {
                fold90(angle_deg(zdir, iso.pipedir)) < th.leg_parallel_max_deg
                    && within_segment(project_t(attapos, iso))
            })
            .map(|iso| iso.index)
            .collect();
        return nearest_by_distance(&cands, isolines, attapos);
    }

    // Path A — generic projection (unnamed objects): nearest segment the point projects onto.
    if obj.name.is_none() {
        let cands: Vec<usize> = isolines
            .iter()
            .filter(|iso| within_segment(project_t(obj.pos, iso)))
            .map(|iso| iso.index)
            .collect();
        return nearest_by_distance(&cands, isolines, obj.pos);
    }

    // Path B — ATTA weldtext: projection-collect all hit segments, then distance tie-break.
    if obj.objecttype == "ISOWELDTEXT" && obj.name_is_atta {
        let cands: Vec<usize> = isolines
            .iter()
            .filter(|iso| within_segment(project_t(obj.pos, iso)))
            .map(|iso| iso.index)
            .collect();
        return tie_break(obj, &cands, isolines, th);
    }

    // Path B — name match in mems (bridging elbow may hit multiple), then tie-break.
    let name = obj.name.as_ref().unwrap();
    let break_first = samedirs(obj, th);
    let mut cands: Vec<usize> = Vec::new();
    for iso in isolines {
        if iso.mems.iter().any(|m| m == name) {
            cands.push(iso.index);
            if break_first {
                break;
            }
        }
    }
    tie_break(obj, &cands, isolines, th)
}

fn write_mode(objecttype: &str) -> WriteMode {
    if INSERT_FRONT_TYPES.contains(&objecttype) {
        WriteMode::InsertFront
    } else {
        WriteMode::Append
    }
}

/// ISOWELDTEXT movedis offset: flip `movedir` when it opposes the target segment's
/// `showdir`, then offset `pos` by `movedir * movedis`.
fn apply_movedis(obj: &DimObject, goodx: usize, isolines: &[IsoGeom], th: &PutThresholds) -> Vec3 {
    if obj.objecttype != "ISOWELDTEXT" || obj.movedis == 0.0 {
        return obj.pos;
    }
    let Some(mut md) = obj.movedir else {
        return obj.pos;
    };
    if angle_deg(md, isolines[goodx].showdir) > th.weldtext_flip_showdir_deg {
        md = -md;
    }
    obj.pos + md.normalize_or_zero() * obj.movedis as f32
}

/// Place every object into an isoline bag. Returns one `Placement` per object,
/// in input order (which is also the PML `this.objects` iteration order).
pub fn put_into_isoline_pml(
    isolines: &[IsoGeom],
    objects: &[DimObject],
    th: &PutThresholds,
) -> Vec<Placement> {
    objects
        .iter()
        .map(|obj| match resolve_goodx(obj, isolines, th) {
            Some(goodx) => Placement {
                obj_id: obj.id.clone(),
                goodx: Some(goodx),
                write: write_mode(&obj.objecttype),
                final_pos: apply_movedis(obj, goodx, isolines, th),
                reason: None,
            },
            None => Placement {
                obj_id: obj.id.clone(),
                goodx: None,
                write: write_mode(&obj.objecttype),
                final_pos: obj.pos,
                reason: Some(format!("Cant find good isoline for {}", obj.id)),
            },
        })
        .collect()
}

/// Build per-isoline bags (ordered object-id lists) from placements, honoring the
/// write mode (`InsertFront` goes to the front of its bag).
pub fn bags_from_placements(
    isolines: &[IsoGeom],
    placements: &[Placement],
) -> HashMap<usize, Vec<String>> {
    let mut bags: HashMap<usize, Vec<String>> = HashMap::new();
    for iso in isolines {
        bags.entry(iso.index).or_default();
    }
    for p in placements {
        if let Some(goodx) = p.goodx {
            let bag = bags.entry(goodx).or_default();
            match p.write {
                WriteMode::Append => bag.push(p.obj_id.clone()),
                WriteMode::InsertFront => bag.insert(0, p.obj_id.clone()),
            }
        }
    }
    bags
}

// ---------------------------------------------------------------------------
// Fixture loading + deserialization (test-only)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use std::fs;
    use std::path::PathBuf;

    fn v3(a: [f64; 3]) -> Vec3 {
        Vec3::new(a[0] as f32, a[1] as f32, a[2] as f32)
    }

    #[derive(Deserialize)]
    struct FixtureFile {
        thresholds_pml: FixThresholds,
        cases: Vec<FixCase>,
    }

    #[derive(Deserialize)]
    struct FixThresholds {
        leg_parallel_max_deg: f64,
        samedirs_adir_ldir_min_deg: f64,
        weldtext_flip_showdir_deg: f64,
        u_dir: [f64; 3],
    }

    #[derive(Deserialize)]
    struct FixCase {
        id: String,
        isolines: Vec<FixIso>,
        objects: Vec<FixObj>,
        expect_bags: HashMap<String, Vec<String>>,
    }

    #[derive(Deserialize)]
    struct FixIso {
        index: usize,
        mems: Vec<String>,
        p0: [f64; 3],
        p1: [f64; 3],
        pipedir: [f64; 3],
        showdir: [f64; 3],
    }

    #[derive(Deserialize)]
    struct FixObj {
        id: String,
        objecttype: String,
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        name_is_atta: bool,
        #[serde(default)]
        elboname: Option<String>,
        #[serde(default)]
        ori_zdir: Option<[f64; 3]>,
        #[serde(default)]
        attapos: Option<[f64; 3]>,
        pos: [f64; 3],
        #[serde(default)]
        member_adir: Option<[f64; 3]>,
        #[serde(default)]
        member_ldir: Option<[f64; 3]>,
        #[serde(default)]
        movedir: Option<[f64; 3]>,
        #[serde(default)]
        movedis: f64,
        expect: FixExpect,
    }

    #[derive(Deserialize)]
    struct FixExpect {
        #[serde(default)]
        isoline: Option<usize>,
        #[serde(default)]
        write: Option<String>,
        #[serde(default)]
        skip: bool,
        #[serde(default)]
        reason_contains: Option<String>,
        #[serde(default)]
        final_pos: Option<[f64; 3]>,
    }

    fn load_p4() -> FixtureFile {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures/p4-put-into-isoline.json");
        let text =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        serde_json::from_str(&text).expect("parse p4-put-into-isoline.json")
    }

    fn to_iso(f: &FixIso) -> IsoGeom {
        IsoGeom {
            index: f.index,
            mems: f.mems.clone(),
            p0: v3(f.p0),
            p1: v3(f.p1),
            pipedir: v3(f.pipedir),
            showdir: v3(f.showdir),
        }
    }

    fn to_obj(f: &FixObj) -> DimObject {
        DimObject {
            id: f.id.clone(),
            objecttype: f.objecttype.clone(),
            name: f.name.clone(),
            name_is_atta: f.name_is_atta,
            elboname: f.elboname.clone(),
            ori_zdir: f.ori_zdir.map(v3),
            attapos: f.attapos.map(v3),
            pos: v3(f.pos),
            member_adir: f.member_adir.map(v3),
            member_ldir: f.member_ldir.map(v3),
            movedir: f.movedir.map(v3),
            movedis: f.movedis,
        }
    }

    fn run_case(id: &str) {
        let file = load_p4();
        let th = PutThresholds {
            leg_parallel_max_deg: file.thresholds_pml.leg_parallel_max_deg,
            samedirs_adir_ldir_min_deg: file.thresholds_pml.samedirs_adir_ldir_min_deg,
            weldtext_flip_showdir_deg: file.thresholds_pml.weldtext_flip_showdir_deg,
            u_dir: v3(file.thresholds_pml.u_dir),
        };
        let case = file.cases.iter().find(|c| c.id == id).unwrap_or_else(|| panic!("missing {id}"));
        let isolines: Vec<IsoGeom> = case.isolines.iter().map(to_iso).collect();
        let objects: Vec<DimObject> = case.objects.iter().map(to_obj).collect();

        let placements = put_into_isoline_pml(&isolines, &objects, &th);
        assert_eq!(placements.len(), objects.len());

        // Per-object expectations.
        for (obj, pl) in case.objects.iter().zip(placements.iter()) {
            if obj.expect.skip {
                assert_eq!(pl.goodx, None, "case {id} obj {}: expected skip", obj.id);
                if let Some(frag) = &obj.expect.reason_contains {
                    let reason = pl.reason.clone().unwrap_or_default();
                    assert!(
                        reason.contains(frag),
                        "case {id} obj {}: reason {reason:?} missing {frag:?}",
                        obj.id
                    );
                }
                continue;
            }
            let want = obj.expect.isoline.unwrap();
            assert_eq!(pl.goodx, Some(want), "case {id} obj {}: goodx", obj.id);

            if let Some(w) = &obj.expect.write {
                let want_write = match w.as_str() {
                    "append" => WriteMode::Append,
                    "insert_front" => WriteMode::InsertFront,
                    other => panic!("case {id} obj {}: bad write {other}", obj.id),
                };
                assert_eq!(pl.write, want_write, "case {id} obj {}: write", obj.id);
            }

            if let Some(fp) = obj.expect.final_pos {
                let want = v3(fp);
                assert!(
                    pl.final_pos.distance(want) < 1e-2,
                    "case {id} obj {}: final_pos {:?} vs {:?}",
                    obj.id,
                    pl.final_pos,
                    want
                );
            }
        }

        // Bag composition + order.
        let bags = bags_from_placements(&isolines, &placements);
        for (key, want) in &case.expect_bags {
            let idx: usize = key.parse().unwrap();
            let got = bags.get(&idx).cloned().unwrap_or_default();
            assert_eq!(&got, want, "case {id} bag {idx}: order/contents");
        }
    }

    #[test]
    fn bridge_legs_and_material() {
        run_case("bridge-legs-and-material");
    }

    #[test]
    fn weldtext_distance_and_slope_order() {
        run_case("weldtext-distance-and-slope-order");
    }
}
