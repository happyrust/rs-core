use crate::rs_surreal;

#[tokio::test]
async fn test_get_world() {
    crate::init_test_surreal().await;
    let mdb = crate::get_db_option().mdb_name.clone();
    println!("Testing with MDB: {}", mdb);
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

    // 先测试一个简单的数据库连接
    println!("Testing simple database connection...");
    match crate::rs_surreal::test_simple_query().await {
        Ok(_) => {
            println!("✅ Simple connection test passed");
        }
        Err(e) => {
            println!("❌ Simple connection test failed: {:?}", e);
        }
    }

    // 使用配置文件中的正确 MDB 名称
    let mdb = crate::get_db_option().mdb_name.clone();
    println!("Testing with MDB: {}", mdb);

    // 分别测试每个函数以确定哪个失败
    println!("1. Testing get_world...");
    let world_result = rs_surreal::get_world(mdb.clone()).await;
    match world_result {
        Ok(world) => {
            let world = world.unwrap();
            println!("✅ get_world succeeded: {:?}", world.refno());
            assert_eq!(&world.noun, "WORL");
        }
        Err(e) => {
            println!("❌ get_world failed: {:?}", e);
            panic!("get_world failed: {:?}", e);
        }
    }

    println!("2. Testing query_mdb_db_nums...");
    let dbnums_result = rs_surreal::query_mdb_db_nums(Some(mdb.clone()), crate::DBType::DESI).await;
    match dbnums_result {
        Ok(result) => {
            println!("✅ query_mdb_db_nums succeeded: {:?}", result);
        }
        Err(e) => {
            println!("❌ query_mdb_db_nums failed: {:?}", e);
            panic!("query_mdb_db_nums failed: {:?}", e);
        }
    }

    println!("3. Testing get_mdb_world_site_pes...");
    let sites_result = rs_surreal::get_mdb_world_site_pes(mdb.clone(), crate::DBType::DESI).await;
    match sites_result {
        Ok(result) => {
            println!(
                "✅ get_mdb_world_site_pes succeeded: {} sites",
                result.len()
            );
        }
        Err(e) => {
            println!("❌ get_mdb_world_site_pes failed: {:?}", e);
            panic!("get_mdb_world_site_pes failed: {:?}", e);
        }
    }

    println!("4. Testing get_mdb_world_site_ele_nodes...");
    let nodes_result = rs_surreal::get_mdb_world_site_ele_nodes(mdb, crate::DBType::DESI).await;
    match nodes_result {
        Ok(result) => {
            println!(
                "✅ get_mdb_world_site_ele_nodes succeeded: {} nodes",
                result.len()
            );
        }
        Err(e) => {
            println!("❌ get_mdb_world_site_ele_nodes failed: {:?}", e);
            panic!("get_mdb_world_site_ele_nodes failed: {:?}", e);
        }
    }
}

#[tokio::test]
async fn test_get_site_pes_by_dbnum() {
    crate::init_test_surreal().await;

    // 首先查询可用的 dbnum
    let dbnums = rs_surreal::query_mdb_db_nums(Some("/ALL".to_string()), crate::DBType::DESI)
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
