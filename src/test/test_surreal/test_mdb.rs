use crate::rs_surreal;

#[tokio::test]
async fn test_get_world() {
    crate::init_test_surreal().await;
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
    crate::init_test_surreal().await;
    let mdb = String::from("/ALL");
    let world = rs_surreal::get_world(mdb.clone()).await.unwrap().unwrap();
    assert_eq!(&world.noun, "WORL");
    let result = rs_surreal::query_mdb_db_nums(crate::DBType::DESI)
        .await
        .unwrap();
    dbg!(&result);
    let result = rs_surreal::get_mdb_world_site_pes(mdb.clone(), crate::DBType::DESI)
        .await
        .unwrap();
    dbg!(&result);
    let result = rs_surreal::get_mdb_world_site_ele_nodes(mdb, crate::DBType::DESI)
        .await
        .unwrap();
    dbg!(&result);
    // assert!(!pes.is_empty());
    // Add additional assertions based on the expected behavior of your function
}

#[tokio::test]
async fn test_get_site_pes_by_dbnum() {
    crate::init_test_surreal().await;

    // 首先查询可用的 dbnum
    let dbnums = rs_surreal::query_mdb_db_nums(crate::DBType::DESI)
        .await
        .unwrap();

    println!("Available dbnums: {:?}", dbnums);

    if dbnums.is_empty() {
        println!("No dbnums found, skipping test");
        return;
    }

    // 测试第一个 dbnum
    let test_dbnum = dbnums[0];
    println!("\nTesting dbnum: {}", test_dbnum);

    let result = rs_surreal::get_site_pes_by_dbnum(test_dbnum).await;
    assert!(
        result.is_ok(),
        "Failed to query sites by dbnum: {:?}",
        result.err()
    );

    let sites = result.unwrap();
    println!(
        "Found {} SITE elements for dbnum {}",
        sites.len(),
        test_dbnum
    );

    // 如果找到 SITE，验证它们的属性
    for (i, site) in sites.iter().enumerate() {
        println!(
            "  SITE {}: name='{}', noun='{}', dbnum={}, refno={}",
            i + 1,
            site.name,
            site.noun,
            site.dbnum,
            site.refno()
        );
        assert_eq!(site.noun, "SITE", "Expected noun to be 'SITE'");
        assert_eq!(site.dbnum, test_dbnum as i32, "Expected dbnum to match");
        assert!(!site.deleted, "Expected deleted to be false");
    }

    // 测试多个 dbnum（如果有的话）
    if dbnums.len() > 1 {
        println!("\nTesting additional dbnums:");
        for dbnum in dbnums.iter().skip(1).take(2) {
            let result = rs_surreal::get_site_pes_by_dbnum(*dbnum).await;
            assert!(result.is_ok(), "Failed to query sites for dbnum {}", dbnum);
            let sites = result.unwrap();
            println!("  dbnum {}: {} SITE elements", dbnum, sites.len());
        }
    }
}
