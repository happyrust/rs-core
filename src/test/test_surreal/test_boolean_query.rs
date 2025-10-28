use crate::types::RefnoEnum;

#[tokio::test]
async fn test_query_cata_neg_boolean_groups() -> anyhow::Result<()> {
    crate::init_test_surreal().await;

    // Empty refnos should default to querying full inst_relate
    let result = crate::rs_surreal::query_cata_neg_boolean_groups(&[], false).await;
    assert!(result.is_ok());
    Ok(())
}

#[tokio::test]
async fn test_query_geom_mesh_data() -> anyhow::Result<()> {
    crate::init_test_surreal().await;

    // Use a valid refno seen in other tests; allow empty geom_refnos (should return Ok with empty results)
    let refno = RefnoEnum::from("24383/101165");
    let result = crate::rs_surreal::query_geom_mesh_data(refno, &[]).await;
    assert!(result.is_ok());
    Ok(())
}

#[tokio::test]
async fn test_query_manifold_boolean_operations() -> anyhow::Result<()> {
    crate::init_test_surreal().await;

    // Choose an existing refno from repository tests
    let refno = RefnoEnum::from("24383/101155");
    let result = crate::rs_surreal::query_manifold_boolean_operations(refno).await;
    assert!(result.is_ok());
    Ok(())
}

#[tokio::test]
async fn test_query_occ_boolean_operations() -> anyhow::Result<()> {
    crate::init_test_surreal().await;

    // Empty refnos should fallback to querying all inst_relate
    let result = crate::rs_surreal::query_occ_boolean_operations(&[], false).await;
    assert!(result.is_ok());
    Ok(())
}

#[tokio::test]
async fn test_query_simple_cata_negative_bool() -> anyhow::Result<()> {
    crate::init_test_surreal().await;

    let result = crate::rs_surreal::query_simple_cata_negative_bool().await;
    assert!(result.is_ok());
    Ok(())
}


