//! 测试惰性计算 world_transform
//!
//! 验证当 pe_transform 缓存不存在时，能够自动计算并写入缓存
//!
//! 运行: cargo run --example test_lazy_world_transform

use std::str::FromStr;
use glam::{DMat3, DQuat};
use aios_core::{RefnoEnum, SUL_DB, SurrealQueryExt};
use aios_core::rs_surreal::pe_transform::query_pe_transform;
use aios_core::transform::{get_world_mat4, get_local_mat4};
use aios_core::tool::math_tool::dquat_to_pdms_ori_xyz_str;
use aios_core::rs_surreal::query_ancestor_refnos;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化数据库
    aios_core::init_surreal().await?;
    
    let refno = RefnoEnum::from_str("17496/142306").expect("解析 refno 失败");
    let pe_key = refno.to_table_key("pe_transform");
    
    println!("=== 测试惰性计算 world_transform ===");
    println!("目标: {}", refno);
    println!("PDMS 期望: Y is X and Z is -Y");
    println!("PDMS 期望位置: X -3160mm Y -21150mm Z 5470mm");
    
    // Step 0: 检查当前节点的属性和 local transform
    println!("\n[Step 0] 检查 FITT 元件属性...");
    let att = aios_core::get_named_attmap(refno).await?;
    println!("  TYPE: {:?}", att.get_str("TYPE"));
    println!("  POSL: {:?}", att.get_str("POSL"));
    println!("  DELP: {:?}", att.get_dvec3("DELP"));
    println!("  ZDIS: {:?}", att.get_f64("ZDIS"));
    println!("  POS: {:?}", att.get_dvec3("POS"));
    println!("  ORI: {:?}", att.get_str("ORI"));
    
    // 检查父节点的属性
    let parent_refno = att.get_owner();
    if !parent_refno.is_unset() {
        let parent_att = aios_core::get_named_attmap(parent_refno).await?;
        println!("  父节点 {}: {:?}", parent_refno, parent_att.get_str("TYPE"));
        println!("  父节点 ORI: {:?}", parent_att.get_str("ORI"));
        println!("  父节点 get_rotation(): {:?}", parent_att.get_rotation());
    }
    
    // 检查 PLINE 位置
    let posl = att.get_str("POSL").unwrap_or("");
    if !posl.is_empty() {
        let owner = att.get_owner();
        let plin_owners = aios_core::query_filter_ancestors(owner, &aios_core::consts::HAS_PLIN_TYPES).await?;
        if let Some(plin_owner) = plin_owners.into_iter().next() {
            if let Ok(Some(param)) = aios_core::query_pline(plin_owner, posl.into()).await {
                println!("  PLINE {} 位置: {:?}", posl, param.pt);
                println!("  PLINE {} PLAX: {:?}", posl, param.plax);
            }
        }
    }
    
    // 检查 parent 的 rotation
    let parent_refno_check = att.get_owner();
    let parent_att_check = aios_core::get_named_attmap(parent_refno_check).await?;
    println!("  Parent {} ORI attr: {:?}", parent_refno_check, parent_att_check.get_str("ORI"));
    println!("  Parent {} get_rotation(): {:?}", parent_refno_check, parent_att_check.get_rotation());
    
    println!("\n[Step 0.1] 检查 local transform ORI...");
    let my_local = get_local_mat4(refno).await?;
    if let Some(mat) = my_local {
        let rot_mat = DMat3::from_cols(
            mat.x_axis.truncate(),
            mat.y_axis.truncate(),
            mat.z_axis.truncate(),
        );
        let quat = DQuat::from_mat3(&rot_mat);
        let ori_str = dquat_to_pdms_ori_xyz_str(&quat, true);
        let translation = mat.w_axis.truncate();
        println!("  {} local ORI: {}", refno, ori_str);
        println!("  {} local POS: X {:.0}mm Y {:.0}mm Z {:.0}mm", refno, translation.x, translation.y, translation.z);
    } else {
        println!("  {} local_mat: None (IDENTITY)", refno);
    }
    
    // 检查祖先链
    println!("\n  === 祖先链 (query_ancestor_refnos) ===");
    let ancestors = query_ancestor_refnos(refno).await?;
    println!("  祖先链原始: {:?}", ancestors);
    println!("  当前节点 {} 是否在祖先链中: {}", refno, ancestors.contains(&refno));
    for ancestor in &ancestors {
        let local_mat = get_local_mat4(*ancestor).await?;
        if let Some(mat) = local_mat {
            let rot_mat = DMat3::from_cols(
                mat.x_axis.truncate(),
                mat.y_axis.truncate(),
                mat.z_axis.truncate(),
            );
            let quat = DQuat::from_mat3(&rot_mat);
            let ori_str = dquat_to_pdms_ori_xyz_str(&quat, true);
            let translation = mat.w_axis.truncate();
            println!("  {} ORI: {} | POS: ({:.0}, {:.0}, {:.0})", 
                ancestor, ori_str, translation.x, translation.y, translation.z);
        } else {
            println!("  {} ORI: IDENTITY | POS: (0, 0, 0)", ancestor);
        }
    }
    
    // Step 1: 删除缓存
    println!("\n[Step 1] 删除 pe_transform 缓存...");
    let delete_sql = format!("DELETE {}", pe_key);
    SUL_DB.query_response(&delete_sql).await?;
    println!("  ✅ 已删除 {}", pe_key);
    
    // 验证已删除
    let cache_before = query_pe_transform(refno).await?;
    if cache_before.is_none() || cache_before.as_ref().map(|c| c.world.is_none()).unwrap_or(true) {
        println!("  ✅ 确认缓存已清空");
    } else {
        println!("  ⚠️ 缓存仍存在");
    }
    
    // Step 1.5: 手动计算验证
    println!("\n[Step 1.5] 手动验证累乘逻辑...");
    // 父节点 17496_106028 的 local (因为上层都是 IDENTITY，所以 local=world)
    let parent_refno = RefnoEnum::from_str("17496/106028").expect("解析 refno 失败");
    let parent_local = get_local_mat4(parent_refno).await?.unwrap_or(glam::DMat4::IDENTITY);
    println!("  父节点 {} world matrix:", parent_refno);
    println!("    col0 (X): {:?}", parent_local.x_axis);
    println!("    col1 (Y): {:?}", parent_local.y_axis);
    println!("    col2 (Z): {:?}", parent_local.z_axis);
    println!("    col3 (W): {:?}", parent_local.w_axis);
    
    // 手动累乘: parent_world * child_local
    let child_local = my_local.unwrap_or(glam::DMat4::IDENTITY);
    let manual_world = parent_local * child_local;
    println!("  手动累乘结果 (parent * child):");
    println!("    translation: {:?}", manual_world.w_axis.truncate());
    
    // Step 2: 调用 get_world_mat4 触发惰性计算
    println!("\n[Step 2] 调用 get_world_mat4 触发惰性计算...");
    let world_mat = get_world_mat4(refno, false).await?;
    
    match &world_mat {
        Some(mat) => {
            println!("  ✅ 惰性计算成功!");
            println!("  Translation: {:?}", mat.w_axis.truncate());
        }
        None => {
            println!("  ❌ 惰性计算返回 None");
            return Err(anyhow::anyhow!("惰性计算失败，返回 None"));
        }
    }
    
    // Step 3: 等待异步写入完成
    println!("\n[Step 3] 等待缓存写入...");
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    
    // Step 4: 验证数据库已更新
    println!("\n[Step 4] 验证数据库缓存...");
    let cache_after = query_pe_transform(refno).await?;
    
    match cache_after {
        Some(cache) => {
            if let Some(world) = cache.world {
                println!("  ✅ 数据库缓存已写入!");
                println!("  world_trans.translation: {:?}", world.translation);
                println!("  world_trans.rotation: {:?}", world.rotation);
                println!("  world_trans.scale: {:?}", world.scale);
            } else {
                println!("  ❌ world_trans 仍为 None");
            }
        }
        None => {
            println!("  ⚠️ pe_transform 记录不存在（可能需要更长等待时间）");
        }
    }
    
    println!("\n=== 测试完成 ===");
    Ok(())
}
