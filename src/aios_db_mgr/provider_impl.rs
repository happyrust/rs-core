//! 基于 QueryProvider 的 PdmsDataInterface 实现
//!
//! 这个模块使用统一的 QueryProvider 接口实现 PDMS 领域特定的数据访问

use crate::aios_db_mgr::PdmsDataInterface;
use crate::pdms_types::{EleTreeNode, PdmsElement};
use crate::pe::SPdmsElement;
use crate::query_provider::QueryProvider;
use crate::{
    NamedAttrMap, RefU64, RefnoEnum, SUL_DB, SurlValue, SurrealQueryExt,
    get_children_ele_nodes, get_named_attmap_with_uda, get_next_prev, get_world_transform,
};
use async_trait::async_trait;
use bevy_transform::components::Transform;
use surrealdb::IndexedResults as Response;
use std::sync::Arc;
use tracing::info;

/// 基于 QueryProvider 的 PdmsDataInterface 实现
pub struct ProviderPdmsInterface {
    provider: Arc<dyn QueryProvider>,
}

impl ProviderPdmsInterface {
    /// 创建新的 ProviderPdmsInterface 实例
    pub fn new(provider: Arc<dyn QueryProvider>) -> Self {
        Self { provider }
    }
}

#[async_trait]
impl PdmsDataInterface for ProviderPdmsInterface {
    async fn get_world(&self, mdb_name: &str) -> anyhow::Result<Option<PdmsElement>> {
        let mdb_name = if mdb_name.starts_with("/") {
            mdb_name.to_string()
        } else {
            format!("/{}", mdb_name)
        };
        
        // 使用 SUL_DB 查询，因为 QueryProvider 没有直接的 get_world 按名称查询
        let sql = format!(
            "SELECT * FROM pe WHERE noun = 'WORL' AND name = '{}' LIMIT 1;",
            mdb_name
        );
        let mut response: Response = SUL_DB.query_response(sql).await?;
        let pe: Option<SPdmsElement> = response.take(0)?;
        
        if let Some(pe) = pe {
            Ok(Some(PdmsElement {
                refno: pe.refno(),
                owner: pe.owner.refno(),
                name: pe.name,
                noun: pe.noun,
                version: 0,
                children_count: 0,
            }))
        } else {
            Ok(None)
        }
    }

    async fn get_pdms_element(&self, refno: RefU64) -> anyhow::Result<Option<PdmsElement>> {
        let pe = self.provider.get_pe(refno.into()).await?;
        if let Some(pe) = pe {
            Ok(Some(PdmsElement {
                refno: pe.refno(),
                owner: pe.owner.refno(),
                name: pe.name,
                noun: pe.noun,
                version: 0,
                children_count: 0,
            }))
        } else {
            Ok(None)
        }
    }

    async fn get_attr(&self, refno: RefU64) -> anyhow::Result<NamedAttrMap> {
        get_named_attmap_with_uda(refno.into()).await
    }

    async fn get_children(&self, refno: RefU64) -> anyhow::Result<Vec<EleTreeNode>> {
        get_children_ele_nodes(refno.into()).await
    }

