use std::env;
use aios_core::RefnoEnum;
use aios_core::pipeline::PipelineQueryService;
use aios_core::{get_named_attmap, RefU64};
use aios_core::rs_surreal::point::query_arrive_leave_points;
use aios_core::rs_surreal::geom::query_refnos_point_map;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    aios_core::init_test_surreal().await;

    // Accept BRAN refno from CLI or fall back to default
    let args: Vec<String> = env::args().collect();
    let bran_arg = args.get(1).cloned().unwrap_or_else(|| "21491/16521".to_string());
    let branch = RefnoEnum::from(bran_arg.as_str());

    println!("🔎 Fetching annotation segments for BRAN: {}", bran_arg);

    let segments: Vec<aios_core::pipeline::PipelineSegmentRecord> =
        PipelineQueryService::fetch_branch_segments(branch.clone()).await?;

    println!("Segments total: {}", segments.len());
    for (idx, seg) in segments.iter().enumerate() {
        let main_span = seg.main_span();
        let ports_count = seg.all_ports().count();
        println!(
            "#{} refno={} noun={:?} ports={} main_span={} extra_count={}",
            idx + 1,
            seg.refno.to_string(),
            seg.noun,
            ports_count,
            main_span.is_some(),
            seg.extra_ports.len()
        );
        if let Some(span) = main_span {
            println!(
                "  - main: start(role={:?}, no={}) -> end(role={:?}, no={}), length={:.3}, straight={:.3}",
                span.start.role, span.start.number, span.end.role, span.end.number, span.length, span.straight_length
            );
        }
        for e in &seg.extra_ports {
            println!("  - extra: role={:?}, no={}, pos=({:.3},{:.3},{:.3})", e.role, e.number, e.world_pos.x, e.world_pos.y, e.world_pos.z);
        }
    }

    // ---- Diagnostics: raw data sources ----
    let r: RefU64 = branch.refno();
    // 1) arrive/leave pairs
    let pairs = query_arrive_leave_points([r].iter(), false).await?;
    println!("arrive_leave_pairs count: {}", pairs.len());
    if let Some(pair) = pairs.get(&r) {
        println!("ARRI no={}, LEAV no={}", pair[0].number, pair[1].number);
    } else {
        println!("no arrive/leave pair found for {}", bran_arg);
    }
    // 2) refnos point map
    let pmap = query_refnos_point_map(vec![branch.clone()]).await?;
    println!("point_map has refnos: {}", pmap.len());
    if let Some(inst_map) = pmap.get(&branch) {
        println!("ptset_map entries: {}", inst_map.ptset_map.len());
    } else {
        println!("no point_map entry for {}", bran_arg);
    }
    // 3) ARRI/LEAV named attributes
    let attrs = get_named_attmap(branch.clone()).await?;
    let arri = attrs.get("ARRI").map(|v| format!("{:?}", v)).unwrap_or("<none>".to_string());
    let leav = attrs.get("LEAV").map(|v| format!("{:?}", v)).unwrap_or("<none>".to_string());
    println!("attrs: ARRI={}, LEAV={}", arri, leav);

    Ok(())
}


