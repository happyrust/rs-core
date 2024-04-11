use crate::rs_surreal;



#[tokio::test]
async fn test_get_world() {
    super::init_test_surreal().await;
    let mdb = String::from("/ALL");
    let result = rs_surreal::get_world(mdb).await;
    // Assert
    assert!(result.is_ok());
    let pe = result.unwrap();
    dbg!(&pe);
    assert!(pe.is_some());
    // Add additional assertions based on the expected behavior of your function
}

#[tokio::test]
async fn test_get_world_sites() {
    super::init_test_surreal().await;
    let mdb = String::from("/ALL");
    // let result = rs_surreal::get_mdb_world_site_pes(mdb.clone(), crate::DBType::DESI).await.unwrap();
    // Assert
    // assert!(result.is_ok());
    // let pes = result.unwrap();
    // dbg!(&result);
    let result = rs_surreal::get_mdb_world_site_ele_nodes(mdb, crate::DBType::DESI).await.unwrap();
    dbg!(&result);
    // assert!(!pes.is_empty());
    // Add additional assertions based on the expected behavior of your function
}