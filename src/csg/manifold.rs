use std::collections::HashMap;
use std::path::Path;

use glam::{DMat4, Vec2, Vec3};
use manifold_rs::{Manifold, Mesh};

/// 布尔操作类型
#[derive(Clone, Copy, Debug)]
pub enum ManifoldOpType {
    Union,
    Intersection,
    Difference,
}

/// 2D 截面，用于生成 3D 实体
pub struct ManifoldCrossSectionRust {
    /// 多边形数据，格式为 [x0, y0, x1, y1, ...]
    polygon_data: Vec<f64>,
}

impl ManifoldCrossSectionRust {
    /// 根据 2d 的点生成 ManifoldCrossSectionRust
    pub fn from_points(pts: &[Vec2]) -> Self {
        let mut polygon_data = Vec::with_capacity(pts.len() * 2);
        for p in pts {
            polygon_data.push(p.x as f64);
            polygon_data.push(p.y as f64);
        }
        Self { polygon_data }
    }

    /// 拉伸成 manifold
    pub fn extrude(&self, height: f32, _slices: u32) -> ManifoldRust {
        let polygon_slice: &[f64] = &self.polygon_data;
        let multi_polygon: &[&[f64]] = &[polygon_slice];
        let manifold = Manifold::extrude(multi_polygon, height as f64, 1, 0.0, 1.0, 1.0);
        ManifoldRust { inner: manifold }
    }

    /// 旋转成 manifold
    pub fn extrude_rotate(&self, segments: i32, angle: f32) -> ManifoldRust {
        let polygon_slice: &[f64] = &self.polygon_data;
        let multi_polygon: &[&[f64]] = &[polygon_slice];
        let manifold = Manifold::revolve(multi_polygon, segments as u32, angle as f64);
        ManifoldRust { inner: manifold }
    }
}

/// Manifold 的 Rust 封装
pub struct ManifoldRust {
    pub inner: Manifold,
}

unsafe impl Send for ManifoldRust {}

impl Clone for ManifoldRust {
    fn clone(&self) -> Self {
        // manifold-rs 的 Manifold 不支持 Clone，需要通过 mesh 转换
        let mesh = self.inner.to_mesh();
        let vertices = mesh.vertices();
        let indices = mesh.indices();
        let new_mesh = Mesh::new(&vertices, &indices);
        Self {
            inner: new_mesh.to_manifold(),
        }
    }
}

impl ManifoldRust {
    pub fn new() -> Self {
        // 使用极小的 cube 替代 empty()，因为 manifold-rs 没有 empty() 方法
        Self {
            inner: Manifold::cube(1e-10, 1e-10, 1e-10),
        }
    }

    /// 原生 BOX：直接使用 Manifold::cube 构造，中心在原点
    ///
    /// 约定与 `unit_box_mesh()` 一致：尺寸 (x, y, z)，中心 (0,0,0)。
    /// Manifold::cube 默认从原点到 (x,y,z)，需平移使中心归零。
    pub fn native_box(x: f64, y: f64, z: f64) -> Self {
        let cube = Manifold::cube(x, y, z);
        Self {
            inner: cube.translate(-x / 2.0, -y / 2.0, -z / 2.0),
        }
    }

    /// 原生圆柱：直接使用 Manifold::cylinder 构造，底面在 z=0
    ///
    /// 约定与 `unit_cylinder_mesh()` 一致：半径 `radius`，底面 z=0，顶面 z=`height`。
    pub fn native_cylinder(radius: f64, height: f64, segments: u32) -> Self {
        Self {
            inner: Manifold::cylinder(radius, radius, height, segments),
        }
    }

    /// 原生球体：直接使用 Manifold::sphere 构造，中心在原点
    ///
    /// 约定与 `unit_sphere_mesh()` 一致：中心 (0,0,0)，半径 `radius`。
    pub fn native_sphere(radius: f64, segments: u32) -> Self {
        Self {
            inner: Manifold::sphere(radius, segments),
        }
    }

    /// 对 Manifold 应用 4x4 变换矩阵
    ///
    /// 通过 mesh 提取 → f64 变换 → 重建 Manifold 实现通用仿射变换。
    /// 原生 Manifold 的 mesh 已保证流形拓扑，变换后无需焊接。
    pub fn apply_transform(&self, mat: DMat4) -> Self {
        let mesh = self.get_mesh();
        if mesh.vertices.is_empty() || mesh.indices.is_empty() {
            return self.clone();
        }
        let mut transformed: Vec<f32> = Vec::with_capacity(mesh.vertices.len());
        for i in (0..mesh.vertices.len()).step_by(3) {
            let pt = mat.transform_point3(glam::DVec3::new(
                mesh.vertices[i] as f64,
                mesh.vertices[i + 1] as f64,
                mesh.vertices[i + 2] as f64,
            ));
            transformed.push(pt.x as f32);
            transformed.push(pt.y as f32);
            transformed.push(pt.z as f32);
        }
        let new_mesh = Mesh::new(&transformed, &mesh.indices);
        Self {
            inner: new_mesh.to_manifold(),
        }
    }

