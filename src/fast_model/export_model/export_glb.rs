use std::path::Path;
use serde_json::json;
use anyhow::Result;
use crate::shape::pdms_shape::PlantMesh;

/// 导出单个 PlantMesh 到 GLB 文件
pub fn export_single_mesh_to_glb(mesh: &PlantMesh, output_path: &Path) -> Result<()> {
    // 转换 Vec3 为 f32 数组
    let positions: Vec<f32> = mesh.vertices.iter().flat_map(|v| [v.x, v.y, v.z]).collect();

    // 构建 buffer 数据
    let mut buffer_data = Vec::new();

    // Positions buffer
    let positions_bytes: Vec<u8> = positions.iter().flat_map(|f| f.to_le_bytes()).collect();
    let positions_offset = buffer_data.len();
    buffer_data.extend_from_slice(&positions_bytes);

    // Indices buffer
    let indices_bytes: Vec<u8> = mesh.indices.iter().flat_map(|i| i.to_le_bytes()).collect();
    let indices_offset = buffer_data.len();
    buffer_data.extend_from_slice(&indices_bytes);

    // 计算 bounding box
    let mut min = [f32::MAX, f32::MAX, f32::MAX];
    let mut max = [f32::MIN, f32::MIN, f32::MIN];
    for v in &mesh.vertices {
        min[0] = min[0].min(v.x);
        min[1] = min[1].min(v.y);
        min[2] = min[2].min(v.z);
        max[0] = max[0].max(v.x);
        max[1] = max[1].max(v.y);
        max[2] = max[2].max(v.z);
    }

    // 构建 glTF JSON
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
                    "POSITION": 0
                },
                "indices": 1,
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
                "byteOffset": indices_offset,
                "byteLength": indices_bytes.len(),
                "target": 34963
            }
        ],
        "accessors": [
            {
                "bufferView": 0,
                "componentType": 5126,
                "count": mesh.vertices.len(),
                "type": "VEC3",
                "min": min,
                "max": max
            },
            {
                "bufferView": 1,
                "componentType": 5125,
                "count": mesh.indices.len(),
                "type": "SCALAR"
            }
        ]
    });

    write_glb_binary(&gltf, &buffer_data, output_path)
}

fn write_glb_binary(gltf: &serde_json::Value, buffer_data: &[u8], output_path: &Path) -> Result<()> {
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
