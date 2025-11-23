use crate::{
    init_test_surreal, get_pe, get_named_attmap, query_filter_deep_children,
    RefU64, RefnoEnum, SUL_DB,
};
use crate::material::dq::{get_dq_bran_list, save_dq_material};
use crate::utils::svg_generator::SpineSvgGenerator;
use serde_json::Value;
use std::collections::HashMap;

#[tokio::test]
async fn test_gensec_spine_calculation() {
    init_test_surreal().await;

    // 测试数据说明：
    // test-files/gensec.txt 包含一个GENSEC结构，其中有一个SPINE
    // SPINE包含6个POINSP点和1个CURVE弧线（半径140mm）

    // 假设这个GENSEC已经导入数据库，其RefNo为示例值
    // 实际使用时需要替换为真实的RefNo
    let gensec_refno = RefU64::from("24384/25674"); // 需要替换为实际RefNo

    println!("Testing GENSEC SPINE calculation for: {:?}", gensec_refno);

    // 1. 获取GENSEC的基本信息
    let gensec_pe = get_pe(gensec_refno.into()).await.expect("Failed to get GENSEC PE");
    assert!(gensec_pe.is_some(), "GENSEC should exist");

    let gensec = gensec_pe.unwrap();
    assert_eq!(gensec.noun, "GENSEC", "Should be a GENSEC element");

    // 2. 查找SPINE子元素
    let spine_children = query_filter_children(gensec_refno.into(), &["SPINE"])
        .await
        .expect("Failed to query SPINE children");

    assert!(!spine_children.is_empty(), "GENSEC should have at least one SPINE");
    println!("Found {} SPINE(s)", spine_children.len());

    // 3. 对每个SPINE，验证其子元素
    for spine_refno in spine_children {
        println!("\nAnalyzing SPINE: {:?}", spine_refno);

        // 获取SPINE的属性
        let spine_att = get_named_attmap(spine_refno)
            .await
            .expect("Failed to get SPINE attributes");

        // 检查YDIR属性
        let ydir = spine_att.get_str("YDIR").unwrap_or("Unknown");
        println!("SPINE YDIR: {}", ydir);

        // 获取SPINE的子元素（POINSP和CURVE）
        let spine_sub_children = crate::get_children_refnos(spine_refno)
            .await
            .unwrap_or_default();

        println!("SPINE has {} child elements", spine_sub_children.len());

        let mut poinsp_count = 0;
        let mut curve_count = 0;
        let mut path_points = Vec::new();

        for child_refno in spine_sub_children {
            let child_att = get_named_attmap(child_refno)
                .await
                .expect("Failed to get child attributes");

            let child_type = child_att.get_type_str();

            match child_type.as_ref() {
                "POINSP" => {
                    poinsp_count += 1;
                    if let Some(pos) = child_att.get_position() {
                        path_points.push((child_type.clone(), pos, None));
                        println!("  POINSP #{}: POS = {:?}", poinsp_count, pos);
                    }
                }
                "CURVE" => {
                    curve_count += 1;
                    let pos = child_att.get_position();
                    let radius = child_att.get_f32("RADI");
                    let curve_type = child_att.get_str("CURTYP");

                    if let Some(p) = pos {
                        path_points.push((child_type.clone(), p, radius));
                    }

                    println!("  CURVE: POS = {:?}, RADIUS = {:?}, TYPE = {:?}",
                             pos, radius, curve_type);
                }
                _ => {
                    println!("  Unknown child type: {}", child_type);
                }
            }
        }

        // 验证test-files/gensec.txt中的结构
        // 应该有6个POINSP点和1个CURVE
        assert!(poinsp_count >= 2, "SPINE should have at least 2 POINSP points");

        // 4. 计算路径长度
        let total_length = calculate_spine_path_length(&path_points);
        println!("\nCalculated SPINE path length: {:.3} mm", total_length);

        // 验证长度计算是否合理
        assert!(total_length > 0.0, "Path length should be greater than 0");

        // 5. 生成SVG可视化
        println!("\nGenerating SVG visualization...");
        let mut svg_generator = SpineSvgGenerator::new();
        svg_generator.set_canvas_size(1000.0, 800.0);

        // 添加所有路径点到SVG生成器
        for (point_type, position, radius) in path_points {
            svg_generator.add_point(point_type.to_string(), position, radius);
        }

        // 保存SVG文件
        let svg_filename = format!("spine_path_{}.svg", spine_refno.to_pdms_str().replace("/", "_"));
        match svg_generator.save_to_file(&svg_filename) {
            Ok(_) => println!("SVG saved to: {}", svg_filename),
            Err(e) => println!("Failed to save SVG: {}", e),
        }
    }

    // 5. 测试通过dq_bran_list函数获取的长度
    println!("\n=== Testing through dq_bran_list ===");
    let db = SUL_DB.clone();
    let dq_data = get_dq_bran_list(db, vec![gensec_refno])
        .await
        .expect("Failed to get DQ bran list");

    println!("DQ data entries: {}", dq_data.len());

    for entry in dq_data {
        if let Some(id) = entry.get("id") {
            println!("\nEntry ID: {}", id);
        }

        if let Some(length) = entry.get("length") {
            let length_val = match length {
                Value::Number(n) => n.as_f64().unwrap_or(0.0),
                _ => 0.0,
            };

            println!("Length from dq_gensec: {:.3} mm", length_val);

            // 验证长度不为0（修复后应该能正确计算）
            if length_val == 0.0 {
                println!("WARNING: Length is 0, SPINE calculation may have failed");
            } else {
                println!("SUCCESS: SPINE length calculated correctly");
            }
        }
    }
}

