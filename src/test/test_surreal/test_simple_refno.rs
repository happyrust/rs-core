//! 简化的 RefnoEnum 测试
//!
//! 避免依赖 Bevy 等复杂的依赖，专注于测试基本功能
//! 使用独立的内存数据库实例进行测试

use crate::pdms_types::RefnoEnum;
use crate::test::test_surreal::test_helpers::create_memory_db;
use anyhow::Result;
use serde_json;

#[tokio::test]
async fn test_memory_database_simple() -> Result<()> {
    // 创建独立的内存数据库实例
    let db = create_memory_db().await?;

    // 执行简单查询
    let sql = "RETURN 'test_ok'";
    let mut response = db.query(sql).await?;

    let result: Option<String> = response.take(0)?;
    assert_eq!(result, Some("test_ok".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_pe_format_string() -> Result<()> {
    // 测试直接返回 RefnoEnum
    let db = create_memory_db().await?;

    let sql = "SELECT VALUE 'pe:17496_123456'";
    let mut response = db.query(sql).await?;

    // 直接获取 RefnoEnum，无需手动反序列化
    let refno_enum: Option<RefnoEnum> = response.take(0)?;
    let refno_enum = refno_enum.unwrap();
    
    assert_eq!(refno_enum.refno().get_0(), 17496);
    assert_eq!(refno_enum.refno().get_1(), 123456);
    assert!(!refno_enum.is_history());

    Ok(())
}

#[tokio::test]
async fn test_multiple_pe_formats() -> Result<()> {
    // 测试多种 pe: 格式
    let db = create_memory_db().await?;

    let test_cases = vec![
        ("pe:17496_111111", 17496, 111111),
        ("pe:24383_222222", 24383, 222222),
        ("pe:17496_333333", 17496, 333333),
    ];

    for (pe_str, expected_dbnum, expected_sesno) in test_cases {
        let sql = format!("SELECT VALUE '{}'", pe_str);
        let mut response = db.query(&sql).await?;

        // 直接获取 RefnoEnum
        let refno_enum: Option<RefnoEnum> = response.take(0)?;
        let refno_enum = refno_enum.unwrap();
        
        assert_eq!(refno_enum.refno().get_0(), expected_dbnum);
        assert_eq!(refno_enum.refno().get_1(), expected_sesno);
        assert!(!refno_enum.is_history());
    }

    Ok(())
}

#[tokio::test]
async fn test_pe_insert_and_query() -> Result<()> {
    // 测试完整的插入和查询流程
    let db = create_memory_db().await?;

    // 插入测试数据
    let insert_sql = r#"
        INSERT INTO pe {
            id: pe:17496_999999,
            noun: 'TEST',
            name: 'Simple Test'
        };
    "#;

    let mut _insert_response = db.query(insert_sql).await?;

    // 查询插入的数据，直接返回 RefnoEnum
    let query_sql = "SELECT VALUE id FROM pe:17496_999999";
    let mut response = db.query(query_sql).await?;

    let refno_enum: Option<RefnoEnum> = response.take(0)?;
    let refno_enum = refno_enum.unwrap();
    
    // 验证 RefnoEnum 功能
    assert_eq!(refno_enum.refno().get_0(), 17496);
    assert_eq!(refno_enum.refno().get_1(), 999999);
    assert_eq!(refno_enum.to_pe_key(), "pe:17496_999999");

    Ok(())
}

#[tokio::test]
async fn test_refno_enum_with_session() -> Result<()> {
    // 测试带会话号的 RefnoEnum
    let db = create_memory_db().await?;

    // 测试 JSON 数组格式 ["refno", sesno]
    let sql = "SELECT ['17496_123456', 733]";
    let mut response = db.query(sql).await?;

    let result: Option<Vec<serde_json::Value>> = response.take(0)?;
    let result = result.unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0], "17496_123456");
    assert_eq!(result[1], 733);

    // 模拟在业务代码中的使用
    let json_array = serde_json::json!(["17496_123456", 733]);
    let refno_enum: RefnoEnum = serde_json::from_value(json_array)?;
    assert!(refno_enum.is_history());
    assert_eq!(refno_enum.refno().get_0(), 17496);
    assert_eq!(refno_enum.refno().get_1(), 123456);

    if let RefnoEnum::SesRef(ses_ref) = refno_enum {
        assert_eq!(ses_ref.sesno, 733);
    } else {
        return Err(anyhow::anyhow!("Expected history RefnoEnum"));
    }

    Ok(())
}