    /// 从 GLB 文件直接转换为 Manifold
    ///
    /// 注意：GLB 文件中的网格应该已经在 CSG 生成阶段通过 weld_vertices_for_manifold
    /// 保证了流形性，这里只需要应用变换矩阵，不再做顶点焊接。
    pub fn import_glb_to_manifold(
        path: &Path,
        mat4: DMat4,
        more_precision: bool,
    ) -> anyhow::Result<Self> {
        let (document, buffers, _) = gltf::import(path)?;

        let mut all_vertices: Vec<f32> = Vec::new();
        let mut all_indices: Vec<u32> = Vec::new();

        let mut vertex_offset = 0u32;
        for mesh in document.meshes() {
            for primitive in mesh.primitives() {
                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

                // 读取顶点并应用变换矩阵
                if let Some(iter) = reader.read_positions() {
                    for v in iter {
                        let pt = mat4.transform_point3(glam::DVec3::new(
                            v[0] as f64,
                            v[1] as f64,
                            v[2] as f64,
                        ));
                        all_vertices.push(pt.x as f32);
                        all_vertices.push(pt.y as f32);
                        all_vertices.push(pt.z as f32);
                    }
                }

                // 读取索引并调整偏移量
                if let Some(iter) = reader.read_indices() {
                    let indices: Vec<u32> = iter.into_u32().collect();
                    let vertex_count = (all_vertices.len() / 3) as u32 - vertex_offset;

                    for &idx in &indices {
                        all_indices.push(vertex_offset + idx);
                    }

                    vertex_offset += vertex_count;
                }
            }
        }

        if all_vertices.is_empty() || all_indices.is_empty() {
            return Ok(Self::new());
        }

        // 关键：GLB 中的网格不保证是“共享顶点拓扑”，需要在这里做顶点焊接，
        // 否则 Manifold::to_manifold 可能输出 0 三角形（典型：BOX 类 24 顶点/12 三角形的 per-face mesh）。
        let build_welded = |precision: f64| -> (Vec<f32>, Vec<u32>) {
            let mut map: HashMap<(i64, i64, i64), u32> = HashMap::new();
            let mut remap: Vec<u32> = Vec::with_capacity(all_vertices.len() / 3);
            let mut welded_vertices: Vec<f32> = Vec::new();

            for i in (0..all_vertices.len()).step_by(3) {
                let x = all_vertices[i] as f64;
                let y = all_vertices[i + 1] as f64;
                let z = all_vertices[i + 2] as f64;
                let key = ManifoldMeshRust::quantize_vertex(x, y, z, precision);
                if let Some(&idx) = map.get(&key) {
                    remap.push(idx);
                    continue;
                }
                let idx = (welded_vertices.len() / 3) as u32;
                map.insert(key, idx);
                remap.push(idx);
                welded_vertices.push(x as f32);
                welded_vertices.push(y as f32);
                welded_vertices.push(z as f32);
            }

            let mut welded_indices: Vec<u32> = Vec::with_capacity(all_indices.len());
            for tri in all_indices.chunks(3) {
                if tri.len() != 3 {
                    continue;
                }
                let a = remap[tri[0] as usize];
                let b = remap[tri[1] as usize];
                let c = remap[tri[2] as usize];
                if a == b || b == c || a == c {
                    continue;
                }
                welded_indices.extend_from_slice(&[a, b, c]);
            }

            (welded_vertices, welded_indices)
        };

        // 估算自适应精度（基于当前已应用 mat4 的坐标）
        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;
        let mut min_z = f32::MAX;
        let mut max_z = f32::MIN;
        for i in (0..all_vertices.len()).step_by(3) {
            let x = all_vertices[i];
            let y = all_vertices[i + 1];
            let z = all_vertices[i + 2];
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
            min_z = min_z.min(z);
            max_z = max_z.max(z);
        }
        let extent_x = (max_x - min_x).abs();
        let extent_y = (max_y - min_y).abs();
        let extent_z = (max_z - min_z).abs();
        let min_extent = extent_x.min(extent_y).min(extent_z);

        let mut precision: f64 = if min_extent < 0.1 {
            100000.0
        } else if min_extent < 1.0 {
            10000.0
        } else if min_extent < 10.0 {
            1000.0
        } else if min_extent < 100.0 {
            100.0
        } else {
            10.0
        };
        if more_precision {
            precision = (precision * 1000.0).min(1_000_000_000.0);
        }

        let input_triangles = all_indices.len() / 3;
        let (mut vertices, mut indices) = build_welded(precision);
        if input_triangles > 0 && indices.is_empty() {
            let retry_precision = (precision * 1000.0).min(1_000_000_000.0);
            (vertices, indices) = build_welded(retry_precision);
        }

        let manifold = Self::from_mesh_with_cap(&ManifoldMeshRust {
            vertices,
            indices,
        });

        Ok(manifold)
    }

