use aios_core::RefU64;
use aios_core::spatial::sqlite;
use glam::Vec3;
use parry3d::bounding_volume::Aabb;

/// SQLite 空间查询演示程序
///
/// 展示如何使用 SQLite R-tree 进行空间索引和查询

fn main() -> anyhow::Result<()> {
    println!("🚀 SQLite 空间查询演示程序");

    // 1. 创建 SQLite 连接和表
    println!("\n📊 创建 SQLite R-tree 表...");

    // 确保目录存在
    std::fs::create_dir_all("assets")?;

    // 直接创建数据库连接（不依赖配置文件）
    let conn = rusqlite::Connection::open("assets/demo_spatial.sqlite")?;
    sqlite::create_rtree_table(&conn)?;
    println!("✅ R-tree 表创建成功");

    // 2. 插入测试数据
    println!("\n📦 插入测试空间数据...");
    let test_data = vec![
        (
            RefU64(12345),
            Aabb::new(
                parry3d::math::Point::new(0.0, 0.0, 0.0),
                parry3d::math::Point::new(10.0, 10.0, 10.0),
            ),
            Some("ROOM".to_string()),
        ),
        (
            RefU64(12346),
            Aabb::new(
                parry3d::math::Point::new(5.0, 5.0, 5.0),
                parry3d::math::Point::new(15.0, 15.0, 15.0),
            ),
            Some("PANEL".to_string()),
        ),
        (
            RefU64(12347),
            Aabb::new(
                parry3d::math::Point::new(20.0, 20.0, 20.0),
                parry3d::math::Point::new(30.0, 30.0, 30.0),
            ),
            Some("EQUI".to_string()),
        ),
    ];

    sqlite::insert_or_update_aabbs_batch(&test_data)?;
    println!("✅ 插入了 {} 个空间对象", test_data.len());

    // 3. 点查询测试
    println!("\n🔍 点查询测试:");
    let test_points = vec![
        Vec3::new(5.0, 5.0, 5.0),       // 应该找到两个对象
        Vec3::new(25.0, 25.0, 25.0),    // 应该找到一个对象
        Vec3::new(100.0, 100.0, 100.0), // 应该找不到对象
    ];

    for point in test_points {
        let results = sqlite::query_containing_point_with_conn(&conn, point, 10)?;
        println!("  点 {:?} -> 找到 {} 个对象:", point, results.len());
        for (refno, aabb) in results {
            println!(
                "    RefNo: {}, AABB: [{:.1},{:.1},{:.1}] - [{:.1},{:.1},{:.1}]",
                refno.0,
                aabb.mins.x,
                aabb.mins.y,
                aabb.mins.z,
                aabb.maxs.x,
                aabb.maxs.y,
                aabb.maxs.z
            );
        }
    }

    // 4. 重叠查询测试
    println!("\n🔄 重叠查询测试:");
    let query_aabb = Aabb::new(
        parry3d::math::Point::new(8.0, 8.0, 8.0),
        parry3d::math::Point::new(12.0, 12.0, 12.0),
    );

    let overlap_results = sqlite::query_overlap_with_conn(&conn, &query_aabb, None, Some(10), &[])?;

    println!(
        "  查询区域 [{:.1},{:.1},{:.1}] - [{:.1},{:.1},{:.1}]:",
        query_aabb.mins.x,
        query_aabb.mins.y,
        query_aabb.mins.z,
        query_aabb.maxs.x,
        query_aabb.maxs.y,
        query_aabb.maxs.z
    );
    println!("  找到 {} 个重叠对象:", overlap_results.len());

    for (refno, aabb, noun) in overlap_results {
        println!(
            "    RefNo: {}, 类型: {:?}, AABB: [{:.1},{:.1},{:.1}] - [{:.1},{:.1},{:.1}]",
            refno.0,
            noun.unwrap_or("未知".to_string()),
            aabb.mins.x,
            aabb.mins.y,
            aabb.mins.z,
            aabb.maxs.x,
            aabb.maxs.y,
            aabb.maxs.z
        );
    }

    // 5. K近邻查询测试
    println!("\n🎯 K近邻查询测试:");
    let query_point = Vec3::new(0.0, 0.0, 0.0);
    let knn_results = sqlite::query_knn_with_conn(
        &conn,
        query_point,
        3,          // 查找最近的3个对象
        Some(50.0), // 搜索半径
        None,
    )?;

    println!("  查询点 {:?} 的最近 3 个对象:", query_point);
    for (refno, aabb, distance, noun) in knn_results {
        println!(
            "    RefNo: {}, 距离: {:.2}, 类型: {:?}",
            refno.0,
            distance,
            noun.unwrap_or("未知".to_string())
        );
        println!(
            "      AABB: [{:.1},{:.1},{:.1}] - [{:.1},{:.1},{:.1}]",
            aabb.mins.x, aabb.mins.y, aabb.mins.z, aabb.maxs.x, aabb.maxs.y, aabb.maxs.z
        );
    }

    // 6. 按类型过滤查询
    println!("\n🏷️ 按类型过滤查询:");
    let type_filter = vec!["ROOM".to_string()];
    let filtered_results = sqlite::query_overlap_with_conn(
        &conn,
        &Aabb::new(
            parry3d::math::Point::new(-5.0, -5.0, -5.0),
            parry3d::math::Point::new(35.0, 35.0, 35.0),
        ),
        Some(&type_filter),
        Some(10),
        &[],
    )?;

    println!("  只查找 ROOM 类型的对象:");
    println!("  找到 {} 个 ROOM 对象:", filtered_results.len());
    for (refno, aabb, noun) in filtered_results {
        println!(
            "    RefNo: {}, 类型: {:?}",
            refno.0,
            noun.unwrap_or("未知".to_string())
        );
    }

    // 7. 性能测试
    println!("\n⚡ 性能测试:");
    let start_time = std::time::Instant::now();
    let mut total_results = 0;

    for i in 0..1000 {
        let test_point = Vec3::new(
            (i as f32 % 50.0) - 25.0,
            (i as f32 % 30.0) - 15.0,
            (i as f32 % 20.0) - 10.0,
        );
        let results = sqlite::query_containing_point_with_conn(&conn, test_point, 5)?;
        total_results += results.len();
    }

    let elapsed = start_time.elapsed();
    println!("  执行 1000 次点查询:");
    println!("  总耗时: {:?}", elapsed);
    println!(
        "  平均每次查询: {:.2} ms",
        elapsed.as_millis() as f64 / 1000.0
    );
    println!("  总结果数: {}", total_results);
    println!(
        "  查询吞吐量: {:.0} 查询/秒",
        1000.0 / elapsed.as_secs_f64()
    );

    println!("\n✅ SQLite 空间查询演示完成");
    println!("\n📋 总结:");
    println!("  - SQLite R-tree 提供高效的空间索引");
    println!("  - 支持点查询、重叠查询、K近邻查询");
    println!("  - 支持按类型过滤和排除特定对象");
    println!("  - 查询性能优秀，适合大规模空间数据");

    Ok(())
}
