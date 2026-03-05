//! 验证 FITT 17496_142306 的 get_local_mat4 计算结果
//!
//! PDMS 实际值:
//!   WORI: Y is X and Z is -Y
//!   POS wrt /*: X -3160mm Y -21150mm Z 5470mm
//!   Owner STWALL: POS wrt /* = X -3360mm Y -21150mm Z 3290mm
//!
//! 运行: cargo run --example verify_fitt_local_mat4

use aios_core::transform::get_local_mat4;
use aios_core::RefnoEnum;
use glam::DVec3;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    aios_core::init_surreal().await?;

    let fitt_refno = RefnoEnum::from("17496_142306");
    let stwall_refno = RefnoEnum::from("17496_106028");

    // 1. 计算 FITT 的 local_mat（纯策略计算，不读 pe_transform 缓存）
    println!("\n=== FITT 17496_142306 local_mat4 ===");
    let local_mat = get_local_mat4(fitt_refno).await?;
    match &local_mat {
        Some(mat) => {
            let pos = mat.w_axis.truncate();
            let (scale, rot, trans) = mat.to_scale_rotation_translation();
            println!("  translation: {:?}", trans);
            println!("  rotation:    {:?}", rot);
            println!("  scale:       {:?}", scale);
            if pos.length() < 0.01 {
                println!("  ❌ local_mat 是 IDENTITY（bug 未修复）");
            } else {
                println!("  ✅ local_mat 非 IDENTITY");
            }
        }
        None => println!("  ❌ 返回 None"),
    }

    // 2. 计算 STWALL 的 local_mat → world_mat（STWALL 是根级，world ≈ local）
    println!("\n=== STWALL 17496_106028 local_mat4 ===");
    let stwall_local = get_local_mat4(stwall_refno).await?;
    if let Some(mat) = &stwall_local {
        let (_, rot, trans) = mat.to_scale_rotation_translation();
        println!("  translation: {:?}", trans);
        println!("  rotation:    {:?}", rot);
    }

    // 3. 从 STWALL 向上累积 world_mat
    //    先获取 STWALL 的完整 world_mat（通过惰性计算，走策略路径）
    println!("\n=== STWALL world_mat（策略计算） ===");
    let stwall_world = aios_core::transform::get_world_mat4(stwall_refno, false).await?;
    if let Some(mat) = &stwall_world {
        let (_, rot, trans) = mat.to_scale_rotation_translation();
        println!("  translation: {:?}", trans);
        println!("  rotation:    {:?}", rot);
        // 期望: [-3360, -21150, 3290]
    }

    // 4. 手动计算 FITT world = STWALL_world * FITT_local
    if let (Some(parent_world), Some(local)) = (&stwall_world, &local_mat) {
        let computed_world = *parent_world * *local;
        let (_, rot, trans) = computed_world.to_scale_rotation_translation();

        println!("\n=== FITT 计算的 world_mat ===");
        println!("  translation: {:?}", trans);
        println!("  rotation:    {:?}", rot);

        let expected_pos = DVec3::new(-3160.0, -21150.0, 5470.0);
        let diff = (DVec3::new(trans.x, trans.y, trans.z) - expected_pos).length();
        println!("\n=== 对比 PDMS 实际值 ===");
        println!("  PDMS:     {:?}", expected_pos);
        println!("  computed: [{:.1}, {:.1}, {:.1}]", trans.x, trans.y, trans.z);
        println!("  差异:     {:.2}mm", diff);

        if diff < 1.0 {
            println!("  ✅ 验证通过！计算结果与 PDMS 一致");
        } else {
            println!("  ❌ 验证失败，偏差 {:.2}mm", diff);
        }
    }

    Ok(())
}