    /// 导出到 GLB 文件
    ///
    /// 注意：导出时会将 manifold mesh（共享顶点）转换为普通 mesh（重复顶点），
    /// 以保证渲染时边缘轮廓清晰（每个面有独立的法线）
    pub fn export_to_glb(&self, path: &Path) -> anyhow::Result<()> {
        let rs_mesh = self.inner.to_mesh();
        let prop_num = rs_mesh.num_props() as usize;
        let raw_vertices = rs_mesh.vertices();
        let old_indices = rs_mesh.indices();

        if raw_vertices.is_empty() || old_indices.is_empty() {
            return Err(anyhow::anyhow!("布尔运算结果为空，无法导出"));
        }
        if prop_num < 3 {
            return Err(anyhow::anyhow!(
                "Manifold mesh 顶点属性数异常：prop_num={}",
                prop_num
            ));
        }

        // 将共享顶点拓扑展开成 per-triangle 顶点（保证硬边 + 便于过滤退化三角形）。
        let vert_num = raw_vertices.len() / prop_num;
        let mut positions: Vec<f32> = Vec::with_capacity(old_indices.len() * 3);
        let mut normals: Vec<f32> = Vec::with_capacity(old_indices.len() * 3);
        let mut indices: Vec<u32> = Vec::with_capacity(old_indices.len());

        let mut out_v = 0u32;
        let mut dropped = 0u32;
        for tri in old_indices.chunks_exact(3) {
            let ia = tri[0] as usize;
            let ib = tri[1] as usize;
            let ic = tri[2] as usize;
            if ia >= vert_num || ib >= vert_num || ic >= vert_num {
                dropped += 1;
                continue;
            }

            let a = Vec3::new(
                raw_vertices[prop_num * ia + 0],
                raw_vertices[prop_num * ia + 1],
                raw_vertices[prop_num * ia + 2],
            );
            let b = Vec3::new(
                raw_vertices[prop_num * ib + 0],
                raw_vertices[prop_num * ib + 1],
                raw_vertices[prop_num * ib + 2],
            );
            let c = Vec3::new(
                raw_vertices[prop_num * ic + 0],
                raw_vertices[prop_num * ic + 1],
                raw_vertices[prop_num * ic + 2],
            );

            let n = (b - a).cross(c - a);
            let n2 = n.length_squared();
            // 过滤退化三角形，避免 normalize(0) => NaN 写入 GLB。
            if !n2.is_finite() || n2 <= 1e-20 {
                dropped += 1;
                continue;
            }
            let n = n / n2.sqrt();

            positions.extend_from_slice(&[a.x, a.y, a.z, b.x, b.y, b.z, c.x, c.y, c.z]);
            normals.extend_from_slice(&[
                n.x, n.y, n.z, //
                n.x, n.y, n.z, //
                n.x, n.y, n.z,
            ]);

            indices.extend_from_slice(&[out_v, out_v + 1, out_v + 2]);
            out_v += 3;
        }

        if indices.is_empty() {
            return Err(anyhow::anyhow!(
                "布尔运算结果导出失败：所有三角形被过滤（dropped={}）",
                dropped
            ));
        }

        // 确保父目录存在
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        crate::fast_model::export_model::export_glb::export_raw_buffers_to_glb(
            &positions, &normals, &indices, path,
        )?;
        Ok(())
    }

    /// 导出到 OBJ 文件（用于调试）
    ///
    /// 注意：导出时会将 manifold mesh（共享顶点）转换为普通 mesh（重复顶点），
    /// 以保证渲染时边缘轮廓清晰（每个面有独立的法线）
    pub fn export_to_obj(&self, path_str: &str) -> anyhow::Result<()> {
        use std::io::Write;

        let rs_mesh = self.inner.to_mesh();
        let prop_num = rs_mesh.num_props() as usize;
        let raw_vertices = rs_mesh.vertices();
        let old_indices = rs_mesh.indices();

        if old_indices.is_empty() {
            return Err(anyhow::anyhow!("布尔运算结果为空，无法导出"));
        }

        // 顶点数 = raw_vertices.len() / prop_num
        let vert_num = raw_vertices.len() / prop_num;
        let mut vert: Vec<[f32; 3]> = Vec::with_capacity(vert_num);
        for i in 0..vert_num {
            vert.push([
                raw_vertices[prop_num * i + 0],
                raw_vertices[prop_num * i + 1],
                raw_vertices[prop_num * i + 2],
            ]);
        }

        // 确保父目录存在
        let path = Path::new(path_str);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut file = std::fs::File::create(path)?;

        // 写入顶点（展开为 per-triangle 顶点）
        let mut vertices: Vec<Vec3> = Vec::with_capacity(old_indices.len());
        let mut normals: Vec<Vec3> = Vec::with_capacity(old_indices.len());

        for c in old_indices.chunks(3) {
            if c.len() != 3 {
                continue;
            }
            let a: Vec3 = Vec3::from(vert[c[0] as usize]);
            let b: Vec3 = Vec3::from(vert[c[1] as usize]);
            let c: Vec3 = Vec3::from(vert[c[2] as usize]);

            let normal = ((b - a).cross(c - a)).normalize();

            vertices.push(a);
            vertices.push(b);
            vertices.push(c);

            normals.push(normal);
            normals.push(normal);
            normals.push(normal);
        }

        // 写入顶点
        for v in &vertices {
            writeln!(file, "v {} {} {}", v.x, v.y, v.z)?;
        }

        // 写入法线
        for n in &normals {
            writeln!(file, "vn {} {} {}", n.x, n.y, n.z)?;
        }

        // 写入面（OBJ 索引从 1 开始）
        for i in (0..vertices.len()).step_by(3) {
            let i1 = i + 1;
            let i2 = i + 2;
            let i3 = i + 3;
            writeln!(file, "f {}//{} {}//{} {}//{}", i1, i1, i2, i2, i3, i3)?;
        }

        Ok(())
    }

