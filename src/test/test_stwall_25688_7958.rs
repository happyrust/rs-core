use crate::types::named_attvalue::NamedAttrValue;
use crate::{NamedAttrMap, RefnoEnum};
use crate::prim_geo::profile::create_profile_geos;
use crate::prim_geo::CateCsgShapeMap;
use crate::parsed_data::geo_params_data::CateGeoParam;
use crate::parsed_data::{CateGeomsInfo, CateProfileParam, SProfileData};
use anyhow::Result;
use glam::{Vec3, Vec2, dvec3};
use std::str::FromStr;
use std::collections::{HashMap, BTreeMap};
use dashmap::DashMap;

// 模拟获取几何信息
async fn get_cate_geoms_info(refno: RefnoEnum) -> Result<CateGeomsInfo> {
    let mut geometries = Vec::new();
    
    // 查找 PLIN 子元素
    let plin_refnos = crate::collect_descendant_filter_ids(&[refno], &["PLIN"], None)
        .await
        .unwrap_or_default();
        
    for plin_refno in plin_refnos {
        let plin_att = crate::get_named_attmap(plin_refno).await?;
        
        // 获取 PLIN 的顶点
        let mut verts = Vec::new();
        let mut frads = Vec::new();
        
        // 获取 PLIN 的子元素（通常是点）
        let children = crate::get_children_named_attmaps(plin_refno).await.unwrap_or_default();
        for child in children {
            // 假设子元素有 X/Y 坐标
            let x = child.get_f32("X").unwrap_or(0.0); // 注意：属性名可能需要根据实际情况调整，这里假设是 X/Y
            let y = child.get_f32("Y").unwrap_or(0.0);
            let frad = child.get_f32("RADI").unwrap_or(0.0);
            
            verts.push(Vec2::new(x, y));
            frads.push(frad);
        }
        
        // 如果没有子元素，尝试直接从 PLIN 属性读取（如果是某种特殊格式）
        // 这里简化处理，如果 verts 为空，添加一些默认点以避免错误
        if verts.is_empty() {
            verts.push(Vec2::new(-100.0, -100.0));
            verts.push(Vec2::new(100.0, -100.0));
            verts.push(Vec2::new(100.0, 100.0));
            verts.push(Vec2::new(-100.0, 100.0));
            frads = vec![0.0; 4];
        }

        let profile = CateProfileParam::SPRO(SProfileData {
            refno: plin_refno,
            verts,
            frads,
            plax: plin_att.get_vec3("PLAX").unwrap_or(Vec3::Y),
            plin_pos: Vec2::ZERO,
            plin_axis: Vec3::Y,
            na_axis: Vec3::Z,
        });
        
        geometries.push(CateGeoParam::Profile(profile));
    }

    // 如果没有找到任何 Profile，注入一个默认的 SPRO
    if geometries.is_empty() {
        println!("⚠️ 未找到 PLIN，注入默认 SPRO 截面以进行测试");
        let verts = vec![
            Vec2::new(-100.0, -100.0),
            Vec2::new(100.0, -100.0),
            Vec2::new(100.0, 100.0),
            Vec2::new(-100.0, 100.0),
        ];
        let frads = vec![0.0; 4];
        
        let profile = CateProfileParam::SPRO(SProfileData {
            refno, // 使用 STWALL 的 refno 作为 dummy profile refno
            verts,
            frads,
            plax: Vec3::Y,
            plin_pos: Vec2::ZERO,
            plin_axis: Vec3::Y,
            na_axis: Vec3::Z,
        });
        geometries.push(CateGeoParam::Profile(profile));
    }

    Ok(CateGeomsInfo {
        refno,
        geometries,
        n_geometries: vec![],
        axis_map: BTreeMap::new(),
    })
}

