use crate::utils::svg_generator::SpineSvgGenerator;
use glam::Vec3;

/// 测试无标签的干净SVG输出
#[tokio::test]
async fn test_clean_svg_without_labels() {
    println!("Testing clean SVG generation without labels...");

    let mut svg_generator = SpineSvgGenerator::new();
    svg_generator.set_canvas_size(1200.0, 900.0);

    // 设置为不显示标签、坐标和图例
    svg_generator.set_display_options(false, false, false);

    // 使用gensec.txt的数据
    let points = vec![
        ("POINSP", Vec3::new(12635.0, -25862.0, 1950.0)),
        ("POINSP", Vec3::new(12785.0, -25862.0, 1950.0)),
        ("CURVE", Vec3::new(12884.0, -25903.01, 1950.0)), // 半径140mm
        ("POINSP", Vec3::new(12925.0, -26002.0, 1950.0)),
        ("POINSP", Vec3::new(12925.0, -26152.0, 1950.0)),
        ("POINSP", Vec3::new(12925.0, -29152.0, 1950.0)),
        ("POINSP", Vec3::new(12925.0, -30652.0, 1950.0)),
    ];

    println!("Adding path points without labels:");
    for (i, (point_type, pos)) in points.iter().enumerate() {
        let radius = if *point_type == "CURVE" {
            Some(140.0)
        } else {
            None
        };

        svg_generator.add_point(point_type.to_string(), *pos, radius);
        println!("  {}: {} at {:?}", i, point_type, pos);
    }

    let svg_filename = "gensec_spine_clean.svg";
    match svg_generator.save_to_file(svg_filename) {
        Ok(_) => {
            println!("✅ Clean SVG saved to: {}", svg_filename);
            println!("   This SVG contains only the path lines and points, no labels or text");
        }
        Err(e) => println!("❌ Failed to save clean SVG: {}", e),
    }

    // 验证SVG不包含文本标签
    let svg_content = svg_generator.generate_svg();
    assert!(
        svg_content.contains("path-line"),
        "Should contain line paths"
    );
    assert!(svg_content.contains("path-arc"), "Should contain arc paths");
    assert!(
        svg_content.contains("point-poinsp"),
        "Should contain POINSP points"
    );
    assert!(
        svg_content.contains("point-curve"),
        "Should contain CURVE points"
    );

    // 验证不包含长度标签
    assert!(
        !svg_content.contains("150.0mm"),
        "Should not contain length labels"
    );
    assert!(
        !svg_content.contains("Arc:"),
        "Should not contain arc labels"
    );
    assert!(
        !svg_content.contains("直线"),
        "Should not contain Chinese text '直线'"
    );
    assert!(
        !svg_content.contains("弧线"),
        "Should not contain Chinese text '弧线'"
    );

    println!("✅ Clean SVG validation passed - no labels found");
}

/// 测试只显示路径不显示其他元素的简单SVG
#[tokio::test]
async fn test_minimal_svg() {
    println!("Testing minimal SVG with paths only...");

    let mut svg_generator = SpineSvgGenerator::new();
    svg_generator.set_canvas_size(800.0, 600.0);

    // 完全关闭所有标签和装饰
    svg_generator.set_display_options(false, false, false);

    // 简单的L形路径
    svg_generator.add_point("POINSP".to_string(), Vec3::new(100.0, 100.0, 0.0), None);
    svg_generator.add_point("POINSP".to_string(), Vec3::new(300.0, 100.0, 0.0), None);
    svg_generator.add_point(
        "CURVE".to_string(),
        Vec3::new(400.0, 200.0, 0.0),
        Some(100.0),
    );
    svg_generator.add_point("POINSP".to_string(), Vec3::new(500.0, 300.0, 0.0), None);

    let svg_filename = "minimal_spine.svg";
    match svg_generator.save_to_file(svg_filename) {
        Ok(_) => {
            println!("✅ Minimal SVG saved to: {}", svg_filename);
            println!("   This SVG shows only the essential path geometry");
        }
        Err(e) => println!("❌ Failed to save minimal SVG: {}", e),
    }

    println!("✅ Minimal SVG test completed");
}

/// 测试自定义显示选项
#[tokio::test]
async fn test_custom_display_options() {
    println!("Testing custom display options...");

    let mut svg_generator = SpineSvgGenerator::new();
    svg_generator.set_canvas_size(1000.0, 700.0);

    // 只显示坐标，不显示长度标签和图例
    svg_generator.set_display_options(false, true, false);

    // 添加测试路径
    svg_generator.add_point("POINSP".to_string(), Vec3::new(0.0, 0.0, 0.0), None);
    svg_generator.add_point("POINSP".to_string(), Vec3::new(200.0, 0.0, 0.0), None);
    svg_generator.add_point(
        "CURVE".to_string(),
        Vec3::new(300.0, 100.0, 0.0),
        Some(75.0),
    );
    svg_generator.add_point("POINSP".to_string(), Vec3::new(400.0, 200.0, 0.0), None);

    let svg_filename = "custom_options_spine.svg";
    match svg_generator.save_to_file(svg_filename) {
        Ok(_) => {
            println!("✅ Custom options SVG saved to: {}", svg_filename);
            println!("   This SVG shows coordinates but no length labels or legend");
        }
        Err(e) => println!("❌ Failed to save custom options SVG: {}", e),
    }

    // 验证包含坐标但不包含长度标签
    let svg_content = svg_generator.generate_svg();
    assert!(
        svg_content.contains("(0,0)"),
        "Should contain coordinate labels"
    );
    assert!(
        !svg_content.contains("mm"),
        "Should not contain length labels"
    );
    assert!(
        !svg_content.contains("SPINE路径信息"),
        "Should not contain legend"
    );

    println!("✅ Custom display options test completed");
}
