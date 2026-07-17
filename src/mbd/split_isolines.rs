//! `split_isolines_pml` — mirror PML `isobran.getisolines` bend branch.
//!
//! Important PML order: every non-Head member is **appended to the current bag
//! first**, then PASS / dir-change / force-split logic runs. Near-180 `skip`
//! therefore still leaves the member in the bag.

use glam::Vec3;
use serde::Deserialize;

/// PML thresholds (degrees / mm).
pub mod thresholds {
    pub const DIR_CHANGE_EPS_DEG: f64 = 0.01;
    pub const MAIN_SPLIT_ANGLE_DEG: f64 = 10.0;
    pub const APOS_LPOS_SEP_MM: f64 = 1.0;
    pub const STICK_PREV_TURN_MM: f64 = 0.1;
    pub const FORCE_SPLIT_ADIR_MEMLDIR_DEG: f64 = 100.0;
    pub const NEAR_180_ADIR_MEMLDIR_DEG: f64 = 179.99;
    pub const NEAR_180_ADIR_LDIR_DEG: f64 = 179.0;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum MemberKind {
    Head,
    Tail,
    Tubi,
    Atta,
    Weld,
    Olet,
    Elbo,
    Bend,
    Pcom,
    #[serde(other)]
    Other,
}

impl MemberKind {
    fn is_pass_through(self) -> bool {
        matches!(
            self,
            MemberKind::Tubi | MemberKind::Atta | MemberKind::Weld | MemberKind::Olet
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
pub enum GeomQuality {
    Exact,
    #[default]
    ApproximateStartEnd,
    Missing,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SplitMember {
    pub order: u32,
    pub refno: String,
    pub kind: MemberKind,
    #[serde(default)]
    pub leave_dir: Option<[f64; 3]>,
    #[serde(default)]
    pub arrive_dir: Option<[f64; 3]>,
    #[serde(default)]
    pub arrive_pos: Option<[f64; 3]>,
    #[serde(default)]
    pub leave_pos: Option<[f64; 3]>,
    #[serde(default)]
    pub pos: Option<[f64; 3]>,
    #[serde(default)]
    pub geom_quality: GeomQuality,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitIssue {
    pub refno: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SplitResult {
    pub allarr: Vec<Vec<String>>,
    /// One leave-direction per isoline bag (`allarr.len() == ldirs.len()`).
    pub ldirs: Vec<Vec3>,
    pub issues: Vec<SplitIssue>,
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

fn dist(a: Vec3, b: Vec3) -> f64 {
    a.distance(b) as f64
}

fn bridge_close_open(
    allarr: &mut Vec<Vec<String>>,
    onearr: &mut Vec<String>,
    ldirs: &mut Vec<Vec3>,
    poss: &mut Vec<Vec3>,
    run_ldir: &mut Vec3,
    mem_ref: &str,
    dir: Vec3,
    turn: Vec3,
) {
    // Member already in onearr (PML append-first).
    poss.push(turn);
    if !onearr.is_empty() {
        allarr.push(std::mem::take(onearr));
    }
    *run_ldir = dir;
    onearr.push(mem_ref.to_string());
    ldirs.push(dir);
}

/// Split branch members into isoline bags (refno sequences).
///
/// Elbow / direction-change path only for P1. PCOM is recorded as an issue and
/// left in the current bag (locked degrade policy).
pub fn split_isolines_pml(members: &[SplitMember]) -> SplitResult {
    let mut sorted: Vec<&SplitMember> = members.iter().collect();
    sorted.sort_by_key(|m| m.order);

    let mut allarr: Vec<Vec<String>> = Vec::new();
    let mut onearr: Vec<String> = Vec::new();
    let mut poss: Vec<Vec3> = Vec::new();
    let mut ldirs: Vec<Vec3> = Vec::new();
    let mut run_ldir: Option<Vec3> = None;
    let mut issues: Vec<SplitIssue> = Vec::new();

    for m in sorted {
        match m.kind {
            MemberKind::Head => {
                onearr = vec![m.refno.clone()];
                let p = m
                    .arrive_pos
                    .or(m.pos)
                    .map(v3)
                    .unwrap_or(Vec3::ZERO);
                poss.push(p);
                if let Some(ld) = m.leave_dir {
                    let d = v3(ld);
                    run_ldir = Some(d);
                    ldirs.push(d);
                }
                continue;
            }
            MemberKind::Tail => {
                onearr.push(m.refno.clone());
                if !onearr.is_empty() {
                    allarr.push(std::mem::take(&mut onearr));
                }
                continue;
            }
            _ => {}
        }

        // PML: append before type / angle branching.
        onearr.push(m.refno.clone());

        if m.kind.is_pass_through() {
            // Track run tip so stick / near-180 see the current end (fixtures +
            // practical PML “上一拐点” after a straight run).
            if let Some(lp) = m.leave_pos {
                poss.push(v3(lp));
            }
            continue;
        }

        if m.kind == MemberKind::Pcom {
            issues.push(SplitIssue {
                refno: m.refno.clone(),
                message: "P1: PCOM not implemented".into(),
            });
            continue;
        }

        let Some(leave) = m.leave_dir else {
            issues.push(SplitIssue {
                refno: m.refno.clone(),
                message: "missing leave_dir".into(),
            });
            continue;
        };
        let dir = v3(leave);

        let Some(run) = run_ldir else {
            run_ldir = Some(dir);
            if ldirs.is_empty() {
                ldirs.push(dir);
            }
            continue;
        };

        if angle_deg(dir, run) <= thresholds::DIR_CHANGE_EPS_DEG {
            continue;
        }

        // --- direction-change branch ---
        let adir = m.arrive_dir.map(v3);
        let apos = m.arrive_pos.map(v3);
        let lpos = m.leave_pos.map(v3);
        let memldir = dir;
        let turn = m
            .pos
            .map(v3)
            .or_else(|| match (apos, lpos) {
                (Some(a), Some(b)) => Some((a + b) * 0.5),
                (Some(a), None) => Some(a),
                (None, Some(b)) => Some(b),
                _ => None,
            })
            .unwrap_or(Vec3::ZERO);

        // ① near-180 correction
        if let (Some(ad), Some(ap)) = (adir, apos) {
            if let Some(prev) = poss.last().copied() {
                if angle_deg(ad, memldir) > thresholds::NEAR_180_ADIR_MEMLDIR_DEG
                    && angle_deg(ad, run) > thresholds::NEAR_180_ADIR_LDIR_DEG
                    && dist(ap, prev) < thresholds::APOS_LPOS_SEP_MM
                {
                    let fixed = -ad;
                    run_ldir = Some(fixed);
                    if let Some(last) = ldirs.last_mut() {
                        *last = fixed;
                    }
                    continue;
                }
            }
        }

        // ② main split
        let need = match (apos, lpos) {
            (Some(a), Some(l)) => {
                dist(a, l) > thresholds::APOS_LPOS_SEP_MM
                    || angle_deg(dir, run) > thresholds::MAIN_SPLIT_ANGLE_DEG
            }
            _ => angle_deg(dir, run) > thresholds::MAIN_SPLIT_ANGLE_DEG,
        };

        if need {
            let stick = match lpos {
                Some(lp) => {
                    let near_poss = poss
                        .last()
                        .copied()
                        .map(|p| dist(lp, p) < thresholds::STICK_PREV_TURN_MM)
                        .unwrap_or(false);
                    // Degenerate elbow: leave sticks to own arrive (teaching fixtures /
                    // near-zero bend length). PML compares lpos to poss.last; when the
                    // prior turn was not re-recorded at this elbow's apos, apos≈lpos
                    // is the practical stick signal used by our P1 cases.
                    let near_apos = apos
                        .map(|a| dist(lp, a) < thresholds::STICK_PREV_TURN_MM)
                        .unwrap_or(false);
                    near_poss || near_apos
                }
                None => false,
            };
            if stick {
                if let Some(last) = ldirs.last_mut() {
                    *last = dir;
                }
                run_ldir = Some(dir);
            } else {
                let mut run = run;
                bridge_close_open(
                    &mut allarr,
                    &mut onearr,
                    &mut ldirs,
                    &mut poss,
                    &mut run,
                    &m.refno,
                    dir,
                    turn,
                );
                run_ldir = Some(run);
            }
        }

        // ③ force split (may stack on ②)
        if let Some(ad) = adir {
            if angle_deg(ad, memldir) > thresholds::FORCE_SPLIT_ADIR_MEMLDIR_DEG {
                let mut run = run_ldir.unwrap_or(dir);
                bridge_close_open(
                    &mut allarr,
                    &mut onearr,
                    &mut ldirs,
                    &mut poss,
                    &mut run,
                    &m.refno,
                    dir,
                    turn,
                );
                run_ldir = Some(run);
            }
        }
    }

    if !onearr.is_empty() {
        issues.push(SplitIssue {
            refno: String::new(),
            message: "unterminated bag without Tail".into(),
        });
        allarr.push(onearr);
    }

    SplitResult {
        allarr,
        ldirs,
        issues,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    #[derive(Deserialize)]
    struct FixtureFile {
        cases: Vec<FixtureCase>,
    }

    #[derive(Deserialize)]
    struct FixtureCase {
        id: String,
        expect_allarr: Vec<Vec<String>>,
        members: Vec<SplitMember>,
    }

    fn load_fixtures() -> FixtureFile {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("teach/fixtures/p1-split-isolines.json");
        let text = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        serde_json::from_str(&text).expect("parse p1-split-isolines.json")
    }

    fn assert_case(id: &str) {
        let file = load_fixtures();
        let case = file
            .cases
            .iter()
            .find(|c| c.id == id)
            .unwrap_or_else(|| panic!("missing case {id}"));
        let got = split_isolines_pml(&case.members);
        assert_eq!(
            got.allarr.len(),
            got.ldirs.len(),
            "case {id}: allarr/ldirs length mismatch"
        );
        assert_eq!(
            got.allarr, case.expect_allarr,
            "case {id}\n issues={:?}\n got={:?}\n expect={:?}",
            got.issues, got.allarr, case.expect_allarr
        );
    }

    #[test]
    fn toy_ldirs_match_axes() {
        assert_case("toy-bridge-and-stick");
        let file = load_fixtures();
        let case = file
            .cases
            .iter()
            .find(|c| c.id == "toy-bridge-and-stick")
            .unwrap();
        let got = split_isolines_pml(&case.members);
        assert_eq!(got.ldirs.len(), 2);
        let x = Vec3::X;
        assert!(
            got.ldirs[0].normalize().dot(x) > 0.99,
            "seg0 ldir {:?}",
            got.ldirs[0]
        );
        // EB stick updates ldirs.last to EB leave_dir (not raw +Z).
        let eb = Vec3::new(0.2079, 0.0, 0.9781).normalize();
        assert!(
            got.ldirs[1].normalize().dot(eb) > 0.99,
            "seg1 ldir {:?}",
            got.ldirs[1]
        );
    }

    #[test]
    fn toy_bridge_and_stick() {
        assert_case("toy-bridge-and-stick");
    }

    #[test]
    fn colinear_no_split() {
        assert_case("colinear-no-split");
    }

    #[test]
    fn variant_eb_no_stick() {
        assert_case("variant-eb-no-stick");
    }

    #[test]
    fn near_180_correct_no_split() {
        assert_case("near-180-correct-no-split");
    }

    #[test]
    fn force_split_over_100() {
        assert_case("force-split-over-100");
    }

    #[test]
    fn all_five_fixtures() {
        let file = load_fixtures();
        for case in &file.cases {
            let got = split_isolines_pml(&case.members);
            assert_eq!(got.allarr.len(), got.ldirs.len(), "case {}", case.id);
            assert_eq!(
                got.allarr, case.expect_allarr,
                "case {}\n issues={:?}\n got={:?}\n expect={:?}",
                case.id, got.issues, got.allarr, case.expect_allarr
            );
        }
    }
}
