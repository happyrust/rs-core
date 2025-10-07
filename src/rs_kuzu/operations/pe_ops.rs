//! PE 操作
//!
//! 提供 PE 节点的写入和更新操作

#[cfg(feature = "kuzu")]
use crate::rs_kuzu::create_kuzu_connection;
#[cfg(feature = "kuzu")]
use crate::types::*;
#[cfg(feature = "kuzu")]
use anyhow::Result;

#[cfg(feature = "kuzu")]
/// 转义字符串中的单引号
fn escape_string(s: &str) -> String {
    s.replace("'", "''")
}

#[cfg(feature = "kuzu")]
/// 保存单个 PE 节点
pub async fn save_pe_node(pe: &SPdmsElement) -> Result<()> {
    let conn = create_kuzu_connection()?;

    let query = format!(
        "CREATE (p:PE {{
            refno: {},
            name: '{}',
            noun: '{}',
            dbnum: {},
            sesno: {},
            cata_hash: '{}',
            deleted: {},
            lock: {},
            typex: {}
        }})",
        pe.refno.refno().0,
        escape_string(&pe.name),
        escape_string(&pe.noun),
        pe.dbnum,
        pe.sesno,
        escape_string(&pe.cata_hash),
        pe.deleted,
        pe.lock,
        pe.typex.unwrap_or(0)
    );

    conn.query(&query)?;
    log::debug!("保存 PE 节点: {} ({}) refno={} typex={:?}",
                pe.name, pe.noun, pe.refno.refno(), pe.typex);

    Ok(())
}

#[cfg(feature = "kuzu")]
/// 批量保存 PE 节点
pub async fn save_pe_batch(pes: &[SPdmsElement]) -> Result<()> {
    let conn = create_kuzu_connection()?;

    conn.query("BEGIN TRANSACTION")?;

    let result = (|| {
        for pe in pes {
            let query = format!(
                "CREATE (p:PE {{
                    refno: {},
                    name: '{}',
                    noun: '{}',
                    dbnum: {},
                    sesno: {},
                    cata_hash: '{}',
                    deleted: {},
                    lock: {},
                    typex: {}
                }})",
                pe.refno.refno().0,
                escape_string(&pe.name),
                escape_string(&pe.noun),
                pe.dbnum,
                pe.sesno,
                escape_string(&pe.cata_hash),
                pe.deleted,
                pe.lock,
                pe.typex.unwrap_or(0)
            );

            conn.query(&query)?;
        }
        Ok::<(), anyhow::Error>(())
    })();

    match result {
        Ok(_) => {
            conn.query("COMMIT")?;
            log::info!("批量保存 PE 节点成功: {} 个", pes.len());
            Ok(())
        }
        Err(e) => {
            conn.query("ROLLBACK")?;
            log::error!("批量保存 PE 节点失败: {}", e);
            Err(e)
        }
    }
}

#[cfg(feature = "kuzu")]
/// 保存 PE（兼容旧接口）
pub async fn save_pe_kuzu(pe: &SPdmsElement) -> Result<()> {
    save_pe_node(pe).await
}

#[cfg(feature = "kuzu")]
/// 批量保存 PE（兼容旧接口）
pub async fn save_pe_batch_kuzu(pes: Vec<SPdmsElement>) -> Result<()> {
    save_pe_batch(&pes).await
}
