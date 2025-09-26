use crate::utils::svg_generator::SpineSvgGenerator;
use glam::Vec3;

/// 专门测试弧线效果的演示
#[tokio::test]
async fn test_arc_visualization_demo() {
    println!("Creating arc visualization demo...");

    let mut svg_generator = SpineSvgGenerator::new();
    svg_generator.set_canvas_size(1000.0, 800.0);

    // 创建几个不同半径的弧线来展示效果
    println!("Adding different arc examples:");

    // 示例1: 小半径弧线
    svg_generator.add_point("POINSP".to_string(), Vec3::new(100.0, 100.0, 0.0), None);
    svg_generator.add_point("POINSP".to_string(), Vec3::new(300.0, 100.0, 0.0), None);
    svg_generator.add_point("CURVE".to_string(), Vec3::new(400.0, 200.0, 0.0), Some(75.0));
    svg_generator.add_point("POINSP".to_string(), Vec3::new(500.0, 300.0, 0.0), None);
    println!("  Added small radius arc (R=75)");

    // 示例2: 中等半径弧线
    svg_generator.add_point("POINSP".to_string(), Vec3::new(600.0, 100.0, 0.0), None);
    svg_generator.add_point("CURVE".to_string(), Vec3::new(700.0, 200.0, 0.0), Some(150.0));
    svg_generator.add_point("POINSP".to_string(), Vec3::new(800.0, 300.0, 0.0), None);
    println!("  Added medium radius arc (R=150)");

    // 示例3: 大半径弧线
    svg_generator.add_point("POINSP".to_string(), Vec3::new(200.0, 400.0, 0.0), None);
    svg_generator.add_point("CURVE".to_string(), Vec3::new(400.0, 450.0, 0.0), Some(300.0));
    svg_generator.add_point("POINSP".to_string(), Vec3::new(600.0, 500.0, 0.0), None);
    println!("  Added large radius arc (R=300)");

    // 示例4: 垂直弧线
    svg_generator.add_point("POINSP".to_string(), Vec3::new(100.0, 600.0, 0.0), None);
    svg_generator.add_point("CURVE".to_string(), Vec3::new(150.0, 700.0, 0.0), Some(100.0));
    svg_generator.add_point("POINSP".to_string(), Vec3::new(200.0, 800.0, 0.0), None);
    println!("  Added vertical arc (R=100)");

    let svg_filename = "arc_demo.svg";
    match svg_generator.save_to_file(svg_filename) {
        Ok(_) => {
            println!("✅ Arc demonstration saved to: {}", svg_filename);
            println!("   This file shows various arc shapes with different radii");
            println!("   You should now see clear curved paths instead of straight lines");
        },
        Err(e) => println!("❌ Failed to save arc demo: {}", e),
    }

    // 验证SVG内容包含弧线
    let svg_content = svg_generator.generate_svg();
    assert!(svg_content.contains("path-arc"), "Demo should contain arc paths");
    println!("✅ Arc demo validation passed");
}