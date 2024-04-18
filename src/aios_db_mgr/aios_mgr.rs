use std::str::FromStr;
use std::time::Duration;
use crate::aios_db_mgr::PdmsDataInterface;
use crate::options::DbOption;
use crate::pdms_types::{EleTreeNode, PdmsElement};
use crate::{AttrMap, get_children_ele_nodes, get_named_attmap, get_named_attmap_with_uda, get_next_prev, get_world, NamedAttrMap, RefU64, SUL_DB, SurlValue};
use async_trait::async_trait;
use bevy_transform::components::Transform;
use config::{Config, File};
use sqlx::{MySql, Pool};
use sqlx::pool::PoolOptions;
use surrealdb::engine::any::Any;
use surrealdb::Surreal;
use crate::table_const::{GLOBAL_DATABASE, PUHUA_MATERIAL_DATABASE};
use crate::pe::SPdmsElement;
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

    /// 获取所属房间的顶标高和底标高
    pub async fn query_own_room_panel_elevations(&self,refno: RefU64) -> anyhow::Result<(f32, f32)> {
        let sql = format!("return fn::room_top_height({});
                        return fn::room_height({});", refno.to_pe_key(), refno.to_pe_key());
        let mut response = SUL_DB
            .query(sql)
            .await?;
        let min: Vec<f32> = response.take(0).unwrap_or(vec![]);
        let max: Vec<f32> = response.take(1).unwrap_or(vec![]);
        let min = min.get(0).map_or(0.0,|x| *x);
        let max = max.get(0).map_or(0.0,|x| *x);
        Ok((min, max))
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

    async fn get_attr(&self, refno: RefU64) -> anyhow::Result<NamedAttrMap> {
        get_named_attmap_with_uda(refno, false).await
    }

    async fn get_children(&self, refno: RefU64) -> anyhow::Result<Vec<EleTreeNode>> {
        get_children_ele_nodes(refno).await
    }

    async fn get_ipara_from_bran(&self, refno: RefU64) -> anyhow::Result<Vec<f32>> {
        let sql = format!("
        select value (select value CATR.refno.PARA from only
        owner.refno.ISPE<-pe_owner<-pe<-pe_owner<-pe[? refno.TYPE = 'SELE'
        and $parent.owner.refno.TEMP >=refno.ANSW and $parent.owner.refno.TEMP <= refno.MAXA ]<-pe_owner<-pe.refno.*
        where $parent.owner.refno.HBOR >=ANSW and $parent.owner.refno.HBOR <= MAXA limit 1) from pe:{};", refno.to_string());
        let mut response = SUL_DB
            .query(sql)
            .await?;
        let result: Vec<Vec<f32>> = response.take(0).unwrap_or(vec![]);
        Ok(result.into_iter().flatten().collect())
    }

    async fn get_ele_from_name(&self, name: &str) -> anyhow::Result<Option<PdmsElement>> {
        let name = if name.starts_with("/") { name.to_string() } else { format!("/{}", name) };
        let sql = format!("select * from pe where name = '{}';", name);
        let mut response = SUL_DB
            .query(sql)
            .await?;
        let pe: Option<SPdmsElement> = response.take(0)?;
        if pe.is_none() { return Ok(None); };
        let pe = pe.unwrap();
        Ok(Some(PdmsElement {
            refno: pe.refno,
            owner: pe.owner,
            name: pe.name,
            noun: pe.noun,
            version: 0,
            children_count: 0,
        }))
    }

    async fn get_spre_attr(&self, refno: RefU64) -> anyhow::Result<Option<NamedAttrMap>> {
        let sql = format!("(select * from {}.refno.SPRE.refno)[0]", refno.to_pe_key());
        let mut response = SUL_DB
            .query(sql)
            .await?;
        let o: SurlValue = response.take(0)?;
        let named_attmap: NamedAttrMap = o.into();
        if named_attmap.map.is_empty() { return Ok(None); };
        Ok(Some(named_attmap))
    }

    async fn get_catr_attr(&self, refno: RefU64) -> anyhow::Result<Option<NamedAttrMap>> {
        let sql = format!("(select * from {}.refno.SPRE.refno.CATR.refno)[0]", refno.to_pe_key());
        let mut response = SUL_DB
            .query(sql)
            .await?;
        let o: SurlValue = response.take(0)?;
        let named_attmap: NamedAttrMap = o.into();
        if named_attmap.map.is_empty() { return Ok(None); };
        Ok(Some(named_attmap))
    }

    async fn get_foreign_attr(&self, refno: RefU64, foreign_type: &str) -> anyhow::Result<Option<NamedAttrMap>> {
        let sql = format!("(select * from {}.refno.{}.refno)[0]", refno.to_pe_key(), foreign_type);
        let mut response = SUL_DB
            .query(sql)
            .await?;
        let o: SurlValue = response.take(0)?;
        let named_attmap: NamedAttrMap = o.into();
        if named_attmap.map.is_empty() { return Ok(None); };
        Ok(Some(named_attmap))
    }

    async fn get_name(&self, refno: RefU64) -> anyhow::Result<String> {
        let sql = format!("
        (select value (if name='' {{ string::concat(noun,
        <string> (array::find_index(select value order_num from ->pe_owner->pe<-pe_owner[where <-pe[where noun=$parent.noun]]
        order by order_num, ->pe_owner[0].order_num) + 1) ) }} else {{ name }} ) from {})[0];
        ", refno.to_pe_key());
        let mut response = SUL_DB
            .query(sql)
            .await?;
        let o: Option<String> = response.take(0)?;
        Ok(o.unwrap_or("".to_string()))
    }

    async fn get_world_transform(&self, refno: RefU64) -> anyhow::Result<Option<Transform>> {
        let sql = format!("
        (select (->inst_relate.world_trans.d)[0] as length from {})[0].length;
        ", refno.to_pe_key());
        let mut response = SUL_DB
            .query(sql)
            .await?;
        let transform: Option<Transform> = response.take(0)?;
        Ok(transform)
    }

    async fn get_prev(&self, refno: RefU64) -> anyhow::Result<RefU64> {
        get_next_prev(refno, false).await
    }

    async fn get_next(&self, refno: RefU64) -> anyhow::Result<RefU64> {
        get_next_prev(refno, true).await
    }

    async fn get_room_code(&self, refno: RefU64) -> anyhow::Result<Option<String>> {
        let sql = format!("
        return fn::room_code({})[0];
        ", refno.to_pe_key());
        let mut response = SUL_DB
            .query(sql)
            .await?;
        Ok(response.take(0)?)
    }
}

impl AiosDBMgr {
    ///获得默认的连接字符串
    pub fn default_conn_str(&self) -> String {
        let d = &self.db_option;
        let user = d.user.as_str();
        let pwd = urlencoding::encode(&d.password);
        let ip = d.ip.as_str();
        let port = d.port.as_str();
        format!("mysql://{user}:{pwd}@{ip}:{port}")
    }

    pub fn puhua_conn_str(&self) -> String {
        let d = &self.db_option;
        let user = d.puhua_database_user.as_str();
        let pwd = d.puhua_database_password.as_str();
        let ip = d.puhua_database_ip.as_str();
        format!("mysql://{user}:{pwd}@{ip}")
    }

    /// 获取项目配置信息pool
    pub async fn get_global_pool(&self) -> anyhow::Result<Pool<MySql>> {
        let connection_str = self.default_conn_str();
        let url = &format!("{connection_str}/{}", GLOBAL_DATABASE);
        PoolOptions::new()
            .max_connections(500)
            .acquire_timeout(Duration::from_secs(10 * 60))
            .connect(url)
            .await
            .map_err({ |x| anyhow::anyhow!(x.to_string()) })
    }

    /// 获取外部的数据库
    pub async fn get_puhua_pool(&self) -> anyhow::Result<Pool<MySql>> {
        let conn = self.puhua_conn_str();
        let url = &format!("{conn}/{}", PUHUA_MATERIAL_DATABASE);
        PoolOptions::new()
            .max_connections(500)
            .acquire_timeout(Duration::from_secs(10 * 60))
            .connect(url)
            .await
            .map_err({ |x| anyhow::anyhow!(x.to_string()) })
    }
}

#[tokio::test]
async fn test_get_world() {
    let mgr = AiosDBMgr::init_from_db_option().await.unwrap();
    let data = mgr.get_world("/ALL").await.unwrap();
    let data = mgr.get_ele_from_name("/1WCC0294").await.unwrap();
    dbg!(&data);
    let attr = mgr.get_spre_attr(RefU64::from_str("24383/67331").unwrap()).await.unwrap();
    dbg!(&attr);
    let attr = mgr.get_catr_attr(RefU64::from_str("24383/67350").unwrap()).await.unwrap();
    dbg!(&attr);
    let attr = mgr.get_foreign_attr(RefU64::from_str("24383/67331").unwrap(), "HSTU").await.unwrap();
    dbg!(&attr);
    let name = mgr.get_name(RefU64::from_str("24383/67331").unwrap()).await.unwrap();
    dbg!(&name);
    let transform = mgr.get_world_transform(RefU64::from_str("24383/67335").unwrap()).await.unwrap();
    dbg!(&transform);
    let room = mgr.get_room_code(RefU64::from_str("24384/24804").unwrap()).await.unwrap();
    dbg!(&room);
}