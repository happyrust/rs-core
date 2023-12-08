use crate::SUL_DB;
use crate::{NamedAttrMap, RefU64, rs_surreal};
use glam::Vec3;
use std::sync::Arc;

#[tokio::test]
async fn test_query_pe_by_refno() -> anyhow::Result<()> {
    super::init_test_surreal().await;
    // let pe = rs_surreal::get_pe("17496/254421".into()).await.unwrap();
    let refno = "13292_0".into();
    let pe = rs_surreal::get_pe(refno)
        .await
        .unwrap();
    dbg!(pe);
    Ok(())
}

#[tokio::test]
async fn test_query_ancestor_by_refno() -> anyhow::Result<()> {
    super::init_test_surreal().await;
    let refno: RefU64 = "17496/171238".into();
    let type_name = rs_surreal::get_type_name(refno)
        .await
        .unwrap_or_default();
    dbg!(&type_name);
    let ancestor = rs_surreal::get_ancestor(refno)
        .await
        .unwrap();
    dbg!(ancestor);
    let ancestor_maps = rs_surreal::get_ancestor_attmaps(refno)
        .await
        .unwrap();
    dbg!(ancestor_maps);
    Ok(())
}

#[tokio::test]
async fn test_query_wtrans_by_refno() -> anyhow::Result<()> {
    super::init_test_surreal().await;
    // let wtrans = mgr.get_world_transform("17496_118635".into()).await.unwrap();
    // dbg!(wtrans);
    //todo fix POSL attribute
    // let wtrans = mgr.get_world_transform("17496_107068".into()).await.unwrap();
    // dbg!(wtrans);

    // let wtrans = mgr
    //     .get_world_transform("17496_259211".into())
    //     .await
    //     .unwrap();
    // assert_eq!(
    //     wtrans.unwrap().translation,
    //     Vec3::new(79800.0, -19000.0, 3460.0)
    // );
    Ok(())
}

#[tokio::test]
async fn test_query_att_by_refno() {
    super::init_test_surreal().await;
    // let attmap = rs_surreal::get_named_attmap("25688/53371".into()).await;
    // dbg!(attmap);
    let attmap = rs_surreal::get_named_attmap_with_uda("17496/258778".into(), true).await.unwrap();
    dbg!(attmap);
}

#[tokio::test]
async fn test_get_siblings_by_refno() {
    super::init_test_surreal().await;
    let refnos = rs_surreal::get_siblings("17496/258778".into()).await.unwrap();
    dbg!(refnos);
    let next = rs_surreal::get_next_prev("17496/258778".into(), true).await.unwrap();
    let prev = rs_surreal::get_next_prev("17496/258778".into(), false).await.unwrap();
    dbg!((next, prev));
}

#[tokio::test]
async fn test_query_children() {
    super::init_test_surreal().await;
    // let refnos = rs_surreal::get_children_refnos("9304_0".into()).await;
    // dbg!(refnos);
    let nodes = rs_surreal::get_children_ele_nodes("17496_256208".into()).await.unwrap();
    dbg!(nodes);
}

#[tokio::test]
async fn test_query_children_att() {
    super::init_test_surreal().await;
    // let children_atts = rs_surreal::get_children_named_attmaps("17496_171555".into()).await;
    // dbg!(children_atts);
    // let children_pes = rs_surreal::get_children_pes("17496_171555".into()).await;
    // dbg!(children_pes);
    let children_atts = rs_surreal::get_children_named_attmaps("13244/142143".into())
        .await
        .unwrap();
    dbg!(children_atts);
}

