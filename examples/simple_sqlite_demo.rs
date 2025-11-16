use glam::Vec3;
use parry3d::bounding_volume::Aabb;
use rusqlite::{Connection, params};

/// 简单的 SQLite R-tree 空间查询演示
///
/// 直接使用 rusqlite 展示空间索引的工作原理

fn main() -> anyhow::Result<()> {
    println!("🚀 SQLite R-tree 空间查询演示");

    // 1. 创建内存数据库和 R-tree 表
    println!("\n📊 创建 R-tree 表...");
    let conn = Connection::open_in_memory()?;

    // 创建 R-tree 虚拟表
    conn.execute(
        "CREATE VIRTUAL TABLE spatial_index USING rtree(
            id INTEGER PRIMARY KEY,
            min_x REAL, max_x REAL,
            min_y REAL, max_y REAL,
            min_z REAL, max_z REAL
        )",
        [],
    )?;

    // 创建元数据表
    conn.execute(
        "CREATE TABLE objects (
            id INTEGER PRIMARY KEY,
            name TEXT,
            type TEXT
        )",
        [],
    )?;

    println!("✅ R-tree 表创建成功");

    // 2. 插入测试数据
    println!("\n📦 插入测试空间数据...");

    let test_objects = vec![
        (1, "房间A", "ROOM", 0.0, 10.0, 0.0, 10.0, 0.0, 3.0),
        (2, "房间B", "ROOM", 10.0, 20.0, 0.0, 10.0, 0.0, 3.0),
        (3, "设备1", "EQUI", 2.0, 4.0, 2.0, 4.0, 0.5, 2.5),
        (4, "管道1", "PIPE", 5.0, 15.0, 5.0, 6.0, 1.0, 1.5),
        (5, "面板1", "PANEL", 9.5, 10.5, 0.0, 10.0, 0.0, 3.0),
    ];

    for (id, name, obj_type, min_x, max_x, min_y, max_y, min_z, max_z) in &test_objects {
        // 插入空间索引
        conn.execute(
            "INSERT INTO spatial_index (id, min_x, max_x, min_y, max_y, min_z, max_z)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, min_x, max_x, min_y, max_y, min_z, max_z],
        )?;

        // 插入元数据
        conn.execute(
            "INSERT INTO objects (id, name, type) VALUES (?1, ?2, ?3)",
            params![id, name, obj_type],
        )?;
    }

    println!("✅ 插入了 {} 个空间对象", test_objects.len());

    // 3. 点查询测试
    println!("\n🔍 点查询测试:");
    let test_points = vec![
        (5.0, 5.0, 1.5),   // 应该在房间A和管道1中
        (15.0, 5.0, 1.5),  // 应该在房间B中
        (3.0, 3.0, 1.5),   // 应该在房间A和设备1中
        (25.0, 25.0, 1.5), // 不在任何对象中
    ];

    for (x, y, z) in test_points {
        println!("  查询点 ({:.1}, {:.1}, {:.1}):", x, y, z);

        let mut stmt = conn.prepare(
            "SELECT s.id, o.name, o.type, s.min_x, s.max_x, s.min_y, s.max_y, s.min_z, s.max_z
             FROM spatial_index s
             JOIN objects o ON s.id = o.id
             WHERE s.min_x <= ?1 AND s.max_x >= ?1
               AND s.min_y <= ?2 AND s.max_y >= ?2
               AND s.min_z <= ?3 AND s.max_z >= ?3",
        )?;

        let rows = stmt.query_map(params![x, y, z], |row| {
            Ok((
                row.get::<_, i32>(0)?,    // id
                row.get::<_, String>(1)?, // name
                row.get::<_, String>(2)?, // type
                row.get::<_, f64>(3)?,    // min_x
                row.get::<_, f64>(4)?,    // max_x
                row.get::<_, f64>(5)?,    // min_y
                row.get::<_, f64>(6)?,    // max_y
                row.get::<_, f64>(7)?,    // min_z
                row.get::<_, f64>(8)?,    // max_z
            ))
        })?;

        let mut count = 0;
        for row in rows {
            let (id, name, obj_type, min_x, max_x, min_y, max_y, min_z, max_z) = row?;
            println!("    -> {} (ID: {}, 类型: {})", name, id, obj_type);
            println!(
                "       包围盒: [{:.1},{:.1},{:.1}] - [{:.1},{:.1},{:.1}]",
                min_x, min_y, min_z, max_x, max_y, max_z
            );
            count += 1;
        }

        if count == 0 {
            println!("    -> 未找到包含该点的对象");
        }
        println!();
    }

    // 4. 重叠查询测试
    println!("🔄 重叠查询测试:");
    let query_box = (8.0, 12.0, 4.0, 8.0, 0.0, 2.0); // (min_x, max_x, min_y, max_y, min_z, max_z)

    println!(
        "  查询区域: [{:.1},{:.1},{:.1}] - [{:.1},{:.1},{:.1}]",
        query_box.0, query_box.2, query_box.4, query_box.1, query_box.3, query_box.5
    );

    let mut stmt = conn.prepare(
        "SELECT s.id, o.name, o.type
         FROM spatial_index s
         JOIN objects o ON s.id = o.id
         WHERE s.max_x >= ?1 AND s.min_x <= ?2
           AND s.max_y >= ?3 AND s.min_y <= ?4
           AND s.max_z >= ?5 AND s.min_z <= ?6",
    )?;

    let rows = stmt.query_map(
        params![
            query_box.0,
            query_box.1,
            query_box.2,
            query_box.3,
            query_box.4,
            query_box.5
        ],
        |row| {
            Ok((
                row.get::<_, i32>(0)?,    // id
                row.get::<_, String>(1)?, // name
                row.get::<_, String>(2)?, // type
            ))
        },
    )?;

    println!("  重叠的对象:");
    let mut count = 0;
    for row in rows {
        let (id, name, obj_type) = row?;
        println!("    -> {} (ID: {}, 类型: {})", name, id, obj_type);
        count += 1;
    }

    if count == 0 {
        println!("    -> 未找到重叠的对象");
    }

    // 5. 按类型查询
    println!("\n🏷️ 按类型查询 (只查找 ROOM):");
    let mut stmt = conn.prepare(
        "SELECT s.id, o.name, s.min_x, s.max_x, s.min_y, s.max_y, s.min_z, s.max_z
         FROM spatial_index s
         JOIN objects o ON s.id = o.id
         WHERE o.type = 'ROOM'",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i32>(0)?,    // id
            row.get::<_, String>(1)?, // name
            row.get::<_, f64>(2)?,    // min_x
            row.get::<_, f64>(3)?,    // max_x
            row.get::<_, f64>(4)?,    // min_y
            row.get::<_, f64>(5)?,    // max_y
            row.get::<_, f64>(6)?,    // min_z
            row.get::<_, f64>(7)?,    // max_z
        ))
    })?;

    for row in rows {
        let (id, name, min_x, max_x, min_y, max_y, min_z, max_z) = row?;
        println!("  -> {} (ID: {})", name, id);
        println!(
            "     包围盒: [{:.1},{:.1},{:.1}] - [{:.1},{:.1},{:.1}]",
            min_x, min_y, min_z, max_x, max_y, max_z
        );
        println!(
            "     体积: {:.1} 立方米",
            (max_x - min_x) * (max_y - min_y) * (max_z - min_z)
        );
    }

    // 6. 性能测试
    println!("\n⚡ 性能测试:");
    let start_time = std::time::Instant::now();
    let mut total_results = 0;

    let mut stmt = conn.prepare(
        "SELECT COUNT(*) FROM spatial_index s
         WHERE s.min_x <= ?1 AND s.max_x >= ?1
           AND s.min_y <= ?2 AND s.max_y >= ?2
           AND s.min_z <= ?3 AND s.max_z >= ?3",
    )?;

    for i in 0..1000 {
        let x = (i as f64 % 20.0) - 5.0;
        let y = (i as f64 % 15.0) - 2.0;
        let z = (i as f64 % 4.0) - 1.0;

        let count: i32 = stmt.query_row(params![x, y, z], |row| row.get(0))?;
        total_results += count;
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

    println!("\n✅ SQLite R-tree 空间查询演示完成");
    println!("\n📋 总结:");
    println!("  - SQLite R-tree 是一个虚拟表，专门用于空间索引");
    println!("  - 支持高效的包围盒查询和重叠检测");
    println!("  - 可以与普通表 JOIN 来获取额外的元数据");
    println!("  - 查询性能优秀，适合房间计算等空间应用");
    println!("  - 房间系统正是基于这种技术实现快速空间查询");

    Ok(())
}