    pub fn from_mesh(m: &ManifoldMeshRust) -> Self {
        let input_tri_count = m.indices.len() / 3;
        let input_vert_count = m.vertices.len() / 3;

        // 如果输入为空，返回空 manifold
        if m.indices.is_empty() || m.vertices.is_empty() {
            eprintln!("[Manifold] 输入 mesh 为空，跳过转换");
            return Self::new();
        }

        let mesh = Mesh::new(&m.vertices, &m.indices);
        let result = Self {
            inner: mesh.to_manifold(),
        };

        // 检查转换结果
        let result_mesh = result.inner.to_mesh();
        let output_tri_count = result_mesh.indices().len() / 3;

        if output_tri_count == 0 && input_tri_count > 0 {
            // Manifold 转换失败，输出诊断信息
            eprintln!(
                "[Manifold] ⚠️ 转换后三角形丢失: 输入 {} 顶点 {} 三角形 -> 输出 0 三角形",
                input_vert_count, input_tri_count
            );

            // 检查 mesh 是否有问题（如退化三角形、非流形等）
            // 计算 AABB 来诊断几何范围
            let mut min_x = f32::MAX;
            let mut max_x = f32::MIN;
            let mut min_y = f32::MAX;
            let mut max_y = f32::MIN;
            let mut min_z = f32::MAX;
            let mut max_z = f32::MIN;

            for i in (0..m.vertices.len()).step_by(3) {
                let x = m.vertices[i];
                let y = m.vertices[i + 1];
                let z = m.vertices[i + 2];
                min_x = min_x.min(x);
                max_x = max_x.max(x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
                min_z = min_z.min(z);
                max_z = max_z.max(z);
            }

            let extent_x = max_x - min_x;
            let extent_y = max_y - min_y;
            let extent_z = max_z - min_z;

            eprintln!(
                "[Manifold] AABB: ({:.2}, {:.2}, {:.2}) -> ({:.2}, {:.2}, {:.2}), 范围: ({:.2}, {:.2}, {:.2})",
                min_x, min_y, min_z, max_x, max_y, max_z, extent_x, extent_y, extent_z
            );

            // 检查是否有极端的长宽比
            let min_extent = extent_x.min(extent_y).min(extent_z);
            let max_extent = extent_x.max(extent_y).max(extent_z);
            if min_extent > 0.0 && max_extent / min_extent > 100.0 {
                eprintln!(
                    "[Manifold] ⚠️ 极端长宽比: {:.1} (最小维度 {:.4}, 最大维度 {:.2})",
                    max_extent / min_extent,
                    min_extent,
                    max_extent
                );
            }
        }

        result
    }

    pub fn get_mesh(&self) -> ManifoldMeshRust {
        let mesh = self.inner.to_mesh();
        let prop_num = mesh.num_props() as usize;
        let raw_vertices = mesh.vertices();
        let indices = mesh.indices();

        // manifold-rs 的 mesh 顶点可能包含额外属性（num_props > 3），这里统一压缩为 xyz 三分量，
        // 避免下游（AABB/导出）误把属性当作位置坐标。
        let mut vertices: Vec<f32> = Vec::new();
        if prop_num >= 3 && !raw_vertices.is_empty() {
            let vert_num = raw_vertices.len() / prop_num;
            vertices.reserve(vert_num * 3);
            for i in 0..vert_num {
                let base = prop_num * i;
                vertices.push(raw_vertices[base + 0]);
                vertices.push(raw_vertices[base + 1]);
                vertices.push(raw_vertices[base + 2]);
            }
        }

        ManifoldMeshRust { vertices, indices }
    }

    /// 不支持 subtract
    pub fn batch_boolean(batch: &[Self], op: ManifoldOpType) -> Self {
        if batch.is_empty() {
            return Self::new();
        }

        let mut result = batch[0].clone();
        for b in batch.iter().skip(1) {
            result.inner = match op {
                ManifoldOpType::Union => result.inner.union(&b.inner),
                ManifoldOpType::Intersection => result.inner.intersection(&b.inner),
                ManifoldOpType::Difference => result.inner.difference(&b.inner),
            };
        }
        result
    }

    pub fn batch_boolean_subtract(&self, negs: &[Self]) -> Self {
        if negs.is_empty() {
            return self.clone();
        }

        let mut result = self.clone();
        for b in negs.iter() {
            result.inner = result.inner.difference(&b.inner);
        }
        result
    }

    /// 从顶点数组和索引数组创建 ManifoldRust
    ///
    /// # 参数
    /// * `vertices` - 顶点数组，每个元素是 Vec3
    /// * `indices` - 索引数组
    /// * `mat4` - 变换矩阵
    /// * `more_precision` - 是否使用更高精度
    ///
    /// 此方法会自动进行顶点焊接以确保流形拓扑
    pub fn from_vertices_indices(
        vertices: &[glam::Vec3],
        indices: &[u32],
        mat4: glam::DMat4,
        more_precision: bool,
    ) -> Self {
        let mesh = ManifoldMeshRust::from_vertices_indices(vertices, indices, mat4, more_precision);
        Self::from_mesh_with_cap(&mesh)
    }

    /// 从 ManifoldMeshRust 创建 ManifoldRust，若 to_manifold 结果为空则尝试补端盖重试
    pub fn from_mesh_with_cap(m: &ManifoldMeshRust) -> Self {
        let manifold = Self::from_mesh(m);
        if !manifold.get_mesh().indices.is_empty() || m.indices.is_empty() || m.vertices.is_empty() {
            return manifold;
        }

        if let Some(capped) = try_cap_boundary_loops(&m.vertices, &m.indices) {
            let capped_manifold = Self::from_mesh(&ManifoldMeshRust {
                vertices: m.vertices.clone(),
                indices: capped,
            });
            if !capped_manifold.get_mesh().indices.is_empty() {
                return capped_manifold;
            }
        }

        manifold
    }

    /// 从 AABB 中心向外微量膨胀（消除布尔运算中的共面薄片）
    ///
    /// 当负实体的面与正实体的面完全共面时，布尔差集会产生零厚度退化三角形。
    /// 通过将负实体从其 AABB 中心向外扩展 `epsilon_mm`（每边），
    /// 使其略微超出正实体表面，从而产生干净的切割。
    ///
    /// # 参数
    /// * `epsilon_mm` - 每边扩展量（与模型单位一致，PDMS 中为 mm）
    pub fn inflate_from_center(&self, epsilon_mm: f64) -> Self {
        let mesh = self.get_mesh();
        let aabb = match mesh.cal_aabb() {
            Some(a) => a,
            None => return self.clone(),
        };
        let extents = aabb.extents();
        let min_ext = extents.x.min(extents.y).min(extents.z) as f64;
        if min_ext < 1e-6 {
            return self.clone();
        }
        let center = aabb.center();
        let cx = center.x as f64;
        let cy = center.y as f64;
        let cz = center.z as f64;
        // 每个轴的缩放因子 = 1 + 2*epsilon / extent（两端各扩展 epsilon）
        let sx = 1.0 + 2.0 * epsilon_mm / extents.x as f64;
        let sy = 1.0 + 2.0 * epsilon_mm / extents.y as f64;
        let sz = 1.0 + 2.0 * epsilon_mm / extents.z as f64;
        // 平移到原点 → 缩放 → 平移回来
        let step1 = self.inner.translate(-cx, -cy, -cz);
        let step2 = step1.scale(sx, sy, sz);
        let step3 = step2.translate(cx, cy, cz);
        Self { inner: step3 }
    }

    pub fn destroy(&self) {
        // manifold-rs 使用 RAII，无需手动释放
    }
}

/// 尝试为简单边界环自动补端盖（用于把开口壳体变成可用于布尔运算的闭合体）。
///
/// 返回：若成功则返回新的 indices（包含追加的端盖三角形）；否则返回 None。
fn try_cap_boundary_loops(vertices_xyz: &[f32], indices: &[u32]) -> Option<Vec<u32>> {
    use std::collections::{HashMap, HashSet};

    if vertices_xyz.len() % 3 != 0 {
        return None;
    }
    let vert_count = (vertices_xyz.len() / 3) as u32;
    if vert_count < 3 {
        return None;
    }

    // 1) 统计无向边出现次数
    let mut edge_count: HashMap<(u32, u32), u32> = HashMap::new();
    for tri in indices.chunks_exact(3) {
        let a = tri[0];
        let b = tri[1];
        let c = tri[2];
        if a >= vert_count || b >= vert_count || c >= vert_count {
            continue;
        }
        for (u, v) in [(a, b), (b, c), (c, a)] {
            let (x, y) = if u < v { (u, v) } else { (v, u) };
            *edge_count.entry((x, y)).or_insert(0) += 1;
        }
    }

    // 2) 收集边界边（count==1），构造邻接
    let mut adj: HashMap<u32, Vec<u32>> = HashMap::new();
    let mut boundary_edges: Vec<(u32, u32)> = Vec::new();
    for ((u, v), cnt) in edge_count {
        if cnt == 1 {
            boundary_edges.push((u, v));
            adj.entry(u).or_default().push(v);
            adj.entry(v).or_default().push(u);
        }
    }
    if boundary_edges.is_empty() {
        return None;
    }

    // mesh 中心（AABB center）
    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);
    for i in (0..vertices_xyz.len()).step_by(3) {
        let p = Vec3::new(vertices_xyz[i], vertices_xyz[i + 1], vertices_xyz[i + 2]);
        min = min.min(p);
        max = max.max(p);
    }
    let mesh_center = (min + max) * 0.5;

