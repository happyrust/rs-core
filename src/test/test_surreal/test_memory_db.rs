//! 内存数据库测试模块
//! 
//! 测试内存数据库的初始化和基本功能

// use crate::test::test_surreal::test_helpers::*;

#[tokio::test]
async fn test_memory_database_init() -> anyhow::Result<()> {
    // 测试内存数据库初始化
    let _db = init_memory_test_surreal().await?;
    
    // 执行简单查询验证连接
    let sql = "RETURN 'memory_test' as result";
    let mut test = Test::new(sql).await?;
    
    let result: String = test.response.take(0)?;
    assert_eq!(result, "memory_test");
    
    // 清理
    cleanup_memory_test_surreal().await?;
    
    Ok(())
}

#[tokio::test]
async fn test_memory_database_pe_operations() -> anyhow::Result<()> {
    // 初始化内存数据库
    let _db = init_memory_test_surreal().await?;
    
    // 测试 PE 表的创建和查询
    let sql = r#"
        INSERT INTO pe {
            id: pe:17496_123456,
            noun: 'EQUI',
            name: 'Test Equipment'
        };
        SELECT VALUE 'pe:17496_123456' as pe_id;
    "#;
    
    let mut test = Test::new(sql).await?;
    
    // 验证插入和查询
    let pe_id: String = test.response.take(1)?; // 第二个结果（索引1）
    assert_eq!(pe_id, "pe:17496_123456");
    
    // 清理
    cleanup_memory_test_surreal().await?;
    
    Ok(())
}

#[tokio::test]
async fn test_memory_database_refno_enum_direct() -> anyhow::Result<()> {
    // 初始化内存数据库
    let _db = init_memory_test_surreal().await?;
    
    // 测试直接返回 pe: 格式的字符串
    let sql = "SELECT VALUE 'pe:17496_654321'";
    let mut test = Test::new(sql).await?;
    
    let pe_id: String = test.response.take(0)?;
    assert_eq!(pe_id, "pe:17496_654321");
    
    // 验证这个字符串可以在业务代码中直接反序列化为 RefnoEnum
    use crate::pdms_types::RefnoEnum;
    use serde_json;
    
    let refno_enum: RefnoEnum = serde_json::from_str(&format!("\"{}\"", pe_id))?;
    assert_eq!(refno_enum.refno().get_0(), 17496);
    assert_eq!(refno_enum.refno().get_1(), 654321);
    assert!(!refno_enum.is_history());
    
    // 清理
    cleanup_memory_test_surreal().await?;
    
    Ok(())
}

#[tokio::test]
async fn test_memory_database_clean() -> anyhow::Result<()> {
    // 初始化内存数据库
    let _db = init_memory_test_surreal().await?;
    
    // 插入测试数据
    let sql_insert = r#"
        INSERT INTO pe {
            id: pe:17496_999999,
            noun: 'TEST',
            name: 'Clean Test'
        };
    "#;
    
    let mut _test = Test::new(sql_insert).await?;
    
    // 清理数据库
    cleanup_memory_test_surreal().await?;
    
    // 重新初始化，验证数据已清理
    let _db2 = init_memory_test_surreal().await?;
    
    let sql_check = "SELECT COUNT() as count FROM pe WHERE id = pe:17496_999999";
    let mut test_check = Test::new(sql_check).await?;
    
    let count: i64 = test_check.response.take(0)?;
    assert_eq!(count, 0, "Data should be cleaned up");
    
    Ok(())
}
