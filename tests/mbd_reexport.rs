//! 验证 aios_core::mbd::* 路径在拆分后仍然可用（下游兼容性测试）。
//!
//! 覆盖 plant-model-gen/src/web_api/mbd_pipe_api.rs 使用的所有符号。

#[test]
fn mbd_reexport_paths_are_stable() {
    use aios_core::mbd::iso_extras::{BendInput, SlopeInput, TagInput, WeldInput};
    use aios_core::mbd::iso_params::{BranchContext, IsoParams, SegmentInput};
    use aios_core::mbd::{
        BranchCalculator, BranchLayoutMode, LayoutRequest, LayoutResult, SolveBranchInput,
    };
    use glam::Vec3;

    let ctx = BranchContext::for_test("reexport_check");
    let params = IsoParams::default();
    let dims = vec![SegmentInput {
        id: "dim:reexport".into(),
        kind: "segment".into(),
        start: Vec3::ZERO,
        end: Vec3::new(1000.0, 0.0, 0.0),
        pipe_dir: Vec3::X,
        od: 100.0,
        text: "1000".into(),
        isoline_index: None,
    }];

    let sections = BranchCalculator::solve_branch(SolveBranchInput::linear_only(
        &ctx, &params, &dims,
    ));

    let request = LayoutRequest {
        mode: BranchLayoutMode::LayoutFirst,
        ..LayoutRequest::default()
    };
    let result: LayoutResult = BranchCalculator::assemble_prelaid_out(&request, sections);
    assert_eq!(result.stats.linear_dims_count, 1);

    let _unused = (
        std::marker::PhantomData::<BendInput>,
        std::marker::PhantomData::<SlopeInput>,
        std::marker::PhantomData::<TagInput>,
        std::marker::PhantomData::<WeldInput>,
    );
}