    async fn get_ipara_from_bran(&self, refno: RefU64) -> anyhow::Result<Vec<f32>> {
        // 复杂的 SurrealQL 查询，暂时保持使用 SUL_DB
        let sql = format!("
        select value (select value CATR.refno.PARA from only
        owner.refno.ISPE<-pe_owner<-pe<-pe_owner<-pe[? refno.TYPE = 'SELE'
        and $parent.owner.refno.TEMP >=refno.ANSW and $parent.owner.refno.TEMP <= refno.MAXA ]<-pe_owner<-pe.refno.*
        where $parent.owner.refno.HBOR >=ANSW and $parent.owner.refno.HBOR <= MAXA limit 1) from pe:{};", refno.to_string());
        let mut response: Response = SUL_DB.query_response(sql).await?;
        let result: Vec<Vec<f32>> = response.take(0).unwrap_or(vec![]);
        Ok(result.into_iter().flatten().collect())
    }

    async fn get_ele_from_name(&self, name: &str) -> anyhow::Result<Option<PdmsElement>> {
        let name = if name.starts_with("/") {
            name.to_string()
        } else {
            format!("/{}", name)
        };
        let sql = format!("select * from pe where name = '{}';", name);
        let mut response: Response = SUL_DB.query_response(sql).await?;
        let pe: Option<SPdmsElement> = response.take(0)?;
        if pe.is_none() {
            return Ok(None);
        };
        let pe = pe.unwrap();
        Ok(Some(PdmsElement {
            refno: pe.refno(),
            owner: pe.owner.refno(),
            name: pe.name,
            noun: pe.noun,
            version: 0,
            children_count: 0,
        }))
    }

    async fn get_spre_attr(&self, refno: RefU64) -> anyhow::Result<Option<NamedAttrMap>> {
        let sql = format!("(select * from {}.refno.SPRE.refno)[0]", refno.to_pe_key());
        let mut response: Response = SUL_DB.query_response(sql).await?;
        let o: SurlValue = response.take(0)?;
        let named_attmap: NamedAttrMap = o.into();
        if named_attmap.map.is_empty() {
            return Ok(None);
        };
        Ok(Some(named_attmap))
    }

    async fn get_catr_attr(&self, refno: RefU64) -> anyhow::Result<Option<NamedAttrMap>> {
        let sql = format!(
            "(select * from {}.refno.SPRE.refno.CATR.refno)[0]",
            refno.to_pe_key()
        );
        let mut response: Response = SUL_DB.query_response(sql).await?;
        let o: SurlValue = response.take(0)?;
        let named_attmap: NamedAttrMap = o.into();
        if named_attmap.map.is_empty() {
            return Ok(None);
        };
        Ok(Some(named_attmap))
    }

    async fn get_foreign_attr(
        &self,
        refno: RefU64,
        foreign_type: &str,
    ) -> anyhow::Result<Option<NamedAttrMap>> {
        let sql = format!(
            "(select * from {}.refno.{}.refno)[0]",
            refno.to_pe_key(),
            foreign_type
        );
        let mut response: Response = SUL_DB.query_response(sql).await?;
        let o: SurlValue = response.take(0)?;
        let named_attmap: NamedAttrMap = o.into();
        if named_attmap.map.is_empty() {
            return Ok(None);
        };
        Ok(Some(named_attmap))
    }

    async fn get_name(&self, refno: RefU64) -> anyhow::Result<String> {
        let sql = format!(
            "
            return fn::default_name({});
        ",
            refno.to_pe_key()
        );
        let mut response: Response = SUL_DB.query_response(sql).await?;
        let o: Option<String> = response.take(0)?;
        Ok(o.unwrap_or("".to_string()))
    }

    async fn get_world_transform(&self, refno: RefU64) -> anyhow::Result<Option<Transform>> {
        get_world_transform(refno.into()).await
    }

    async fn get_prev(&self, refno: RefU64) -> anyhow::Result<RefU64> {
        get_next_prev(refno.into(), false).await.map(|x| x.into())
    }

    async fn get_next(&self, refno: RefU64) -> anyhow::Result<RefU64> {
        get_next_prev(refno.into(), true).await.map(|x| x.into())
    }

    async fn get_room_code(&self, refno: RefU64) -> anyhow::Result<Option<String>> {
        let sql = format!("return fn::room_code({})[0];", refno.to_pe_key());
        let sql_debug = sql.clone();
        let mut response: Response = SUL_DB.query_response(sql).await?;
        let r: Option<String> = response.take(0)?;
        match r {
            Some(room_code) => {
                if room_code.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(room_code))
                }
            }
            None => Ok(None),
        }
    }

    async fn query_all_rooms(&self) -> anyhow::Result<Vec<PdmsElement>> {
        let mut response = SUL_DB.query_response(r#"
            SELECT 
                id,
                refno,
                name,
                noun,
                owner,
                description,
                purp,
                status
            FROM pe 
            WHERE noun IN ['ROOM', 'FRMW'] 
            AND name != NONE
            ORDER BY name
        "#).await?;
        
        let rooms: Vec<PdmsElement> = response.take(0)?;
        info!("🏠 查询到 {} 个房间", rooms.len());
        Ok(rooms)
    }

    async fn query_room_elements(&self, room_refno: RefU64) -> anyhow::Result<Vec<PdmsElement>> {
        let room_refno_enum: RefnoEnum = room_refno.into();
        
        let mut response = SUL_DB.query_response(&format!(r#"
            SELECT 
                id,
                refno,
                name,
                noun,
                owner,
                description,
                purp,
                status,
                solid,
                (SELECT VALUE noun FROM pe WHERE refno = $parent.owner LIMIT 1) as owner_noun
            FROM pe 
            WHERE room_relate CONTAINS {}
            AND solid = true
            ORDER BY noun, name
        "#, room_refno_enum.to_pe_key())).await?;
        
        let elements: Vec<PdmsElement> = response.take(0)?;
        info!("🔍 房间 {} 查询到 {} 个元素", room_refno.0, elements.len());
        Ok(elements)
    }
}
