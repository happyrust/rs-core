//! P3: polar neighbor obstacle registration (teaching implementation).
//! Spec: `teach/fixtures/p3-polar-neighbors.json` · lessons 46 / 61 / 64–66.

use glam::Vec3;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy)]
pub struct PolarThresholds {
    pub skip_one_angle_deg: f64,
    pub skip_one_dist_od_mult: f64,
    pub thick_diameter_od_mult: f64,
}

impl Default for PolarThresholds {
    fn default() -> Self {
        Self {
            skip_one_angle_deg: 160.0,
            skip_one_dist_od_mult: 15.0,
            thick_diameter_od_mult: 1.5,
        }
    }
}

#[derive(Debug, Clone)]
pub struct IsolineGeom {
    pub id: usize,
    pub od: f64,
    pub pipedir: Vec3,
    pub p0: Vec3,
    pub p1: Vec3,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObstacleLink {
    pub from: usize,
    pub diameter_scale: f64,
}

fn v3(a: [f64; 3]) -> Vec3 {
    Vec3::new(a[0] as f32, a[1] as f32, a[2] as f32)
}

fn angle_deg(a: Vec3, b: Vec3) -> f64 {
    let a = a.normalize_or_zero();
    let b = b.normalize_or_zero();
    if a.length_squared() < 1e-12 || b.length_squared() < 1e-12 {
        return 0.0;
    }
    let cos = a.dot(b).clamp(-1.0, 1.0);
    (cos.acos() as f64).to_degrees()
}

fn segment_distance(a0: Vec3, a1: Vec3, b0: Vec3, b1: Vec3) -> f64 {
    // Teaching approx: min distance between endpoints and midpoints.
    let samples_a = [a0, a1, (a0 + a1) * 0.5];
    let samples_b = [b0, b1, (b0 + b1) * 0.5];
    let mut best = f64::MAX;
    for pa in samples_a {
        for pb in samples_b {
            best = best.min(pa.distance(pb) as f64);
        }
    }
    best
}

fn push_unique(map: &mut HashMap<usize, Vec<ObstacleLink>>, into: usize, link: ObstacleLink) {
    let list = map.entry(into).or_default();
    if let Some(existing) = list.iter_mut().find(|e| e.from == link.from) {
        // Keep thicker scale if both apply.
        if link.diameter_scale > existing.diameter_scale {
            existing.diameter_scale = link.diameter_scale;
        }
    } else {
        list.push(link);
    }
}

/// Apply adjacent + skip-one neighbor obstacles. Does not change isoline count.
pub fn apply_polar_neighbors(
    isolines: &[IsolineGeom],
    th: &PolarThresholds,
) -> HashMap<usize, Vec<ObstacleLink>> {
    let mut out: HashMap<usize, Vec<ObstacleLink>> = HashMap::new();
    if isolines.is_empty() {
        return out;
    }

    // Adjacent (x > 1 in PML 1-based ⇒ index i >= 1)
    for i in 1..isolines.len() {
        let cur = &isolines[i];
        let pre = &isolines[i - 1];
        if cur.od <= 1e-9 || pre.od <= 1e-9 {
            continue;
        }
        // current height proxy: segment length; skip add-back if "height" <= 1 (mm)
        let cur_len = cur.p0.distance(cur.p1) as f64;
        push_unique(
            &mut out,
            cur.id,
            ObstacleLink {
                from: pre.id,
                diameter_scale: 1.0,
            },
        );
        if cur_len > 1.0 {
            push_unique(
                &mut out,
                pre.id,
                ObstacleLink {
                    from: cur.id,
                    diameter_scale: 1.0,
                },
            );
        }
    }

    // Skip-one near U-turn (x > 2 ⇒ i >= 2)
    for i in 2..isolines.len() {
        let cur = &isolines[i];
        let far = &isolines[i - 2];
        if cur.od <= 1e-9 || far.od <= 1e-9 {
            continue;
        }
        let ang = angle_deg(cur.pipedir, far.pipedir);
        let dist = segment_distance(cur.p0, cur.p1, far.p0, far.p1);
        let od_ref = cur.od.max(far.od);
        if ang > th.skip_one_angle_deg && dist < od_ref * th.skip_one_dist_od_mult {
            let scale = th.thick_diameter_od_mult;
            push_unique(
                &mut out,
                cur.id,
                ObstacleLink {
                    from: far.id,
                    diameter_scale: scale,
                },
            );
            push_unique(
                &mut out,
                far.id,
                ObstacleLink {
                    from: cur.id,
                    diameter_scale: scale,
                },
            );
        }
    }

    out
}

#[derive(Deserialize)]
struct FixtureFile {
    thresholds: FixtureThresholds,
    cases: Vec<FixtureCase>,
}

#[derive(Deserialize)]
struct FixtureThresholds {
    skip_one_angle_deg: f64,
    skip_one_dist_od_mult: f64,
    thick_diameter_od_mult: f64,
}

#[derive(Deserialize)]
struct FixtureCase {
    id: String,
    isolines: Vec<FixtureIso>,
    expect_obstacles: HashMap<String, Vec<FixtureObs>>,
}

#[derive(Deserialize)]
struct FixtureIso {
    id: usize,
    od: f64,
    pipedir: [f64; 3],
    p0: [f64; 3],
    p1: [f64; 3],
}

#[derive(Debug, Deserialize)]
struct FixtureObs {
    from: usize,
    diameter_scale: f64,
}

fn load_p3() -> FixtureFile {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures/p3-polar-neighbors.json");
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&text).expect("parse p3-polar-neighbors.json")
}

