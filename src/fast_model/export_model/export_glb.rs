use crate::shape::pdms_shape::PlantMesh;
use anyhow::Result;
use glam::Vec3;
use serde_json::json;
use std::path::Path;

/// 计算顶点法线
/// 如果 mesh 没有提供法线，则根据三角形面法线计算顶点法线
fn compute_vertex_normals(vertices: &[Vec3], indices: &[u32]) -> Vec<Vec3> {
    let vertex_count = vertices.len();
    let mut normals = vec![Vec3::ZERO; vertex_count];

    // 遍历每个三角形，累加面法线到顶点
    for tri in indices.chunks_exact(3) {
        let i0 = tri[0] as usize;
        let i1 = tri[1] as usize;
        let i2 = tri[2] as usize;

        if i0 >= vertex_count || i1 >= vertex_count || i2 >= vertex_count {
            continue;
        }

        let v0 = vertices[i0];
        let v1 = vertices[i1];
        let v2 = vertices[i2];

        // 计算面法线 (不归一化，保留面积权重)
        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        let face_normal = edge1.cross(edge2);

        // 累加到每个顶点
        normals[i0] += face_normal;
        normals[i1] += face_normal;
        normals[i2] += face_normal;
    }

    // 归一化所有法线
    for normal in &mut normals {
        let len = normal.length();
        if len > 1e-10 {
            *normal /= len;
        } else {
            *normal = Vec3::Y; // 默认向上
        }
    }

    normals
}

