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

#[tokio::test]
async fn test_branch_38293_8916_segments() -> anyhow::Result<()> {
    crate::init_test_surreal().await;

    let branch_refno = RefnoEnum::from("38293/8916");
    let segments = PipelineQueryService::fetch_branch_segments(branch_refno.clone()).await?;

    assert!(!segments.is_empty(), "BRAN 38293/8916 应该包含至少一个管段");

    // 验证所有管段都有有效的 main_span
    let segments_with_spans: Vec<_> = segments
        .iter()
        .filter(|seg| seg.main_span().is_some())
        .collect();

    assert!(
        !segments_with_spans.is_empty(),
        "至少应有一个管段具备有效的主跨度（arrive/leave）"
    );

    // 验证端口坐标数据
    for segment in &segments {
        if let Some(span) = segment.main_span() {
            assert!(span.start.world_pos.is_finite(), "起点坐标应为有效值");
            assert!(span.end.world_pos.is_finite(), "终点坐标应为有效值");
            assert!(span.length > 0.0, "长度应大于0");
        }
    }

    println!("✅ BRAN 38293/8916 验证通过:");
    println!("   - 管段总数: {}", segments.len());
    println!("   - 有主跨度的管段: {}", segments_with_spans.len());

    // 打印详细信息用于调试
    for (idx, segment) in segments.iter().take(5).enumerate() {
        println!(
            "   [{}] {} ({:?}) 长度: {:.1}mm",
            idx + 1,
            segment.refno.to_string(),
            segment.noun,
            segment.length
        );
    }

    Ok(())
}
