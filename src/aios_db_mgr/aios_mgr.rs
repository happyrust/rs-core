use crate::aios_db_mgr::PdmsDataInterface;
use crate::options::DbOption;
use crate::pdms_types::PdmsElement;
use crate::{AttrMap, get_named_attmap, get_world, NamedAttrMap, RefU64};
use async_trait::async_trait;
use config::{Config, File};
use crate::test::test_surreal::{init_surreal_with_signin, init_test_surreal};

pub struct AiosDBMgr {
    pub db_option: DbOption,
}

impl AiosDBMgr {
    pub async fn init_from_db_option() -> anyhow::Result<Self> {
        let s = Config::builder()
            .add_source(File::with_name("DbOption"))
            .build()
            .unwrap();
        let db_option: DbOption = s.try_deserialize().unwrap();
        init_surreal_with_signin(&db_option).await?;
        Ok(Self {
            db_option,
        })
    }
}

#[async_trait]
impl PdmsDataInterface for AiosDBMgr {
    async fn get_world(&self, mdb_name: &str) -> anyhow::Result<Option<PdmsElement>> {
        let Some(world) = get_world(format!("/{}",mdb_name)).await? else { return Ok(None); };
        Ok(Some(PdmsElement {
            refno: world.refno,
            owner: world.owner,
            name: world.name,
            noun: world.noun,
            version: 0,
            children_count: 1,
        }))
    }

    async fn get_named_attr(&self, refno: RefU64) -> anyhow::Result<NamedAttrMap> {
        get_named_attmap(refno).await
    }
}

#[tokio::test]
async fn test_get_world() {
    let mgr = AiosDBMgr::init_from_db_option().await.unwrap();
    let data = mgr.get_world("/ALL").await.unwrap();
    dbg!(&data);
}