    let pos_of = |idx: u32| -> Vec3 {
        let base = (idx as usize) * 3;
        Vec3::new(
            vertices_xyz[base + 0],
            vertices_xyz[base + 1],
            vertices_xyz[base + 2],
        )
    };

    // 4) 提取所有边界环（按“边”遍历；允许少量非理想情况，但要求最终形成闭环）
    let mut visited_edges: HashSet<(u32, u32)> = HashSet::new();
    let mut loops: Vec<Vec<u32>> = Vec::new();
    let norm_edge = |a: u32, b: u32| if a < b { (a, b) } else { (b, a) };

    for &(eu, ev) in &boundary_edges {
        let ekey = norm_edge(eu, ev);
        if visited_edges.contains(&ekey) {
            continue;
        }

        let mut ring: Vec<u32> = Vec::new();
        let start_u = eu;
        let start_v = ev;
        ring.push(start_u);
        ring.push(start_v);
        visited_edges.insert(ekey);

        let mut prev = start_u;
        let mut curr = start_v;

        loop {
            let neigh = adj.get(&curr)?;
            // 优先选“不是 prev 且该边尚未访问”的邻居
            let mut next: Option<u32> = None;
            for &cand in neigh {
                if cand == prev {
                    continue;
                }
                let k = norm_edge(curr, cand);
                if !visited_edges.contains(&k) {
                    next = Some(cand);
                    break;
                }
            }

            let Some(nxt) = next else {
                // 若已经回到起点则视为闭合，否则失败
                if curr == start_u {
                    break;
                }
                return None;
            };

            // 若闭合
            if nxt == start_u {
                ring.push(nxt);
                visited_edges.insert(norm_edge(curr, nxt));
                break;
            }

            ring.push(nxt);
            visited_edges.insert(norm_edge(curr, nxt));
            prev = curr;
            curr = nxt;

            // 防止异常：环长度不能无限增长
            if ring.len() > boundary_edges.len() + 2 {
                return None;
            }
        }

        // ring 形如 [v0, v1, ..., v0]，去掉末尾重复点
        if ring.len() >= 4 && *ring.last()? == ring[0] {
            ring.pop();
        }
        // 去重检查：简单环不应出现重复顶点
        let mut uniq: HashSet<u32> = HashSet::new();
        if ring.iter().all(|&v| uniq.insert(v)) && ring.len() >= 3 {
            loops.push(ring);
        }
    }

