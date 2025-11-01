use crate::pdms_types::PdmsGenericType;
use crate::pipeline::{PipelineQueryService, PipelineSegmentRecord};
use crate::{RefnoEnum, query_filter_all_bran_hangs};

#[tokio::test]
async fn pipeline_fetch_branch_segments_provides_spans() -> anyhow::Result<()> {
    crate::init_test_surreal().await;

    // 选择一个已知包含分支的节点，遍历其 BRAN 以找到实际有管段的分支
    let seed = RefnoEnum::from("24383/73928");
    let branches = query_filter_all_bran_hangs(seed).await?;
    assert!(
        !branches.is_empty(),
        "预期从测试数据中获取到至少一个 BRAN/HANG"
    );

    let mut chosen = None;
    for branch in branches {
        let segments: Vec<PipelineSegmentRecord> =
            PipelineQueryService::fetch_branch_segments(branch.clone()).await?;
        if !segments.is_empty() {
            chosen = Some((branch, segments));
            break;
        }
    }

    let (branch, segments) = chosen.expect("测试数据中应至少存在一个带有管段的 BRAN");
    assert!(
        segments.iter().all(|seg| seg.branch == branch),
        "所有段的 branch 字段都应该回指同一个 BRAN"
    );
    assert!(
        segments
            .iter()
            .any(|segment| segment.is_kind(PdmsGenericType::PIPE)),
        "至少应包含一个 PIPE 类型的管段"
    );
    assert!(
        segments.iter().any(|segment| segment.main_span().is_some()),
        "至少一个管段需要具备有效的主跨度"
    );

    let total_port_count: usize = segments
        .iter()
        .map(|segment| segment.all_ports().count())
        .sum();
    assert!(
        total_port_count > 0,
        "聚合后应存在至少一个端口（用于后续标注）"
    );

    Ok(())
}
