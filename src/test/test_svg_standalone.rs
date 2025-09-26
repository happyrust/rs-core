use crate::utils::svg_generator::SpineSvgGenerator;
use glam::Vec3;

/// 简单的独立SVG测试，不依赖数据库
#[tokio::test]
async fn test_simple_svg_generation() {
    println!("Testing simple SVG generation...");

    let mut svg_generator = SpineSvgGenerator::new();
    svg_generator.set_canvas_size(800.0, 600.0);

    // 添加简单的测试路径
    svg_generator.add_point("POINSP".to_string(), Vec3::new(0.0, 0.0, 0.0), None);
    svg_generator.add_point("POINSP".to_string(), Vec3::new(150.0, 0.0, 0.0), None);
    svg_generator.add_point("CURVE".to_string(), Vec3::new(200.0, 50.0, 0.0), Some(50.0));
    svg_generator.add_point("POINSP".to_string(), Vec3::new(250.0, 100.0, 0.0), None);
    svg_generator.add_point("POINSP".to_string(), Vec3::new(250.0, 250.0, 0.0), None);

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

/// 测试gensec.txt数据的SVG生成
#[tokio::test]
async fn test_gensec_data_svg_generation() {
    println!("Testing gensec.txt data SVG generation...");

    // 模拟文件中的数据点
    let points = vec![
        ("POINSP", Vec3::new(12635.0, -25862.0, 1950.0)),
        ("POINSP", Vec3::new(12785.0, -25862.0, 1950.0)),
        ("CURVE",  Vec3::new(12884.0, -25903.01, 1950.0)), // 半径140mm
        ("POINSP", Vec3::new(12925.0, -26002.0, 1950.0)),
        ("POINSP", Vec3::new(12925.0, -26152.0, 1950.0)),
        ("POINSP", Vec3::new(12925.0, -29152.0, 1950.0)),
        ("POINSP", Vec3::new(12925.0, -30652.0, 1950.0)),
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
    assert!(total_expected < 5100.0, "Total length should be less than 5100mm");

    // 生成SVG可视化
    println!("\nGenerating SVG for gensec.txt data...");
    generate_gensec_svg(&points, total_expected);
}

/// 测试复杂路径的SVG生成
#[tokio::test]
async fn test_complex_spine_path_svg() {
    println!("Testing complex SPINE path SVG generation...");

    // 模拟一个复杂的工业管道路径
    let complex_points = vec![
        ("POINSP", Vec3::new(1000.0, 2000.0, 1500.0)),
        ("POINSP", Vec3::new(1500.0, 2000.0, 1500.0)),
        ("CURVE", Vec3::new(1750.0, 2250.0, 1500.0)),
        ("POINSP", Vec3::new(2000.0, 2500.0, 1500.0)),
        ("POINSP", Vec3::new(2000.0, 3000.0, 1500.0)),
        ("CURVE", Vec3::new(2250.0, 3250.0, 1500.0)),
        ("POINSP", Vec3::new(2500.0, 3500.0, 1500.0)),
        ("POINSP", Vec3::new(3500.0, 3500.0, 1500.0)),
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

/// 为gensec.txt数据生成SVG可视化
fn generate_gensec_svg(points: &Vec<(&str, Vec3)>, total_length: f32) {
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