#[tokio::test]
async fn test_query_custom() -> anyhow::Result<()> {
    super::init_test_surreal().await;
    let mut response = SUL_DB
        .query(r#"(select owner, owner.noun as o_noun from (type::thing("pe", $refno)))[0]"#)
        .bind(("refno", "17496_171555"))
        .await
        .unwrap();
    let owner_noun: Option<String> = response.take("o_noun").unwrap();
    dbg!(owner_noun);
    let owner: RefU64 = response.take::<Option<String>>("owner")?.unwrap().into();
    dbg!(owner);
    Ok(())
}

#[tokio::test]
async fn test_query_cata() -> anyhow::Result<()> {
    super::init_test_surreal().await;

    let refno = "24384/24827".into();
    let cat_refno = rs_surreal::get_cat_refno(refno)
        .await
        .unwrap();
    dbg!(cat_refno);
    // get_cat_attmap
    let cat_attmap = rs_surreal::get_cat_attmap(refno)
        .await
        .unwrap();
    dbg!(cat_attmap);
    Ok(())
}

#[tokio::test]
async fn test_query_path() -> anyhow::Result<()> {
    super::init_test_surreal().await;
    let cat_refno =
        rs_surreal::query_single_by_paths("15194/5835".into(), &["->GMRE"], &[])
            .await
            .unwrap();
    dbg!(cat_refno);
    Ok(())
}

#[tokio::test]
async fn test_query_paths() -> anyhow::Result<()> {
    super::init_test_surreal().await;
    let cat_refno = rs_surreal::query_single_by_paths(
        "15194/5835".into(),
        &["->GMRE", "->GMSR"],
        &["id"],
    )
    .await
    .unwrap();
    dbg!(cat_refno);
    Ok(())
}

#[tokio::test]
async fn test_query_record_link() -> anyhow::Result<()> {
    super::init_test_surreal().await;

    let str1 = r#"
            {
    "id": "25688_32684",
    "refno": FITT:25688_32684,
    "owner": pe:25688_32682,
    "name": "/1AR07WW0002R",
    "noun": "FITT",
    "dbnum": 1112,
    "e3d_version": 0,
    "cata_hash": "110329119932332",
    "lock": false
}
        "#;

    let str2 = r#"
    {
         "id": "25688_32682",
        "refno": TEST:25688_32682,
        "owner": pe:25688_32681,
        "name": "/1AR07WW0002R",
        "noun": "TEST",
        "dbnum": 1112,
        "e3d_version": 0,
        "cata_hash": "110329119932332",
        "lock": false
    }
        "#;

    // let d1: pdms_element::Model = serde_json::from_str(
    //     str1
    // ).unwrap();
    //
    // let d2: pdms_element::Model = serde_json::from_str(
    // str2
    // ).unwrap();

    SUL_DB
        .query("delete pe:25688_32684; delete pe:25688_32682;")
        // .query("use ns 1516;use db AvevaMarineSample;INSERT IGNORE INTO pe $values")
        // .bind(("values", &[str1, str2]))
        .await
        .unwrap();

    // let response = SUL_DB
    //     .query("use ns 1516;use db AvevaMarineSample;INSERT IGNORE INTO pe $values")
    //     .bind(("values", &[serde_json::to_string_pretty(&d1).unwrap(),
    //         serde_json::to_string_pretty(&d2).unwrap()]))
    //     .await
    //     .unwrap();
    // let s1 = serde_json::to_string_pretty(&d1).unwrap();
    // let s2 = serde_json::to_string_pretty(&d2).unwrap();
    // let mut v1: serde_json::Value = serde_json::to_value(d1.clone()).unwrap();
    // v1.as_object_mut().unwrap().insert("owner".into(), format!("pe:{}", d1.owner.to_string()).into());
    //
    // let mut v2: serde_json::Value = serde_json::to_value(d2).unwrap();
    // v2.as_object_mut().unwrap().insert("owner".into(), format!("pe:{}", "0_0").into());

    let mut sql = format!(
        r#"use ns 1516;use db AvevaMarineSample;INSERT IGNORE INTO pe [{}, {}];"#,
        // serde_json::to_string_pretty(&[str1, str2]).unwrap()
        str1,
        str2
    );

    println!("{}", &sql);

    let response = SUL_DB
        .query(&sql)
        // .query("use ns 1516;use db AvevaMarineSample;INSERT IGNORE INTO pe $values")
        // .bind(("values", &[str1, str2]))
        .await
        .unwrap();

    let q = SUL_DB
        .query("select owner.* from pe:25688_32684;")
        .await
        .unwrap();
    dbg!(q);
    // let q1 = SUL_DB.query("select owner.* from pe:17496_100102;").await.unwrap();
    // dbg!(q1);
    //
    Ok(())
}
