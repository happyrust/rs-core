use crate::aios_db_mgr::PdmsDataInterface;
use crate::options::DbOption;
use crate::pdms_types::{EleTreeNode, PdmsElement};
use crate::{AttrMap, get_children_ele_nodes, get_named_attmap, get_world, NamedAttrMap, RefU64, SUL_DB};
use async_trait::async_trait;
use config::{Config, File};
use surrealdb::engine::any::Any;
use surrealdb::Surreal;
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

    pub async fn get_surreal_db(&self) -> anyhow::Result<Surreal<Any>> {
        init_surreal_with_signin(&self.db_option).await?;
        Ok(SUL_DB.clone())
    }
}

#[async_trait]
impl PdmsDataInterface for AiosDBMgr {
    async fn get_world(&self, mdb_name: &str) -> anyhow::Result<Option<PdmsElement>> {
        let Some(world) = get_world(format!("/{}", mdb_name)).await? else { return Ok(None); };
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

    async fn get_children(&self, refno: RefU64) -> anyhow::Result<Vec<EleTreeNode>> {
        get_children_ele_nodes(refno).await
    }
}

#[tokio::test]
async fn test_get_world() {
    let mgr = AiosDBMgr::init_from_db_option().await.unwrap();
    let data = mgr.get_world("/ALL").await.unwrap();
    dbg!(&data);
}