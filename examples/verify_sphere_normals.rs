//! 验证球体法向量与三角形绕序是否一致

use glam::Vec3;

fn main() {
    // 模拟 generate_sphere_mesh 的逻辑
    let radius = 1.0f32;
    let height = 8usize;
    let radial = 8usize;
    let center = Vec3::ZERO;

    let mut vertices = Vec::new();
    let mut normals = Vec::new();

    // 北极点
    let north_pole = center + Vec3::Z * radius;
    vertices.push(north_pole);
    normals.push(Vec3::Z);

    // 中间纬度环
    for lat in 1..height {
        let v = lat as f32 / height as f32;
        let theta = v * std::f32::consts::PI;
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();

        for lon in 0..radial {
            let phi = std::f32::consts::TAU * lon as f32 / radial as f32;
            let (sin_phi, cos_phi) = phi.sin_cos();

            let normal = Vec3::new(sin_theta * cos_phi, sin_theta * sin_phi, cos_theta);
            let vertex = center + normal * radius;
            vertices.push(vertex);
            normals.push(normal);
        }
    }

    // 南极点
    let south_pole = center - Vec3::Z * radius;
    vertices.push(south_pole);
    normals.push(-Vec3::Z);

    println!("顶点数: {}", vertices.len());
    println!("北极点: {:?}, 法向量: {:?}", vertices[0], normals[0]);
    println!(
        "南极点: {:?}, 法向量: {:?}",
        vertices[vertices.len() - 1],
        normals[vertices.len() - 1]
    );

    // 验证北极扇形三角形
    println!("\n=== 北极扇形三角形验证 ===");
    let first_ring_start = 1usize;
    for lon in 0..2 {
        let next_lon = (lon + 1) % radial;
        let idx0 = 0; // north_pole
        let idx1 = first_ring_start + next_lon; // next
        let idx2 = first_ring_start + lon; // curr

        let v0 = vertices[idx0];
        let v1 = vertices[idx1];
        let v2 = vertices[idx2];

        // 计算三角形几何法向量 (v1-v0) × (v2-v0)
        let e1 = v1 - v0;
        let e2 = v2 - v0;
        let geo_normal = e1.cross(e2).normalize();

        // 三角形中心点
        let tri_center = (v0 + v1 + v2) / 3.0;
        // 期望法向量（从球心指向外）
        let expected_normal = (tri_center - center).normalize();

        // 点积判断方向是否一致
        let dot = geo_normal.dot(expected_normal);

        println!(
            "三角形[{},{},{}]: 几何法向量={:?}, 期望法向量={:?}, dot={:.3} {}",
            idx0,
            idx1,
            idx2,
            geo_normal,
            expected_normal,
            dot,
            if dot > 0.0 {
                "✓ 正确"
            } else {
                "✗ 反向!"
            }
        );
    }

    // 验证中间纬度带
    println!("\n=== 中间纬度带验证 ===");
    let lat = 0;
    let ring_start = first_ring_start + lat * radial;
    let next_ring_start = ring_start + radial;
    for lon in 0..2 {
        let next_lon = (lon + 1) % radial;
        let curr = ring_start + lon;
        let curr_next = ring_start + next_lon;
        let below = next_ring_start + lon;
        let below_next = next_ring_start + next_lon;

        // 第一个三角形 [curr, curr_next, below]
        {
            let v0 = vertices[curr];
            let v1 = vertices[curr_next];
            let v2 = vertices[below];
            let e1 = v1 - v0;
            let e2 = v2 - v0;
            let geo_normal = e1.cross(e2).normalize();
            let tri_center = (v0 + v1 + v2) / 3.0;
            let expected_normal = (tri_center - center).normalize();
            let dot = geo_normal.dot(expected_normal);
            println!(
                "三角形[{},{},{}]: dot={:.3} {}",
                curr,
                curr_next,
                below,
                dot,
                if dot > 0.0 { "✓" } else { "✗ 反向!" }
            );
        }

        // 第二个三角形 [below, curr_next, below_next]
        {
            let v0 = vertices[below];
            let v1 = vertices[curr_next];
            let v2 = vertices[below_next];
            let e1 = v1 - v0;
            let e2 = v2 - v0;
            let geo_normal = e1.cross(e2).normalize();
            let tri_center = (v0 + v1 + v2) / 3.0;
            let expected_normal = (tri_center - center).normalize();
            let dot = geo_normal.dot(expected_normal);
            println!(
                "三角形[{},{},{}]: dot={:.3} {}",
                below,
                curr_next,
                below_next,
                dot,
                if dot > 0.0 { "✓" } else { "✗ 反向!" }
            );
        }
    }

    // 验证南极扇形
    println!("\n=== 南极扇形三角形验证 ===");
    let last_ring_start = first_ring_start + (height - 2) * radial;
    let south_pole_idx = vertices.len() - 1;
    for lon in 0..2 {
        let next_lon = (lon + 1) % radial;
        let idx0 = last_ring_start + lon; // curr
        let idx1 = last_ring_start + next_lon; // next
        let idx2 = south_pole_idx;

        let v0 = vertices[idx0];
        let v1 = vertices[idx1];
        let v2 = vertices[idx2];

        let e1 = v1 - v0;
        let e2 = v2 - v0;
        let geo_normal = e1.cross(e2).normalize();
        let tri_center = (v0 + v1 + v2) / 3.0;
        let expected_normal = (tri_center - center).normalize();
        let dot = geo_normal.dot(expected_normal);

        println!(
            "三角形[{},{},{}]: dot={:.3} {}",
            idx0,
            idx1,
            idx2,
            dot,
            if dot > 0.0 { "✓" } else { "✗ 反向!" }
        );
    }
}