    if loops.is_empty() {
        return None;
    }

    // 5) 给每个环加端盖
    let mut out = indices.to_vec();
    let mut added_tris = 0usize;

    for mut ring in loops {
        // 去掉闭环末尾重复的 start（提取时我们以 curr==start 结束，但未 push start 第二次）
        if ring.len() < 3 {
            continue;
        }

        // Newell 法求环法线
        let mut n = Vec3::ZERO;
        let mut center = Vec3::ZERO;
        for &vid in &ring {
            center += pos_of(vid);
        }
        center /= ring.len() as f32;
        for i in 0..ring.len() {
            let p0 = pos_of(ring[i]);
            let p1 = pos_of(ring[(i + 1) % ring.len()]);
            n.x += (p0.y - p1.y) * (p0.z + p1.z);
            n.y += (p0.z - p1.z) * (p0.x + p1.x);
            n.z += (p0.x - p1.x) * (p0.y + p1.y);
        }
        if n.length_squared() <= 1e-12 {
            // 退化：用两条边叉乘
            let p0 = pos_of(ring[0]);
            let p1 = pos_of(ring[1]);
            let p2 = pos_of(ring[2]);
            n = (p1 - p0).cross(p2 - p0);
        }
        if n.length_squared() <= 1e-12 {
            continue;
        }
        n = n.normalize();

        // 让法线朝外：与 (ring_center - mesh_center) 同向
        if n.dot(center - mesh_center) < 0.0 {
            ring.reverse();
        }

        // 扇形三角化：v0, vi, v(i+1)
        let v0 = ring[0];
        for i in 1..(ring.len() - 1) {
            out.extend_from_slice(&[v0, ring[i], ring[i + 1]]);
            added_tris += 1;
        }
    }

    if added_tris == 0 {
        return None;
    }

    Some(out)
}

/// Mesh 的 Rust 封装
pub struct ManifoldMeshRust {
    pub vertices: Vec<f32>,
    pub indices: Vec<u32>,
}

impl ManifoldMeshRust {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    /// 计算适应性精度因子
    ///
    /// 根据几何体的尺寸范围选择合适的精度：
    /// - 小尺寸几何体（如单位化的扫掠体）使用更高精度
    /// - 大尺寸几何体使用较低精度以避免数值问题
    pub fn compute_adaptive_precision(vertices: &[glam::Vec3], mat4: &glam::DMat4) -> f32 {
        let mut min = glam::DVec3::new(f64::MAX, f64::MAX, f64::MAX);
        let mut max = glam::DVec3::new(f64::MIN, f64::MIN, f64::MIN);

        for v in vertices {
            let tv = mat4.transform_point3(v.as_dvec3());
            min = min.min(tv);
            max = max.max(tv);
        }

        let extent = max - min;
        let min_extent = extent.x.min(extent.y).min(extent.z);

        // Adjust precision based on min_extent
        if min_extent < 0.1 {
            100000.0 // High precision for very small geometries
        } else if min_extent < 1.0 {
            10000.0 // High precision for small geometries
        } else if min_extent < 10.0 {
            1000.0 // Medium precision
        } else if min_extent < 100.0 {
            100.0 // Lower precision
        } else {
            10.0
        }
    }

