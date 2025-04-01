use crate::{rs_surreal::SUL_DB, RefU64};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

/// Represents a connection between pipe elements
#[derive(Debug, Serialize, Deserialize)]
pub struct PipeConnection {
    pub id: RefU64,
    pub prev: Option<RefU64>,
    pub next: Option<RefU64>,
}

/// Get the previous connected pipe element
///
/// Wraps the Surreal function fn::prev_connect_pe
pub async fn get_prev_connect_pe(pe_id: RefU64) -> Result<Option<RefU64>> {
    let sql = format!("RETURN fn::prev_connect_pe({})", pe_id.to_pe_key());
    let result: Option<RefU64> = SUL_DB.query(sql).await?.take(0)?;

    Ok(result)
}

/// Get the next connected pipe element
///
/// Wraps the Surreal function fn::next_connect_pe
pub async fn get_next_connect_pe(pe_id: RefU64) -> Result<Option<RefU64>> {
    let sql = format!("RETURN fn::next_connect_pe({})", pe_id.to_pe_key());
    let result: Option<RefU64> = SUL_DB.query(sql).await?.take(0)?;

    Ok(result)
}

/// Check if a pipe element has a leaving tubi
///
/// Wraps the Surreal function fn::has_leave_tubi
pub async fn has_leave_tubi(pe_id: RefU64) -> Result<bool> {
    let sql = format!("RETURN fn::has_leave_tubi({})", pe_id.to_pe_key());
    let result: Option<bool> = SUL_DB.query(sql).await?.take(0)?;

    Ok(result.unwrap_or(false))
}

/// Check if a pipe element has an arriving tubi
///
/// Wraps the Surreal function fn::has_arrive_tubi
pub async fn has_arrive_tubi(pe_id: RefU64) -> Result<bool> {
    let sql = format!("RETURN fn::has_arrive_tubi({})", pe_id.to_pe_key());
    let result: Option<bool> = SUL_DB.query(sql).await?.take(0)?;

    Ok(result.unwrap_or(false))
}

/// Get both previous and next connected pipe elements
///
/// Returns a PipeConnection struct containing the original PE and its connections
pub async fn get_pipe_connections(pe_id: RefU64) -> Result<PipeConnection> {
    let prev = get_prev_connect_pe(pe_id).await?;
    let next = get_next_connect_pe(pe_id).await?;

    Ok(PipeConnection {
        id: pe_id,
        prev,
        next,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{init_test_surreal, RefnoEnum};

    #[tokio::test]
    async fn test_get_pipe_connections() -> Result<()> {
        // Initialize the SurrealDB connection
        init_test_surreal().await;

        // Use a specific refno for testing
        let pe_id: RefU64 = "24383_76176".into();

        // Get pipe connections
        let connections = get_pipe_connections(pe_id).await?;

        // Print the connections for debugging
        println!("Pipe connections for {:?}:", pe_id);
        println!("  Previous: {:?}", connections.prev);
        println!("  Next: {:?}", connections.next);

        // Assert that we got a valid connection structure
        assert_eq!(connections.id, pe_id);

        // Note: The actual prev/next values depend on your database content
        // You might want to add specific assertions for your test data

        Ok(())
    }
}