/// 导出单个 PlantMesh 到 GLB 文件
pub fn export_single_mesh_to_glb(mesh: &PlantMesh, output_path: &Path) -> Result<()> {
    if mesh.vertices.is_empty() || mesh.indices.is_empty() {
        return Err(anyhow::anyhow!(
            "无法导出空 mesh：vertices={} indices={}",
            mesh.vertices.len(),
            mesh.indices.len()
        ));
    }

    // 转换 Vec3 为 f32 数组（positions）
    let positions: Vec<f32> = mesh.vertices.iter().flat_map(|v| [v.x, v.y, v.z]).collect();

    // 获取或计算法线
    let normals: Vec<Vec3> =
        if mesh.normals.len() == mesh.vertices.len() && !mesh.normals.is_empty() {
            mesh.normals.clone()
        } else {
            compute_vertex_normals(&mesh.vertices, &mesh.indices)
        };
    let normals_f32: Vec<f32> = normals.iter().flat_map(|n| [n.x, n.y, n.z]).collect();

    export_raw_buffers_to_glb(&positions, &normals_f32, &mesh.indices, output_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_rejects_empty_mesh() {
        let mesh = PlantMesh::default();
        let err = export_single_mesh_to_glb(&mesh, Path::new("/tmp/should_not_write.glb"))
            .expect_err("空 mesh 应该被拒绝导出");
        let _ = err.to_string(); // 仅确保错误可格式化
    }
}

fn write_glb_binary(
    gltf: &serde_json::Value,
    buffer_data: &[u8],
    output_path: &Path,
) -> Result<()> {
    let mut json_bytes = serde_json::to_vec(gltf)?;
    while json_bytes.len() % 4 != 0 {
        json_bytes.push(b' ');
    }

    let mut bin_data = buffer_data.to_vec();
    while bin_data.len() % 4 != 0 {
        bin_data.push(0);
    }

    let total_length = 12 + 8 + json_bytes.len() + 8 + bin_data.len();

    let mut file = std::fs::File::create(output_path)?;
    use std::io::Write;
    file.write_all(b"glTF")?;
    file.write_all(&2u32.to_le_bytes())?;
    file.write_all(&(total_length as u32).to_le_bytes())?;

    file.write_all(&(json_bytes.len() as u32).to_le_bytes())?;
    file.write_all(&0x4E4F534Au32.to_le_bytes())?;
    file.write_all(&json_bytes)?;

    file.write_all(&(bin_data.len() as u32).to_le_bytes())?;
    file.write_all(&0x004E4942u32.to_le_bytes())?;
    file.write_all(&bin_data)?;

    Ok(())
}

/// 直接用原始 buffer（positions/normals/indices）导出 GLB，避免强制依赖 PlantMesh 作为中间层。
///
/// - positions: [x0,y0,z0,x1,y1,z1,...]，长度必须是 3 的倍数
/// - normals: 同上（可由调用方自行决定 per-face 或 per-vertex）
/// - indices: 三角形索引（u32）
pub fn export_raw_buffers_to_glb(
    positions: &[f32],
    normals: &[f32],
    indices: &[u32],
    output_path: &Path,
) -> Result<()> {
    if positions.is_empty() || indices.is_empty() {
        return Err(anyhow::anyhow!(
            "无法导出空 mesh buffers：positions={} indices={}",
            positions.len(),
            indices.len()
        ));
    }
    if positions.len() % 3 != 0 {
        return Err(anyhow::anyhow!(
            "positions 长度必须为 3 的倍数：len={}",
            positions.len()
        ));
    }
    if !normals.is_empty() && normals.len() % 3 != 0 {
        return Err(anyhow::anyhow!(
            "normals 长度必须为 3 的倍数：len={}",
            normals.len()
        ));
    }
    let vertex_count = positions.len() / 3;
    let normal_count = normals.len() / 3;
    if !normals.is_empty() && normal_count != vertex_count {
        return Err(anyhow::anyhow!(
            "normals/positions 顶点数不一致：normals={} positions={}",
            normal_count,
            vertex_count
        ));
    }

    // 构建 buffer 数据
    let mut buffer_data = Vec::new();

    // Positions buffer
    let positions_bytes: Vec<u8> = positions.iter().flat_map(|f| f.to_le_bytes()).collect();
    let positions_offset = buffer_data.len();
    buffer_data.extend_from_slice(&positions_bytes);

    // Normals buffer（允许为空；但 GLB 里缺 NORMAL 有些查看器会渲染很暗）
    let normals_bytes: Vec<u8> = normals.iter().flat_map(|f| f.to_le_bytes()).collect();
    let normals_offset = buffer_data.len();
    buffer_data.extend_from_slice(&normals_bytes);

    // Indices buffer
    let indices_bytes: Vec<u8> = indices.iter().flat_map(|i| i.to_le_bytes()).collect();
    let indices_offset = buffer_data.len();
    buffer_data.extend_from_slice(&indices_bytes);

    // 计算 bounding box（positions）
    let mut min = [f32::MAX, f32::MAX, f32::MAX];
    let mut max = [f32::MIN, f32::MIN, f32::MIN];
    for i in (0..positions.len()).step_by(3) {
        let x = positions[i];
        let y = positions[i + 1];
        let z = positions[i + 2];
        min[0] = min[0].min(x);
        min[1] = min[1].min(y);
        min[2] = min[2].min(z);
        max[0] = max[0].max(x);
        max[1] = max[1].max(y);
        max[2] = max[2].max(z);
    }

    // 构建 glTF JSON
    // accessors: 0=POSITION, 1=NORMAL, 2=indices
    let gltf = json!({
        "asset": {
            "version": "2.0",
            "generator": "AIOS GLB Exporter"
        },
        "scene": 0,
        "scenes": [{"nodes": [0]}],
        "nodes": [{"mesh": 0}],
        "meshes": [{
            "primitives": [{
                "attributes": {
                    "POSITION": 0,
                    "NORMAL": 1
                },
                "indices": 2,
                "mode": 4
            }]
        }],
        "buffers": [{
            "byteLength": buffer_data.len()
        }],
        "bufferViews": [
            {
                "buffer": 0,
                "byteOffset": positions_offset,
                "byteLength": positions_bytes.len(),
                "target": 34962
            },
            {
                "buffer": 0,
                "byteOffset": normals_offset,
                "byteLength": normals_bytes.len(),
                "target": 34962
            },
            {
                "buffer": 0,
                "byteOffset": indices_offset,
                "byteLength": indices_bytes.len(),
                "target": 34963
            }
        ],
        "accessors": [
            {
                "bufferView": 0,
                "componentType": 5126,
                "count": vertex_count,
                "type": "VEC3",
                "min": min,
                "max": max
            },
            {
                "bufferView": 1,
                "componentType": 5126,
                "count": normal_count,
                "type": "VEC3"
            },
            {
                "bufferView": 2,
                "componentType": 5125,
                "count": indices.len(),
                "type": "SCALAR"
            }
        ]
    });

    write_glb_binary(&gltf, &buffer_data, output_path)
}
