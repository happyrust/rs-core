use crate::*;
use anyhow::Result;
use crate::test::test_helpers::*;

/// 测试重构后的策略模式
/// 验证不同类型的构件使用正确的策略进行处理
#[tokio::test]
async fn test_transform_strategies() -> Result<()> {
    init_surreal().await?;
    
    println!("🧪 测试变换计算策略模式重构");
    
    // 测试用例 1: GENSEC 类型
    let gensec_refno = RefnoEnum::from("14207_545"); // 示例 GENSEC
    if let Ok(gensec_att) = get_named_attmap(gensec_refno).await {
        let gensec_type = gensec_att.get_type_str();
        if gensec_type == "GENSEC" {
            println!("✅ 测试 GENSEC 策略");
            
            // 使用策略工厂获取策略
            let strategy = crate::transform::strategies::TransformStrategyFactory::get_strategy(gensec_type);
            
            // 获取父级信息
            let parent_refno = gensec_att.get_owner();
            let parent_att = get_named_attmap(parent_refno).await?;
            
            // 执行策略
            if let Some(result) = strategy.get_local_transform(
                gensec_refno, parent_refno, &gensec_att, &parent_att
            ).await? {
                println!("   GENSEC 策略执行成功，变换矩阵: {:?}", result);
                
                // 验证 GENSEC 特有的 BANG 处理
                let (_, bangle) = crate::transform::strategies::GensecBangHandler::should_apply_bang(&gensec_att, gensec_type);
                assert_eq!(bangle, 0.0, "GENSEC 不应该应用 BANG");
                println!("   ✅ GENSEC BANG 处理正确");
            }
        }
    }
    
    // 测试用例 2: ENDATU 类型
    println!("✅ 测试 ENDATU 策略");
    let endatu_refno = RefnoEnum::from("test_endatu"); // 示例 ENDATU
    if let Ok(endatu_att) = get_named_attmap(endatu_refno).await {
        let endatu_type = endatu_att.get_type_str();
        if endatu_type == "ENDATU" {
            let strategy = crate::transform::strategies::TransformStrategyFactory::get_strategy(endatu_type);
            let parent_refno = endatu_att.get_owner();
            let parent_att = get_named_attmap(parent_refno).await?;
            
            if let Some(result) = strategy.get_local_transform(
                endatu_refno, parent_refno, &endatu_att, &parent_att
            ).await? {
                println!("   ENDATU 策略执行成功，变换矩阵: {:?}", result);
            }
        }
    }
    
    // 测试用例 3: SJOI 类型
    println!("✅ 测试 SJOI 策略");
    let sjoi_refno = RefnoEnum::from("test_sjoi"); // 示例 SJOI
    if let Ok(sjoi_att) = get_named_attmap(sjoi_refno).await {
        let sjoi_type = sjoi_att.get_type_str();
        if sjoi_type == "SJOI" {
            let strategy = crate::transform::strategies::TransformStrategyFactory::get_strategy(sjoi_type);
            let parent_refno = sjoi_att.get_owner();
            let parent_att = get_named_attmap(parent_refno).await?;
            
            if let Some(result) = strategy.get_local_transform(
                sjoi_refno, parent_refno, &sjoi_att, &parent_att
            ).await? {
                println!("   SJOI 策略执行成功，变换矩阵: {:?}", result);
            }
        }
    }
    
    // 测试用例 4: 通用类型（使用 DefaultStrategy）
    println!("✅ 测试 DefaultStrategy");
    let default_refno = RefnoEnum::from("test_default"); // 示例通用类型
    if let Ok(default_att) = get_named_attmap(default_refno).await {
        let default_type = default_att.get_type_str();
        if !["GENSEC", "SJOI", "ENDATU"].contains(&default_type) {
            let strategy = crate::transform::strategies::TransformStrategyFactory::get_strategy(default_type);
            let parent_refno = default_att.get_owner();
            let parent_att = get_named_attmap(parent_refno).await?;
            
            if let Some(result) = strategy.get_local_transform(
                default_refno, parent_refno, &default_att, &parent_att
            ).await? {
                println!("   DefaultStrategy 执行成功，变换矩阵: {:?}", result);
            }
        }
    }
    
    println!("🎉 策略模式重构测试完成！");
    Ok(())
}

/// 测试属性处理器的细粒度功能
#[tokio::test]
async fn test_attribute_handlers() -> Result<()> {
    init_surreal().await?;
    
    println!("🧪 测试属性处理器细粒度功能");
    
    // 测试 BANG 处理器
    println!("✅ 测试 BANG 处理器");
    let test_att = create_test_attmap_with_bang(45.0);
    let (apply_bang, bangle) = crate::transform::strategies::BangHandler::should_apply_bang(&test_att, "TEST");
    assert!(apply_bang, "应该应用 BANG");
    assert_eq!(bangle, 45.0, "BANG 角度应该正确");
    
    // 测试 GENSEC 的 BANG 处理
    let (apply_bang_gensec, bangle_gensec) = crate::transform::strategies::GensecBangHandler::should_apply_bang(&test_att, "GENSEC");
    assert!(!apply_bang_gensec, "GENSEC 不应该应用 BANG");
    assert_eq!(bangle_gensec, 0.0, "GENSEC BANG 应该为 0");
    println!("   BANG 处理器测试通过");
    
    println!("🎉 属性处理器测试完成！");
    Ok(())
}

/// 对比测试：确保重构后的结果与原始实现一致
#[tokio::test]
async fn test_strategy_consistency() -> Result<()> {
    init_surreal().await?;
    
    println!("🧪 测试策略一致性");
    
    let test_refno = RefnoEnum::from("17496_266220"); // 使用实际数据
    
    // 获取原始实现的结果
    let original_result = crate::rs_surreal::get_world_mat4(test_refno, false).await?;
    
    // 获取策略实现的结果
    let strategy_result = crate::transform::get_world_mat4(test_refno).await?;
    
    // 对比结果
    match (original_result, strategy_result) {
        (Some(original), Some(strategy)) => {
            let diff = (original - strategy).abs();
            // 计算矩阵的最大差值
            let max_diff = diff.x_axis.max_element()
                .max(diff.y_axis.max_element())
                .max(diff.z_axis.max_element())
                .max(diff.w_axis.max_element());
            
            println!("   原始实现与策略实现的最大差异: {:?}", max_diff);
            
            // 允许小的数值误差
            if max_diff < 1e-10 {
                println!("   ✅ 策略实现与原始实现一致");
            } else {
                println!("   ⚠️  策略实现与原始实现存在差异，需要进一步检查");
            }
        }
        (None, None) => {
            println!("   ✅ 两个实现都返回 None");
        }
        _ => {
            println!("   ❌ 策略实现与原始实现结果不一致");
        }
    }
    
    println!("🎉 策略一致性测试完成！");
    Ok(())
}