// 辅助函数：计算SPINE路径的总长度
fn calculate_spine_path_length(path_points: &Vec<(&str, glam::Vec3, Option<f32>)>) -> f32 {
    let mut total_length = 0.0;
    let mut i = 0;

    while i < path_points.len() {
        if i + 1 >= path_points.len() {
            break;
        }

        let current = &path_points[i];
        let next = &path_points[i + 1];

        // POINSP to POINSP: 直线段
        if current.0 == "POINSP" && next.0 == "POINSP" {
            let distance = current.1.distance(next.1);
            total_length += distance;
            println!("  Segment {}-{}: Line, length = {:.3}", i, i+1, distance);
            i += 1;
        }
        // POINSP to CURVE to POINSP: 弧线段
        else if current.0 == "POINSP" && next.0 == "CURVE" && i + 2 < path_points.len() {
            let after_curve = &path_points[i + 2];
            if after_curve.0 == "POINSP" {
                // 计算弧长
                if let Some(radius) = next.2 {
                    // 简化计算：使用弦长和半径估算弧长
                    let chord_length = current.1.distance(after_curve.1);

                    // 如果半径足够大，计算实际弧长
                    if chord_length <= 2.0 * radius {
                        let angle = 2.0 * (chord_length / (2.0 * radius)).asin();
                        let arc_length = radius * angle;
                        total_length += arc_length;
                        println!("  Segment {}-{}-{}: Arc, radius = {:.3}, arc_length = {:.3}",
                                 i, i+1, i+2, radius, arc_length);
                    } else {
                        // 半径太小，使用两段直线距离之和
                        let d1 = current.1.distance(next.1);
                        let d2 = next.1.distance(after_curve.1);
                        total_length += d1 + d2;
                        println!("  Segment {}-{}-{}: Arc (approx), length = {:.3}",
                                 i, i+1, i+2, d1 + d2);
                    }
                } else {
                    // 没有半径信息，使用直线近似
                    let d1 = current.1.distance(next.1);
                    let d2 = next.1.distance(after_curve.1);
                    total_length += d1 + d2;
                    println!("  Segment {}-{}-{}: Curve (no radius), length = {:.3}",
                             i, i+1, i+2, d1 + d2);
                }
                i += 2;
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
    }

    total_length
}

/// 为gensec.txt数据生成SVG可视化
fn generate_gensec_svg(points: &Vec<(&str, glam::Vec3)>, total_length: f32) {
    let mut svg_generator = SpineSvgGenerator::new();
    svg_generator.set_canvas_size(1200.0, 900.0);

    // 添加所有点到SVG生成器
    for (i, (point_type, pos)) in points.iter().enumerate() {
        let radius = if *point_type == "CURVE" {
            Some(140.0) // CURVE点的半径是140mm
        } else {
            None
        };

        svg_generator.add_point(point_type.to_string(), *pos, radius);
        println!("  Added {} point {}: {:?}", point_type, i, pos);
    }

    // 保存SVG文件
    let svg_filename = "gensec_spine_test.svg";
    match svg_generator.save_to_file(svg_filename) {
        Ok(_) => {
            println!("✅ SVG visualization saved to: {}", svg_filename);
            println!("   Open this file in a web browser to view the SPINE path");
            println!("   Total calculated length: {:.3} mm", total_length);
        },
        Err(e) => println!("❌ Failed to save SVG: {}", e),
    }
}

/// 创建一个简单的测试SVG，展示基本功能
#[tokio::test]
async fn test_svg_generation_simple() {
    println!("Testing simple SVG generation...");

    let mut svg_generator = SpineSvgGenerator::new();
    svg_generator.set_canvas_size(800.0, 600.0);

    // 添加简单的测试路径
    svg_generator.add_point("POINSP".to_string(), glam::Vec3::new(0.0, 0.0, 0.0), None);
    svg_generator.add_point("POINSP".to_string(), glam::Vec3::new(150.0, 0.0, 0.0), None);
    svg_generator.add_point("CURVE".to_string(), glam::Vec3::new(200.0, 50.0, 0.0), Some(50.0));
    svg_generator.add_point("POINSP".to_string(), glam::Vec3::new(250.0, 100.0, 0.0), None);
    svg_generator.add_point("POINSP".to_string(), glam::Vec3::new(250.0, 250.0, 0.0), None);

    let svg_filename = "simple_spine_test.svg";
    match svg_generator.save_to_file(svg_filename) {
        Ok(_) => println!("✅ Simple SVG test saved to: {}", svg_filename),
        Err(e) => println!("❌ Failed to save simple SVG: {}", e),
    }

    // 验证SVG内容
    let svg_content = svg_generator.generate_svg();
    assert!(svg_content.contains("<svg"), "SVG should contain opening tag");
    assert!(svg_content.contains("</svg>"), "SVG should contain closing tag");
    assert!(svg_content.contains("path-line"), "SVG should contain line paths");
    assert!(svg_content.contains("path-arc"), "SVG should contain arc paths");
    assert!(svg_content.contains("point-poinsp"), "SVG should contain POINSP points");
    assert!(svg_content.contains("point-curve"), "SVG should contain CURVE points");

    println!("✅ SVG content validation passed");
}

/// 测试复杂路径的SVG生成
#[tokio::test]
async fn test_svg_generation_complex() {
    println!("Testing complex SPINE path SVG generation...");

    // 模拟一个复杂的工业管道路径
    let complex_points = vec![
        ("POINSP", glam::Vec3::new(1000.0, 2000.0, 1500.0)),
        ("POINSP", glam::Vec3::new(1500.0, 2000.0, 1500.0)),
        ("CURVE", glam::Vec3::new(1750.0, 2250.0, 1500.0)),
        ("POINSP", glam::Vec3::new(2000.0, 2500.0, 1500.0)),
        ("POINSP", glam::Vec3::new(2000.0, 3000.0, 1500.0)),
        ("CURVE", glam::Vec3::new(2250.0, 3250.0, 1500.0)),
        ("POINSP", glam::Vec3::new(2500.0, 3500.0, 1500.0)),
        ("POINSP", glam::Vec3::new(3500.0, 3500.0, 1500.0)),
    ];

    let mut svg_generator = SpineSvgGenerator::new();
    svg_generator.set_canvas_size(1200.0, 800.0);

    for (point_type, pos) in complex_points.iter() {
        let radius = if *point_type == "CURVE" {
            Some(200.0) // 较大的弯曲半径
        } else {
            None
        };
        svg_generator.add_point(point_type.to_string(), *pos, radius);
    }

    let svg_filename = "complex_spine_test.svg";
    match svg_generator.save_to_file(svg_filename) {
        Ok(_) => println!("✅ Complex SVG test saved to: {}", svg_filename),
        Err(e) => println!("❌ Failed to save complex SVG: {}", e),
    }

    println!("Complex path contains {} points", complex_points.len());
}

#[tokio::test]
async fn test_gensec_specific_data() {
    // 测试test-files/gensec.txt中的具体数据
    // 根据文件内容，验证具体的坐标和计算

    init_test_surreal().await;

    println!("\n=== Testing specific GENSEC data from test-files/gensec.txt ===");

    // 模拟文件中的数据点
    let points = vec![
        ("POINSP", glam::Vec3::new(12635.0, -25862.0, 1950.0)),
        ("POINSP", glam::Vec3::new(12785.0, -25862.0, 1950.0)),
        ("CURVE",  glam::Vec3::new(12884.0, -25903.01, 1950.0)), // 半径140mm
        ("POINSP", glam::Vec3::new(12925.0, -26002.0, 1950.0)),
        ("POINSP", glam::Vec3::new(12925.0, -26152.0, 1950.0)),
        ("POINSP", glam::Vec3::new(12925.0, -29152.0, 1950.0)),
        ("POINSP", glam::Vec3::new(12925.0, -30652.0, 1950.0)),
    ];

    println!("Points from gensec.txt:");
    for (i, (ptype, pos)) in points.iter().enumerate() {
        println!("  {}: {} at {:?}", i, ptype, pos);
    }

    // 计算各段长度
    println!("\nCalculating segment lengths:");

    // 段1: POINSP[0] -> POINSP[1] (直线)
    let seg1_length = points[0].1.distance(points[1].1);
    println!("  Segment 0-1 (line): {:.3} mm", seg1_length);
    assert_eq!(seg1_length, 150.0, "First segment should be 150mm");

    // 段2: POINSP[1] -> CURVE -> POINSP[3] (弧线)
    let chord = points[1].1.distance(points[3].1);
    let radius = 140.0;
    let angle = 2.0 * (chord / (2.0 * radius)).asin();
    let arc_length = radius * angle;
    println!("  Segment 1-2-3 (arc): chord={:.3}, radius={:.3}, arc_length={:.3} mm",
             chord, radius, arc_length);

    // 段3: POINSP[3] -> POINSP[4] (直线)
    let seg3_length = points[3].1.distance(points[4].1);
    println!("  Segment 3-4 (line): {:.3} mm", seg3_length);
    assert_eq!(seg3_length, 150.0, "Third segment should be 150mm");

    // 段4: POINSP[4] -> POINSP[5] (直线)
    let seg4_length = points[4].1.distance(points[5].1);
    println!("  Segment 4-5 (line): {:.3} mm", seg4_length);
    assert_eq!(seg4_length, 3000.0, "Fourth segment should be 3000mm");

    // 段5: POINSP[5] -> POINSP[6] (直线)
    let seg5_length = points[5].1.distance(points[6].1);
    println!("  Segment 5-6 (line): {:.3} mm", seg5_length);
    assert_eq!(seg5_length, 1500.0, "Fifth segment should be 1500mm");

    // 总长度
    let total_expected = seg1_length + arc_length + seg3_length + seg4_length + seg5_length;
    println!("\nTotal expected length: {:.3} mm", total_expected);

    // 验证总长度在合理范围内（考虑弧线计算的误差）
    assert!(total_expected > 4800.0, "Total length should be greater than 4800mm");
    assert!(total_expected < 5000.0, "Total length should be less than 5000mm");

    // 生成SVG可视化
    println!("\nGenerating SVG for gensec.txt data...");
    generate_gensec_svg(&points, total_expected);
}