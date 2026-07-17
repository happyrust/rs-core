//! P2 wiring sketch — NOT wired into aios_core extract path yet.
//!
//! Purpose: show the `use_pml_split` bypass shape for teaching (lesson 62–63).
//! Compile-checked as part of `p1_split` tests via `#[cfg(test)]` exercises below.

use crate::{split_isolines_pml, SplitIssue, SplitMember, SplitResult};
use glam::Vec3;

/// Feature flag container (sketch).
#[derive(Debug, Clone, Default)]
pub struct BranchCalcConfigSketch {
    /// When false, callers should use legacy extract only.
    pub use_pml_split: bool,
}

/// Stand-in for layout isoline output.
#[derive(Debug, Clone, PartialEq)]
pub struct IsolineInfoSketch {
    pub mem_refnos: Vec<String>,
    pub leave_dir: Vec3,
}

/// Map layout members → split input. Sketch: identity.
pub fn map_to_split_members(members: &[SplitMember]) -> Result<Vec<SplitMember>, Vec<SplitIssue>> {
    if members.iter().any(|m| m.geom_quality == crate::GeomQuality::Missing) {
        return Err(vec![SplitIssue {
            refno: String::new(),
            message: "GeomQuality::Missing — abort P1".into(),
        }]);
    }
    Ok(members.to_vec())
}

/// Bags + ldirs → sketch isolines.
pub fn bags_to_isolines(result: &SplitResult) -> Vec<IsolineInfoSketch> {
    result
        .allarr
        .iter()
        .zip(result.ldirs.iter())
        .map(|(bag, dir)| IsolineInfoSketch {
            mem_refnos: bag.clone(),
            leave_dir: *dir,
        })
        .collect()
}

/// P1 attempt; Err means "fall back to legacy".
pub fn try_pml_split(members: &[SplitMember]) -> Result<Vec<IsolineInfoSketch>, Vec<SplitIssue>> {
    let mapped = map_to_split_members(members)?;
    let r = split_isolines_pml(&mapped);
    // Soft issues (e.g. unterminated) still produce bags; only hard map errors abort.
    Ok(bags_to_isolines(&r))
}

/// Facade matching lesson 62.
pub fn extract_isolines_sketch(
    members: &[SplitMember],
    cfg: &BranchCalcConfigSketch,
    legacy: impl FnOnce(&[SplitMember]) -> Vec<IsolineInfoSketch>,
) -> Vec<IsolineInfoSketch> {
    if cfg.use_pml_split {
        if let Ok(v) = try_pml_split(members) {
            return v;
        }
        // fall through
    }
    legacy(members)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture_load::load_case;

    #[test]
    fn flag_off_uses_legacy() {
        let case = load_case("colinear-no-split");
        let cfg = BranchCalcConfigSketch {
            use_pml_split: false,
        };
        let out = extract_isolines_sketch(&case.members, &cfg, |_| vec![IsolineInfoSketch {
            mem_refnos: vec!["LEGACY".into()],
            leave_dir: Vec3::X,
        }]);
        assert_eq!(out[0].mem_refnos, vec!["LEGACY"]);
    }

    #[test]
    fn flag_on_matches_fixture_bags() {
        let case = load_case("toy-bridge-and-stick");
        let cfg = BranchCalcConfigSketch {
            use_pml_split: true,
        };
        let out = extract_isolines_sketch(&case.members, &cfg, |_| panic!("should not legacy"));
        let bags: Vec<Vec<String>> = out.iter().map(|i| i.mem_refnos.clone()).collect();
        assert_eq!(bags, case.expect_allarr);
        assert_eq!(out.len(), 2);
    }
}