    /// 计算网格的 AABB
    pub fn cal_aabb(&self) -> Option<parry3d::bounding_volume::Aabb> {
        use parry3d::bounding_volume::BoundingVolume;
        if self.vertices.is_empty() {
            return None;
        }
        let mut aabb = parry3d::bounding_volume::Aabb::new_invalid();
        for chunk in self.vertices.chunks_exact(3) {
            aabb.take_point(parry3d::math::Point::new(chunk[0], chunk[1], chunk[2]));
        }

        let mins = glam::Vec3::from(aabb.mins);
        let maxs = glam::Vec3::from(aabb.maxs);
        if !mins.is_finite() || !maxs.is_finite() {
            return None;
        }
        let ext_mag = aabb.extents().magnitude();
        if !ext_mag.is_finite() || ext_mag <= 0.0 {
            return None;
        }
        Some(aabb)
    }

    /// 修复不一致的三角形绕序（BFS 传播一致性）
    ///
    /// CSG 生成的 mesh 可能存在混合绕序（如底/顶面与侧面相反），
    /// 导致 Manifold 库拒绝。此方法通过 BFS 遍历半边邻接关系，
    /// 确保所有三角形绕序一致，然后用有符号体积判断是否需要整体翻转。
    pub fn orient_consistently(&mut self) {
        use std::collections::{HashMap, VecDeque};

        let n_tris = self.indices.len() / 3;
        if n_tris == 0 {
            return;
        }

        // 半边 (a,b) → 所属三角形索引
        let mut half_edge_tri: HashMap<(u32, u32), usize> = HashMap::with_capacity(n_tris * 3);
        for ti in 0..n_tris {
            let base = ti * 3;
            for j in 0..3 {
                let a = self.indices[base + j];
                let b = self.indices[base + (j + 1) % 3];
                half_edge_tri.insert((a, b), ti);
            }
        }

        let mut visited = vec![false; n_tris];
        let mut flip = vec![false; n_tris];
        let mut queue = VecDeque::new();

        // 从三角形 0 开始 BFS
        visited[0] = true;
        queue.push_back(0);

        while let Some(ti) = queue.pop_front() {
            let base = ti * 3;
            let (a, b, c) = (self.indices[base], self.indices[base + 1], self.indices[base + 2]);
            // 有效边：如果当前三角形需要翻转，则边顺序反转
            let edges: [(u32, u32); 3] = if flip[ti] {
                [(a, c), (c, b), (b, a)]
            } else {
                [(a, b), (b, c), (c, a)]
            };

            for (ea, eb) in edges {
                // 一致的邻居应持有反向半边 (eb, ea)
                if let Some(&ni) = half_edge_tri.get(&(eb, ea)) {
                    if !visited[ni] {
                        visited[ni] = true;
                        queue.push_back(ni);
                    }
                }
                // 不一致的邻居持有同向半边 (ea, eb)
                if let Some(&ni) = half_edge_tri.get(&(ea, eb)) {
                    if ni != ti && !visited[ni] {
                        visited[ni] = true;
                        flip[ni] = true;
                        queue.push_back(ni);
                    }
                }
            }
        }

        // 应用翻转
        for ti in 0..n_tris {
            if flip[ti] {
                let base = ti * 3;
                self.indices.swap(base + 1, base + 2);
            }
        }

        // 用有符号体积判断法线朝向：负体积 → 法线朝内 → 整体翻转
        let signed_vol = self.signed_volume();
        if signed_vol < 0.0 {
            for tri in self.indices.chunks_exact_mut(3) {
                tri.swap(1, 2);
            }
        }
    }

    /// 计算 mesh 的有符号体积（正 = 法线朝外，负 = 法线朝内）
    fn signed_volume(&self) -> f64 {
        let mut vol = 0.0f64;
        for tri in self.indices.chunks_exact(3) {
            let (ai, bi, ci) = (tri[0] as usize * 3, tri[1] as usize * 3, tri[2] as usize * 3);
            if ai + 2 >= self.vertices.len() || bi + 2 >= self.vertices.len() || ci + 2 >= self.vertices.len() {
                continue;
            }
            let (ax, ay, az) = (self.vertices[ai] as f64, self.vertices[ai + 1] as f64, self.vertices[ai + 2] as f64);
            let (bx, by, bz) = (self.vertices[bi] as f64, self.vertices[bi + 1] as f64, self.vertices[bi + 2] as f64);
            let (cx, cy, cz) = (self.vertices[ci] as f64, self.vertices[ci + 1] as f64, self.vertices[ci + 2] as f64);
            // 有符号体积 = det([a, b, c]) / 6
            vol += ax * (by * cz - bz * cy)
                 + ay * (bz * cx - bx * cz)
                 + az * (bx * cy - by * cx);
        }
        vol / 6.0
    }

    /// 将顶点坐标量化为整数键（用于顶点焊接）
    fn quantize_vertex(x: f64, y: f64, z: f64, precision: f64) -> (i64, i64, i64) {
        (
            (x * precision).round() as i64,
            (y * precision).round() as i64,
            (z * precision).round() as i64,
        )
    }

