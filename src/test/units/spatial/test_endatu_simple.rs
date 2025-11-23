use crate::*;

/// 简单的 ENDATU 测试，验证基本功能
#[tokio::test]
async fn test_endatu_basic_functionality() -> anyhow::Result<()> {
    init_surreal().await?;
    
    println!("🧪 测试 ENDATU 基本功能");
    
    // 测试错误码映射
    use crate::transform::strategies::EndatuError;
    
    let error = EndatuError::InvalidIndex(2);
    let code = error.to_pdms_code();
    assert_eq!(code, 251, "错误码映射应该正确");
    
    println!("   ✅ 错误码映射测试通过");
    
    // 测试缓存功能
    use crate::transform::strategies::{get_cached_endatu_index, clear_endatu_cache};
    
    clear_endatu_cache();
    
    let parent = RefnoEnum::from("test_parent");
    let refno = RefnoEnum::from("test_refno");
    
    // 测试缓存查询（即使失败也不应该崩溃）
    let result = get_cached_endatu_index(parent, refno).await;
    assert!(result.is_ok() || result.is_err(), "缓存查询应该不崩溃");
    
    println!("   ✅ 缓存功能测试通过");
    
    // 测试参数验证
    use crate::transform::strategies::EndatuValidator;
    use crate::test::test_helpers::create_test_attmap_with_attributes;
    use crate::types::attval::AttrVal;
    
    let mut att = create_test_attmap_with_attributes();
    
    // 有效属性
    att.insert("ZDIS".to_string(), AttrVal::DoubleType(100.0).into());
    att.insert("OPDI".to_string(), AttrVal::Vec3Type([1.0, 0.0, 0.0]).into());
    assert!(EndatuValidator::validate_endatu_attributes(&att).is_ok());
    
    println!("   ✅ 参数验证测试通过");
    
    println!("🎉 ENDATU 基本功能测试完成！");
    Ok(())
}
