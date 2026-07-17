//! P1: PML `getisolines` elbow-path split (teaching alignment).
//!
//! Pure function — no Polar / putIntoIsoLine / lindim.
//! Spec: `teach/reference/split-isolines-pseudocode.html`
//! Fixtures: `teach/fixtures/p1-split-isolines.json`

pub mod split_isolines;

pub use split_isolines::{
    split_isolines_pml, GeomQuality, MemberKind, SplitIssue, SplitMember, SplitResult,
};
