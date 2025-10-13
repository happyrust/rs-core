#[cfg(feature = "kuzu")]
use aios_core::rs_kuzu::schema::init_kuzu_schema;
#[cfg(feature = "kuzu")]
use aios_core::rs_kuzu::{get_kuzu_connection, init_kuzu};
use anyhow::Error;
#[cfg(feature = "kuzu")]
use kuzu::SystemConfig;
#[cfg(feature = "kuzu")]
use tempfile::TempDir;

#[test]
fn test_attr_elbo_table_creation() {
    #[cfg(not(feature = "kuzu"))]
    {
        println!("kuzu feature not enabled; skipping test");
        return;
    }
    #[cfg(feature = "kuzu")]
    {
        let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
        rt.block_on(async {
            let temp_dir = TempDir::new()?;
            let db_path = temp_dir.path().join("kuzu_elbo");

            init_kuzu(
                db_path.to_str().expect("temp path should be valid UTF-8"),
                SystemConfig::default(),
            )
            .await?;

            init_kuzu_schema().await?;

            let conn_guard = get_kuzu_connection()?;
            let conn = &*conn_guard;

            conn.query(
                "CREATE (:Attr_ELBO {refno: 1, STATUS_CODE: 'OK', LOCK: false, SPRE_REFNO: 100})",
            )?;

            let mut result = conn.query("MATCH (a:Attr_ELBO {refno: 1}) RETURN a.STATUS_CODE")?;
            assert!(result.next().is_some(), "Attr_ELBO 表创建或查询失败");
            Ok::<(), Error>(())
        })
        .unwrap();
    }
}