    /// 从顶点数组和索引数组创建 ManifoldMeshRust
    ///
    /// # 参数
    /// * `vertices` - 顶点数组，每个元素是 [x, y, z] 或 Vec3
    /// * `indices` - 索引数组
    /// * `mat4` - 变换矩阵
    /// * `more_precision` - 是否使用更高精度
    ///
    /// 此方法会自动进行顶点焊接以确保流形拓扑
    pub fn from_vertices_indices(
        vertices: &[glam::Vec3],
        indices: &[u32],
        mat4: glam::DMat4,
        more_precision: bool,
    ) -> Self {
        if vertices.is_empty() || indices.is_empty() {
            return Self::new();
        }

        // 计算自适应精度
        let mut base_precision = Self::compute_adaptive_precision(vertices, &mat4) as f64;
        if more_precision {
            base_precision = (base_precision * 1000.0).min(1_000_000_000.0);
        }

        let build = |precision: f64| -> (Vec<f32>, Vec<u32>) {
            let mut map: HashMap<(i64, i64, i64), u32> = HashMap::new();
            let mut remap: Vec<u32> = Vec::with_capacity(vertices.len());
            let mut welded_vertices: Vec<f32> = Vec::new();

            for v in vertices {
                let pt =
                    mat4.transform_point3(glam::DVec3::new(v.x as f64, v.y as f64, v.z as f64));
                let key = Self::quantize_vertex(pt.x, pt.y, pt.z, precision);
                if let Some(&idx) = map.get(&key) {
                    remap.push(idx);
                    continue;
                }
                let idx = (welded_vertices.len() / 3) as u32;
                map.insert(key, idx);
                remap.push(idx);
                welded_vertices.push(pt.x as f32);
                welded_vertices.push(pt.y as f32);
                welded_vertices.push(pt.z as f32);
            }

            let mut welded_indices: Vec<u32> = Vec::with_capacity(indices.len());
            for tri in indices.chunks(3) {
                if tri.len() != 3 {
                    continue;
                }
                let a = remap[tri[0] as usize];
                let b = remap[tri[1] as usize];
                let c = remap[tri[2] as usize];
                if a == b || b == c || a == c {
                    continue;
                }
                welded_indices.extend_from_slice(&[a, b, c]);
            }

            (welded_vertices, welded_indices)
        };

        let input_triangles = indices.len() / 3;
        let (mut transformed_vertices, mut welded_indices) = build(base_precision);
        if input_triangles > 0 && welded_indices.is_empty() {
            // 退化保护：量化过粗时会把"薄壁/小三角形"合并塌陷成退化三角形，导致全被过滤。
            // 这里提高精度重试。
            let retry_precision = (base_precision * 1000.0).min(1_000_000_000.0);
            (transformed_vertices, welded_indices) = build(retry_precision);
        }

        Self {
            vertices: transformed_vertices,
            indices: welded_indices,
        }
    }


    /// 保存为二进制文件 (.manifold)
    ///
    /// 格式: [vertex_count: u32][index_count: u32][vertices: f32 × N][indices: u32 × M]
    pub fn save_to_file(&self, path: &std::path::Path) -> anyhow::Result<()> {
        use std::io::Write;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let vertex_count = (self.vertices.len()) as u32;
        let index_count = self.indices.len() as u32;

        let mut file = std::fs::File::create(path)?;
        file.write_all(&vertex_count.to_le_bytes())?;
        file.write_all(&index_count.to_le_bytes())?;

        // SAFETY: f32 和 u32 都是 4 字节 POD 类型
        let vert_bytes = unsafe {
            std::slice::from_raw_parts(
                self.vertices.as_ptr() as *const u8,
                self.vertices.len() * 4,
            )
        };
        file.write_all(vert_bytes)?;

        let idx_bytes = unsafe {
            std::slice::from_raw_parts(
                self.indices.as_ptr() as *const u8,
                self.indices.len() * 4,
            )
        };
        file.write_all(idx_bytes)?;
        Ok(())
    }

    /// 从二进制文件加载 (.manifold)
    pub fn load_from_file(path: &std::path::Path) -> anyhow::Result<Self> {
        use std::io::Read;

        let mut file = std::fs::File::open(path)
            .map_err(|e| anyhow::anyhow!("打开 .manifold 文件失败: {} - {}", path.display(), e))?;

        let mut buf4 = [0u8; 4];
        file.read_exact(&mut buf4)?;
        let vertex_count = u32::from_le_bytes(buf4) as usize;
        file.read_exact(&mut buf4)?;
        let index_count = u32::from_le_bytes(buf4) as usize;

        let mut vertices = vec![0f32; vertex_count];
        let vert_bytes = unsafe {
            std::slice::from_raw_parts_mut(vertices.as_mut_ptr() as *mut u8, vertex_count * 4)
        };
        file.read_exact(vert_bytes)?;

        let mut indices = vec![0u32; index_count];
        let idx_bytes = unsafe {
            std::slice::from_raw_parts_mut(indices.as_mut_ptr() as *mut u8, index_count * 4)
        };
        file.read_exact(idx_bytes)?;

        Ok(Self { vertices, indices })
    }
}
//负实体的模型应该更大一些
//正实体的模型更小一些
