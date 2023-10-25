use crate::named_attmap::NamedAttrMap;
use crate::named_attvalue::NamedAttrValue;
use crate::test::test_sql::get_version_conn;

#[tokio::test]
async fn test_gen_insert_sql() {
    // let create_sql = db_info.gen_create_table_sql("BOX").unwrap();
    // dbg!(&create_sql);
    let mut att = NamedAttrMap::new("PIPE");
    att.insert("REFNO".to_string(), NamedAttrValue::StringType("1/1".into()));
    att.insert("OWNER".to_string(), NamedAttrValue::StringType("1/0".into()));
    // att.insert("REFNO".to_string(), NamedAttrValue::StringType("1/0".into()));
    dbg!(&att);
    let sql = att.gen_insert_sql().unwrap();
    dbg!(&sql);

    let db = get_version_conn().await;
    att.exec_insert(&db).await.unwrap();
}