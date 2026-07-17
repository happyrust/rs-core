# Teaching notes

- Mission locked (timeout auto): port/verify V2 isoline split vs PML; short quiz lessons.
- Prior exposure: user already skimmed pipeline (isobran → isoline → isoDim → lindim) and asked for getisolines bend rules deep-dive.
- Language: Chinese for lesson body; keep PML identifiers in English as in source.
- Prefer opening lesson HTML in browser after each lesson.

## Progress (2026-07-17)

- Lessons **1–67**
- **P1+P2 sketch+P3**: `teach/p1_split` **12/12** tests
  - split_isolines + ldirs
  - wiring_sketch (use_pml_split)
  - polar_neighbors (ortho + u-turn)
- Also mirrored split in `src/mbd/split_isolines.rs`

## Next recommended

1. Wire into real Layout when V2 extract path exists.
2. Or continue tutorials / putIntoIsoLine fixtures.
