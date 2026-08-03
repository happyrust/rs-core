# Teaching notes

- Mission locked (timeout auto): port/verify V2 isoline split vs PML; short quiz lessons.
- Prior exposure: user already skimmed pipeline (isobran → isoline → isoDim → lindim) and asked for getisolines bend rules deep-dive.
- Language: Chinese for lesson body; keep PML identifiers in English as in source.
- Prefer opening lesson HTML in browser after each lesson.

## Progress (2026-07-18)

- Lessons **1–71**
- **P1+P2 sketch+P3+P4**: `teach/p1_split` **14/14** tests
  - split_isolines + ldirs (7)
  - wiring_sketch (use_pml_split) (2)
  - polar_neighbors (ortho + u-turn) (3)
  - put_into_isoline (bridge legs/material + weldtext/slope order) (2)
- Split logic lives ONLY in teach crate; workspace `src/` has NO `mbd/split_isolines.rs` (main chain not wired)
- Lesson 68 = β overview: three phases landed offline, **main chain not wired** (no real extract calls the crate; no `use_pml_split` in `src`)
- P4 draft: `teach/fixtures/p4-put-into-isoline.json` — putIntoIsoLine 归段 2 cases（桥接双候选 leg/材料决胜 + 焊缝字距离/movedis 翻转/insert(1) 次序），字段与期望 bags 已写清，待 Rust 消费
- Lesson 69 = **P4 归段 fixture 导读**: walks both cases of `p4-put-into-isoline.json` (path A/B, 焊缝字距离 vs 材料更水平决胜, insert(1) 次序, skip on no-candidate). Reading-only; no `put_into_isoline_pml` Rust yet, not wired.
- Lesson 70 = **P4 归段开码检查单**: 13-item preflight for `put_into_isoline_pml` in teach crate.
- Lesson 71 = **P4 归段已落地**: `teach/p1_split/src/put_into_isoline.rs` implements `put_into_isoline_pml` (path A/B分流, samedirs, weldtext距离 vs 更水平 tie-break, movedis flip+offset, insert(1)次序, skip). Both `p4-put-into-isoline.json` cases green → crate **14/14**. Test-only fixture plumbing under `#[cfg(test)]`. Still offline, **not wired to main chain**.

## Next recommended

1. Wire P1→P4→P3 into real Layout when V2 extract path exists (still the only "make it affect real output" step).
2. Or continue tutorials (e.g. PCOM handling, or a putIntoIsoLine getobjects priority lesson).
