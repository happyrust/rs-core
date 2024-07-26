use crate::{get_db_option, get_default_pdms_db_info, query_same_refnos_by_type, RefU64};

///获得branch下的所有托臂
#[tokio::test]
async fn test_query_support_arms() -> anyhow::Result<()> {
    // let gensec_refno: RefU64 = "24384/25797".into();
    // let mgr = get_test_ams_db_manager_async().await;
    // let mut tmp = mgr.query_foreign_refnos(&[gensec_refno], &[&["SPRE", "CATR"]],
    //                                     &["PSTR", "PTSS"], &[], 4).await?;
    // assert_eq!(tmp.pop().unwrap().to_string(), "21438/2368");

    Ok(())
}

#[tokio::test]
async fn test_query_same_refnos() -> anyhow::Result<()> {
    crate::init_test_surreal().await;
    let refno: RefU64 = "17496/274055".into();
    let db_option = get_db_option();
    let same_refnos =
        query_same_refnos_by_type(refno, db_option.mdb_name.clone(), crate::DBType::DESI).await?;
    dbg!(&same_refnos);

    Ok(())
}

//query_same_refnos_by_type
