use crate::{RefU64, rs_surreal};
use glam::Vec3;
use std::sync::Arc;

#[tokio::test]
async fn test_group_cata_hash() -> anyhow::Result<()> {
    crate::init_test_surreal().await;
    let refnos: Vec<RefU64> = vec!["24383/73928".into()];

    let r = crate::query_multi_deep_children_filter_inst(
        refnos.clone(),
        vec!["BRAN".into(), "HANG".into()],
        false,
    )
        .await
        .unwrap();

    dbg!(&r);
    let branch_refnos: Vec<RefU64> = r.into_iter().collect();

    let map =
        crate::query_group_by_cata_hash(&branch_refnos)
            .await
            .unwrap_or_default();
    dbg!(map.get("15194523709157400402").map(|x| x.group_refnos.clone()));
    // dbg!(&map);

    // let group = rs_surreal::query_group_by_cata_hash(&refnos)
    //     .await
    //     .unwrap();
    // dbg!(&group);
    Ok(())
}
