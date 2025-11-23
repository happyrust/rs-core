use aios_core::*;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化数据库连接
    init_surreal().await?;
    
    let poinsp_refno = RefnoEnum::from("17496_266220");
    
    println!("🔍 深度分析POINSP {} 的方位计算问题", poinsp_refno);
    
    // 1. 获取POINSP的属性和层级关系
    let att = get_named_attmap(poinsp_refno).await?;
    println!("📋 POINSP基本信息:");
    println!("  类型: {}", att.get_type_str());
    println!("  所有者: {:?}", att.get_owner());
    
    // 2. 获取父级信息
    let owner_refno = att.get_owner();
    let owner_att = get_named_attmap(owner_refno).await?;
    let owner_type = owner_att.get_type_str();
    println!("📐 父级 {} 信息:", owner_refno);
    println!("  类型: {}", owner_type);
    
    let (gensec_refno, spine_refno) = if owner_type == "SPINE" {
        let spine_refno = owner_refno;
        let gensec_refno = owner_att.get_owner();
        println!("  这是一个 SPINE 元素，其父级应该是 GENSEC/WALL");
        println!("  GENSEC: {:?}", gensec_refno);
        (gensec_refno, Some(spine_refno))
    } else if owner_type == "GENSEC" || owner_type == "WALL" {
         let gensec_refno = owner_refno;
         // 查找 SPINE 子节点
         let gensec_children = get_children_refnos(gensec_refno).await?;
         let mut s_ref = None;
         for &child_refno in &gensec_children {
             let child_att = get_named_attmap(child_refno).await?;
             if child_att.get_type_str() == "SPINE" {
                 s_ref = Some(child_refno);
                 break;
             }
         }
         (gensec_refno, s_ref)
    } else {
        println!("  ⚠️ 未知的父级类型: {}", owner_type);
        (owner_refno, None)
    };
    
    if let Some(spine_refno) = spine_refno {
        println!("🦴 SPINE {} 信息:", spine_refno);
        let spine_att = get_named_attmap(spine_refno).await?;
        
        // 检查YDIR属性
        if let Some(ydir) = spine_att.get_dvec3("YDIR") {
            println!("  YDIR: {:?}", ydir);
        } else {
            println!("  YDIR: 未设置");
        }
        
        // 4. 获取SPINE的路径点
        let spine_pts = get_spline_pts(gensec_refno).await?;
        println!("  SPINE路径点数: {}", spine_pts.len());
        if spine_pts.len() >= 2 {
            let spine_dir = (spine_pts[1] - spine_pts[0]).normalize();
            println!("  SPINE方向: {:?}", spine_dir);
            
            // 5. 模拟方位计算过程
            println!("\n🔧 方位计算模拟:");
            
            // 使用当前的cal_spine_orientation_basis函数
            let current_quat = cal_spine_orientation_basis(spine_dir, false);
            
            println!("  当前计算的四元数: {:?}", current_quat);
            
            // 计算局部坐标轴
            let local_x = current_quat * glam::DVec3::X;
            let local_y = current_quat * glam::DVec3::Y;
            let local_z = current_quat * glam::DVec3::Z;
            
            println!("  当前局部坐标轴:");
            println!("    X轴: {:?}", local_x);
            println!("    Y轴: {:?}", local_y);
            println!("    Z轴: {:?}", local_z);
            
            // 6. 分析期望的方位
            println!("\n🎯 期望方位分析:");
            println!("  期望 WORI: Y is N 88.958 U and Z is N 0.0451 W 1.0416 D");
            
            // 解析期望方向
            // Y轴: N 88.958 U -> 主要向北(Y), 偏上(Z)
            // 88.958度是与垂直方向的夹角？还是方位角？
            // PDMS "Y is N 88.958 U" 通常意味着 Y轴指向北，但向上偏转了 (90-88.958) 度? 
            // 或者是在 N-U 平面上，与 N 轴夹角 88.958 度?
            // 通常 "D is N 88.958 U" 格式是: D轴在 N-U 平面，偏向 U。
            
            // Z轴: N 0.0451 W 1.0416 D
            // 这是一个混合方向，看起来像是一个未归一化的向量或者带有偏移量的描述
            // W 1.0416 D 可能是指 向西 1.0416度 偏下? 或者是分量比?
            
            // 7. 尝试使用 YDIR 修正计算
            if let Some(ydir) = spine_att.get_dvec3("YDIR") {
                println!("\n🔧 使用YDIR修正计算:");
                let fixed_quat = cal_spine_orientation_basis_with_ydir(spine_dir, Some(ydir), false);
                
                let fixed_x = fixed_quat * glam::DVec3::X;
                let fixed_y = fixed_quat * glam::DVec3::Y;
                let fixed_z = fixed_quat * glam::DVec3::Z;
                
                println!("  修正后局部坐标轴 (World系):");
                println!("    X轴 (E): {:.4}, {:.4}, {:.4}", fixed_x.x, fixed_x.y, fixed_x.z);
                println!("    Y轴 (N): {:.4}, {:.4}, {:.4}", fixed_y.x, fixed_y.y, fixed_y.z);
                println!("    Z轴 (U): {:.4}, {:.4}, {:.4}", fixed_z.x, fixed_z.y, fixed_z.z);
                
                // 检查与期望的匹配程度
                // 期望 Z (Blue) 应该是 SPINE 方向
                // 期望 Z is N 0.0451 W 1.0416 D
                // West is -X, North is +Y, Down is -Z
                // 假设数字是分量或者角度，我们需要先看计算出的 Z 轴指向哪里
                
                println!("  计算出的 Z 轴方向: {:?}", fixed_z);
                // 将其转换为 W/E N/S U/D 描述以便对比
                let we = if fixed_z.x < 0.0 { format!("W {:.4}", -fixed_z.x) } else { format!("E {:.4}", fixed_z.x) };
                let ns = if fixed_z.y < 0.0 { format!("S {:.4}", -fixed_z.y) } else { format!("N {:.4}", fixed_z.y) };
                let ud = if fixed_z.z < 0.0 { format!("D {:.4}", -fixed_z.z) } else { format!("U {:.4}", fixed_z.z) };
                println!("  Z轴方向描述: {} {} {}", we, ns, ud);
            }

            // 8. 验证世界坐标 (WPOS)
            println!("\n🌍 验证世界坐标 (WPOS):");
            println!("  期望 WPOS: W 5375.49mm N 1771.29mm D 2607.01mm");
            // 期望坐标 (PDMS 坐标系: E, N, U)
            // W 5375.49 -> X = -5375.49
            // N 1771.29 -> Y =  1771.29
            // D 2607.01 -> Z = -2607.01
            let expected_pos = glam::DVec3::new(-5375.49, 1771.29, -2607.01);
            println!("  期望坐标 (ENU): {:?}", expected_pos);

            // 获取 POINSP 的局部位置
            if let Some(local_pos) = att.get_position() {
                let local_pos_d = local_pos.as_dvec3();
                println!("  POINSP 局部坐标: {:?}", local_pos_d);

                // 获取 GENSEC 的世界变换
                if let Some(gensec_mat) = get_world_mat4(gensec_refno, false).await? {
                    // 计算世界坐标 = GENSEC_WorldMatrix * POINSP_LocalPos
                    // 注意: POINSP 作为一个点，通常是 geometry 的一部分，
                    // 如果它作为子节点存在于层级树中，其 transform 应该是相对于 GENSEC 的。
                    let calculated_wpos = gensec_mat.transform_point3(local_pos_d);
                    
                    println!("  计算出的 WPOS: {:?}", calculated_wpos);
                    
                    let diff = calculated_wpos - expected_pos;
                    println!("  坐标差异: {:?}", diff);
                    println!("  距离误差: {:.4} mm", diff.length());
                    
                    if diff.length() < 1.0 {
                         println!("  ✅ WPOS 验证通过!");
                    } else {
                         println!("  ❌ WPOS 验证失败，偏差较大");
                         
                         // 调试: 检查是否需要考虑 STRU 的变换或者其他层级
                         // println!("  GENSEC 父级: {:?}", gensec_att.get_owner());
                    }
                } else {
                    println!("  ❌ 无法获取 GENSEC 的世界变换矩阵");
                }
            } else {
                println!("  ❌ POINSP 没有 POS 属性 (局部坐标)");
                // 尝试直接获取 POINSP 的世界变换 (如果系统支持直接查)
                if let Some(poinsp_mat) = get_world_mat4(poinsp_refno, false).await? {
                    let wpos = poinsp_mat.w_axis.truncate();
                    println!("  通过 get_world_mat4 获取的 WPOS: {:?}", wpos);
                }
            }
            
        } else {
            println!("  ❌ SPINE路径点不足2个，无法计算方向");
        }
    } else {
        println!("  ❌ 未找到SPINE子元素");
    }
    
    Ok(())
}