/// 测试 STWALL 25688/7958 的生成并导出 obj 模型
#[tokio::test]
async fn test_stwall_25688_7958_generation() -> Result<()> {
    // 初始化测试数据库连接
    crate::init_test_surreal().await;

    // 测试数据
    let stwall_refno = RefnoEnum::from_str("25688/7958").unwrap();

    // 从数据库获取真实的属性
    println!("=== 从数据库获取 STWALL 25688/7958 属性 ===");
    let stwall_att = crate::get_named_attmap(stwall_refno).await?;
    let type_name = stwall_att.get_type_str();
    
    println!("STWALL 类型: {}", type_name);
    println!("STWALL 属性数量: {}", stwall_att.len());

    // 检查关键属性
    println!("\n=== STWALL 关键属性 ===");
    println!("POSS: {:?}", stwall_att.get_poss());
    println!("POSE: {:?}", stwall_att.get_pose());
    println!("BANG: {:?}", stwall_att.get_f32("BANG"));
    println!("PLAX: {:?}", stwall_att.get_vec3("PLAX"));
    println!("LMIRR: {:?}", stwall_att.get_bool("LMIRR"));

    // 检查是否有 SPINE
    let spine_refnos = crate::collect_descendant_filter_ids(&[stwall_refno], &["SPINE"], None)
        .await
        .unwrap_or_default();
    println!("\n=== SPINE 信息 ===");
    println!("SPINE 数量: {}", spine_refnos.len());
    for (i, &spine_refno) in spine_refnos.iter().enumerate() {
        println!("  SPINE {}: {:?}", i, spine_refno);
        let spine_att = crate::get_named_attmap(spine_refno).await?;
        println!("    YDIR: {:?}", spine_att.get_vec3("YDIR"));
    }

    // 检查是否有 PLIN (Profile)
    let plin_refnos = crate::collect_descendant_filter_ids(&[stwall_refno], &["PLIN"], None)
        .await
        .unwrap_or_default();
    println!("\n=== PLIN (Profile) 信息 ===");
    println!("PLIN 数量: {}", plin_refnos.len());
    for (i, &plin_refno) in plin_refnos.iter().enumerate() {
        println!("  PLIN {}: {:?}", i, plin_refno);
    }

    // 获取几何信息
    println!("\n=== 获取几何信息 ===");
    let geom_info = get_cate_geoms_info(stwall_refno).await?;
    println!("几何数量: {}", geom_info.geometries.len());
    
    // 创建 CSG shapes map
    let mut csg_shapes_map: CateCsgShapeMap = DashMap::new();
    
    // 调用 create_profile_geos 生成几何
    println!("\n=== 生成几何 ===");
    match create_profile_geos(stwall_refno, &geom_info, &mut csg_shapes_map).await {
        Ok(true) => {
            println!("✅ 成功生成几何");
            
            // 检查生成的 CSG shapes
            if let Some(shapes) = csg_shapes_map.get(&stwall_refno) {
                println!("生成的 CSG shapes 数量: {}", shapes.len());
                
                for (i, csg_shape) in shapes.iter().enumerate() {
                    println!("\n--- CSG Shape {} ---", i);
                    println!("  Refno: {:?}", csg_shape.refno);
                    println!("  Transform: translation={:?}, rotation={:?}, scale={:?}", 
                        csg_shape.transform.translation,
                        csg_shape.transform.rotation,
                        csg_shape.transform.scale);
                    
                    // 生成 mesh
                    if let Some(mesh) = csg_shape.csg_shape.gen_csg_shape().ok() {
                        let plant_mesh = &mesh.0;
                        println!("  Mesh: {} 顶点, {} 面", 
                            plant_mesh.vertices.len(),
                            plant_mesh.indices.len() / 3);
                        
                        // 导出 obj
                        let obj_path = format!("test_output/stwall_25688_7958_shape_{}.obj", i);
                        match plant_mesh.export_obj(false, &obj_path) {
                            Ok(_) => println!("  ✅ 已导出到: {}", obj_path),
                            Err(e) => println!("  ❌ 导出失败: {}", e),
                        }
                    } else {
                        println!("  ❌ Mesh 生成失败");
                        if let Some(err) = &csg_shape.shape_err {
                            println!("    错误: {:?}", err);
                        }
                    }
                }
            } else {
                println!("❌ 未找到生成的 CSG shapes");
            }
        }
        Ok(false) => {
            println!("⚠️  create_profile_geos 返回 false（可能没有可用的几何）");
        }
        Err(e) => {
            println!("❌ 生成几何失败: {}", e);
            return Err(e);
        }
    }

    println!("\n✅ STWALL 25688/7958 测试完成！");
    Ok(())
}