fn assert_case(id: &str) {
    let file = load_p3();
    let th = PolarThresholds {
        skip_one_angle_deg: file.thresholds.skip_one_angle_deg,
        skip_one_dist_od_mult: file.thresholds.skip_one_dist_od_mult,
        thick_diameter_od_mult: file.thresholds.thick_diameter_od_mult,
    };
    let case = file
        .cases
        .iter()
        .find(|c| c.id == id)
        .unwrap_or_else(|| panic!("missing {id}"));
    let geos: Vec<IsolineGeom> = case
        .isolines
        .iter()
        .map(|i| IsolineGeom {
            id: i.id,
            od: i.od,
            pipedir: v3(i.pipedir),
            p0: v3(i.p0),
            p1: v3(i.p1),
        })
        .collect();
    let got = apply_polar_neighbors(&geos, &th);

    for (key, expect_list) in &case.expect_obstacles {
        let idx: usize = key.parse().unwrap();
        let got_list = got.get(&idx).cloned().unwrap_or_default();
        assert_eq!(
            got_list.len(),
            expect_list.len(),
            "case {id} isoline {idx}: count mismatch got={got_list:?} expect={expect_list:?}"
        );
        for exp in expect_list {
            let g = got_list
                .iter()
                .find(|o| o.from == exp.from)
                .unwrap_or_else(|| panic!("case {id} isoline {idx}: missing from={}", exp.from));
            assert!(
                (g.diameter_scale - exp.diameter_scale).abs() < 1e-6,
                "case {id} isoline {idx} from={}: scale {} vs {}",
                exp.from,
                g.diameter_scale,
                exp.diameter_scale
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ortho_neighbors() {
        assert_case("ortho-neighbors");
    }

    #[test]
    fn u_turn_skip() {
        assert_case("u-turn-skip");
    }

    #[test]
    fn does_not_change_isoline_count() {
        let file = load_p3();
        let case = &file.cases[0];
        let n = case.isolines.len();
        let th = PolarThresholds::default();
        let geos: Vec<IsolineGeom> = case
            .isolines
            .iter()
            .map(|i| IsolineGeom {
                id: i.id,
                od: i.od,
                pipedir: v3(i.pipedir),
                p0: v3(i.p0),
                p1: v3(i.p1),
            })
            .collect();
        let _ = apply_polar_neighbors(&geos, &th);
        assert_eq!(geos.len(), n);
    }
}
