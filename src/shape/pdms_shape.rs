use anyhow::anyhow;
#[cfg(feature = "render")]
use bevy_asset::RenderAssetUsages;
use bevy_ecs::component::Component;
#[cfg(feature = "render")]
use bevy_mesh::{Indices, Mesh};
#[cfg(feature = "render")]
use bevy_render::render_resource::PrimitiveTopology;
use bevy_transform::prelude::Transform;
use derive_more::{Deref, DerefMut};
use downcast_rs::*;
use dyn_clone::DynClone;
use glam::{DMat4, DVec3};
use glam::{Mat4, Vec3, Vec4};
use itertools::Itertools;
use parry3d::bounding_volume::Aabb;
use parry3d::math::{Point, Vector};
use parry3d::shape::{TriMesh, TriMeshFlags};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::fs::File;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::BufWriter;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::vec;
use surrealdb::types::SurrealValue;
#[cfg(feature = "truck")]
use truck_base::bounding_box::BoundingBox;
#[cfg(feature = "truck")]
use truck_base::cgmath64::{Matrix4, Point3, Vector3, Vector4};
#[cfg(feature = "truck")]
use truck_meshalgo::prelude::{MeshableShape, MeshedShape};
#[cfg(feature = "truck")]
use truck_modeling::{Curve, Shell};

use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::tool::float_tool::f32_round_3;
use parry3d::bounding_volume::BoundingVolume;

use crate::geometry::PlantGeoData;

pub const TRIANGLE_TOL: f64 = 0.01;

pub trait VerifiedShape {
    fn check_valid(&self) -> bool {
        true
    }
}

#[cfg(feature = "truck")]
#[inline]
pub fn gen_bounding_box(shell: &Shell) -> BoundingBox<Point3> {
    let mut bdd_box = BoundingBox::new();
    shell
        .iter()
        .flat_map(truck_modeling::Face::boundaries)
        .flatten()
        .for_each(|edge| {
            let curve = edge.oriented_curve();
            bdd_box += match curve {
                Curve::Line(line) => vec![line.0, line.1].into_iter().collect(),
                Curve::BSplineCurve(curve) => {
                    let bdb = curve.roughly_bounding_box();
                    vec![bdb.max(), bdb.min()].into_iter().collect()
                }
                Curve::NurbsCurve(curve) => curve.roughly_bounding_box(),
                Curve::IntersectionCurve(_) => BoundingBox::new(),
            };
        });
    bdd_box
}

/// 表示一条边，由顶点序列组成
///
/// 边可以包含多个顶点，用于表示直线段或曲线段
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Edge {
    /// 边的顶点序列
    pub vertices: Vec<Vec3>,
}

impl Edge {
    /// 创建新边
    pub fn new(vertices: Vec<Vec3>) -> Self {
        Self { vertices }
    }

    /// 从 Vec<Vec3> 创建边
    pub fn from_vec(vertices: Vec<Vec3>) -> Self {
        Self { vertices }
    }

    /// 获取起点
    pub fn start(&self) -> Option<Vec3> {
        self.vertices.first().copied()
    }

    /// 获取终点
    pub fn end(&self) -> Option<Vec3> {
        self.vertices.last().copied()
    }

    /// 计算边的总长度（遍历所有顶点）
    pub fn length(&self) -> f32 {
        if self.vertices.len() < 2 {
            return 0.0;
        }
        let mut total_length = 0.0;
        for i in 0..(self.vertices.len() - 1) {
            total_length += self.vertices[i].distance(self.vertices[i + 1]);
        }
        total_length
    }

    /// 获取线段数量（顶点数-1）
    pub fn segment_count(&self) -> usize {
        if self.vertices.is_empty() {
            0
        } else {
            self.vertices.len() - 1
        }
    }

    /// 转换为 Vec<Vec3>（兼容现有代码）
    pub fn to_vec(&self) -> Vec<Vec3> {
        self.vertices.clone()
    }
}

impl From<Vec<Vec3>> for Edge {
    fn from(vertices: Vec<Vec3>) -> Self {
        Self { vertices }
    }
}

impl Into<Vec<Vec3>> for Edge {
    fn into(self) -> Vec<Vec3> {
        self.vertices
    }
}

/// 边的集合类型
pub type Edges = Vec<Edge>;

/// 从三角网格索引中提取唯一的边（内部辅助函数）
fn extract_edges_from_mesh_internal(indices: &[u32], vertices: &[Vec3]) -> Edges {
    use std::collections::HashSet;

    if indices.len() < 3 || vertices.is_empty() {
        return Vec::new();
    }

    // 使用 HashSet 存储标准化的边（较小的索引在前）
    let mut edge_set: HashSet<(u32, u32)> = HashSet::new();

    // 遍历所有三角形，提取每条边
    for triangle in indices.chunks_exact(3) {
        let v0 = triangle[0];
        let v1 = triangle[1];
        let v2 = triangle[2];

        // 三条边，标准化为较小的索引在前
        let edges = [
            if v0 < v1 { (v0, v1) } else { (v1, v0) },
            if v1 < v2 { (v1, v2) } else { (v2, v1) },
            if v2 < v0 { (v2, v0) } else { (v0, v2) },
        ];

        for edge in edges {
            edge_set.insert(edge);
        }
    }

    // 将边索引转换为顶点坐标
    let mut edges = Vec::with_capacity(edge_set.len());
    for (idx0, idx1) in edge_set {
        if idx0 < vertices.len() as u32 && idx1 < vertices.len() as u32 {
            let edge = Edge::new(vec![vertices[idx0 as usize], vertices[idx1 as usize]]);
            edges.push(edge);
        }
    }

    edges
}

//todo 增加LOD的实现
#[derive(Serialize, Deserialize, Component, Debug, Clone)]
pub struct PlantMesh {
    pub indices: Vec<u32>,
    pub vertices: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    /// 顶点 UV 坐标（长度与 vertices 一致），用于纹理映射
    #[serde(default)]
    pub uvs: Vec<[f32; 2]>,
    #[serde(skip)]
    pub wire_vertices: Vec<Vec<Vec3>>,
    // edges 现在会被序列化，以支持在 plant3d 中渲染边
    pub edges: Edges,
    #[serde(skip)]
    pub aabb: Option<Aabb>,
}

impl Default for PlantMesh {
    fn default() -> Self {
        Self {
            indices: Vec::new(),
            vertices: Vec::new(),
            normals: Vec::new(),
            uvs: Vec::new(),
            wire_vertices: Vec::new(),
            edges: Vec::new(),
            aabb: None,
        }
    }
}

impl PlantMesh {
    /// 获取边的集合
    pub fn edges(&self) -> &Edges {
        &self.edges
    }

    /// 设置边的集合
    pub fn set_edges(&mut self, edges: Edges) {
        self.edges = edges;
        // 同步更新 wire_vertices 以保持向后兼容
        self.wire_vertices = self.edges.iter().map(|e| e.to_vec()).collect();
    }

    /// 从 wire_vertices 更新 edges
    pub fn sync_edges_from_wire_vertices(&mut self) {
        self.edges = self
            .wire_vertices
            .iter()
            .map(|v| Edge::from_vec(v.clone()))
            .collect();
    }

    /// 从 edges 更新 wire_vertices
    pub fn sync_wire_vertices_from_edges(&mut self) {
        self.wire_vertices = self.edges.iter().map(|e| e.to_vec()).collect();
    }

    ///合并两个mesh
    pub fn merge(&mut self, other: &Self) {
        let vertex_offset = self.vertices.len() as u32;
        self.indices
            .extend(other.indices.iter().map(|&i| i + vertex_offset));
        self.vertices.extend(other.vertices.iter());
        self.normals.extend(other.normals.iter());
        // 合并 edges
        self.edges.extend(other.edges.iter().cloned());
        // 同步更新 wire_vertices 以保持向后兼容
        self.sync_wire_vertices_from_edges();

        // Merge aabb if present
        if let Some(other_aabb) = &other.aabb {
            if let Some(self_aabb) = &mut self.aabb {
                *self_aabb = self_aabb.merged(other_aabb);
            } else {
                self.aabb = Some(*other_aabb);
            }
        }
    }
}

impl PlantMesh {
    ///生成occ mesh (仅在启用 occ feature 时可用)
    #[cfg(feature = "occ")]
    pub fn gen_occ_mesh(shape: &opencascade::primitives::Shape, tol: f64) -> anyhow::Result<Self> {
        let mut aabb = Aabb::new_invalid();
        let mesh = shape.mesh_with_tolerance(tol)?;
        let vertices = mesh
            .vertices
            .iter()
            .map(|&x| x.as_vec3())
            .collect::<Vec<_>>();
        for point in vertices.iter() {
            aabb.take_point(nalgebra::Point3::new(
                point.x as f32,
                point.y as f32,
                point.z as f32,
            ));
        }
        ///生成mesh
        let indices: Vec<u32> = mesh.indices.iter().map(|&x| x as u32).collect();
        let edges = extract_edges_from_mesh_internal(&indices, &vertices);
        let mut mesh = PlantMesh {
            indices,
            vertices,
            normals: mesh.normals.iter().map(|&x| x.as_vec3()).collect(),
            uvs: Vec::new(),
            wire_vertices: vec![],
            edges,
            aabb: Some(aabb),
        };
        mesh.sync_wire_vertices_from_edges();
        Ok(mesh)
    }

    ///生成tri mesh
    #[inline]
    pub fn get_tri_mesh(&self, trans: Mat4) -> Option<TriMesh> {
        self.get_tri_mesh_with_flag(trans, TriMeshFlags::default())
    }

    ///生成带flag的tri mesh
    #[inline]
    pub fn get_tri_mesh_with_flag(&self, trans: Mat4, flag: TriMeshFlags) -> Option<TriMesh> {
        if self.indices.len() < 3 {
            return None;
        }
        let mut points: Vec<Point<f32>> = vec![];
        let mut indices: Vec<[u32; 3]> = vec![];
        //如果 数量太大，需要使用LOD的模型去做碰撞检测
        self.vertices.iter().for_each(|p| {
            let new_pt = trans.transform_point3(*p);
            points.push(new_pt.into())
        });
        // dbg!(&self.indices);
        self.indices.chunks(3).for_each(|i| {
            indices.push([i[0] as u32, i[1] as u32, i[2] as u32]);
        });
        let tri_mesh = TriMesh::with_flags(points, indices, flag).ok()?;
        Some(tri_mesh)
    }

    /// 根据当前顶点自动生成一套简单的 UV（按包围盒进行投影映射）
    ///
    /// - 选择尺寸最大的两个轴作为 U/V 投影轴；
    /// - 将对应坐标归一化到 [0, 1]，保证所有顶点都有可用的 UV；
    pub fn generate_auto_uvs(&mut self) {
        if self.vertices.is_empty() {
            self.uvs.clear();
            return;
        }

        // 计算包围盒
        let mut min_v = Vec3::splat(f32::INFINITY);
        let mut max_v = Vec3::splat(f32::NEG_INFINITY);
        for v in &self.vertices {
            min_v = min_v.min(*v);
            max_v = max_v.max(*v);
        }

        let ext = max_v - min_v;

        // 选择两个跨度最大的轴作为投影轴
        let (axis_u, axis_v) = {
            let ex = ext.x.abs();
            let ey = ext.y.abs();
            let ez = ext.z.abs();

            // 找到最大轴
            if ex >= ey && ex >= ez {
                // X 为最大轴，第二轴取较大的 Y/Z
                if ey >= ez {
                    (0usize, 1usize)
                } else {
                    (0usize, 2usize)
                }
            } else if ey >= ex && ey >= ez {
                // Y 为最大轴
                if ex >= ez {
                    (0usize, 1usize)
                } else {
                    (1usize, 2usize)
                }
            } else {
                // Z 为最大轴
                if ex >= ey {
                    (0usize, 2usize)
                } else {
                    (1usize, 2usize)
                }
            }
        };

        let min_arr = [min_v.x, min_v.y, min_v.z];
        let ext_arr = [ext.x, ext.y, ext.z];

        let min_u = min_arr[axis_u];
        let min_vv = min_arr[axis_v];
        let scale_u = if ext_arr[axis_u].abs() > f32::EPSILON {
            ext_arr[axis_u]
        } else {
            1.0
        };
        let scale_v = if ext_arr[axis_v].abs() > f32::EPSILON {
            ext_arr[axis_v]
        } else {
            1.0
        };

        self.uvs.clear();
        self.uvs.reserve(self.vertices.len());
        for v in &self.vertices {
            let coords = [v.x, v.y, v.z];
            let u = (coords[axis_u] - min_u) / scale_u;
            let vv = (coords[axis_v] - min_vv) / scale_v;
            self.uvs.push([u, vv]);
        }
    }

    ///计算aabb
    pub fn cal_aabb(&self) -> Option<Aabb> {
        let mut aabb = Aabb::new_invalid();
        self.vertices.iter().for_each(|v| {
            aabb.take_point(nalgebra::Point3::new(v.x, v.y, v.z));
        });
        if Vec3::from(aabb.mins).is_nan() || Vec3::from(aabb.maxs).is_nan() {
            return None;
        }
        Some(aabb)
    }

    ///计算法线
    pub fn cal_normals(&mut self) {
        for (_i, c) in self.indices.chunks(3).enumerate() {
            let a: Vec3 = self.vertices[c[0] as usize];
            let b: Vec3 = self.vertices[c[1] as usize];
            let c: Vec3 = self.vertices[c[2] as usize];

            let normal = ((b - a).cross(c - a)).normalize();
            self.normals.push(normal);
            self.normals.push(normal);
            self.normals.push(normal);
        }
    }

    ///todo 后面需要把uv使用上
    #[cfg(feature = "render")]
    pub fn gen_bevy_mesh(&self) -> Mesh {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.vertices.clone());
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals.clone());

        // 如果已有 UV，直接使用；否则生成占位 UV，避免渲染错误
        let uvs: Vec<[f32; 2]> = if !self.uvs.is_empty() && self.uvs.len() == self.vertices.len() {
            self.uvs.clone()
        } else {
            vec![[0.0f32, 0.0f32]; self.vertices.len()]
        };
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

        mesh.insert_indices(Indices::U32(self.indices.clone()));
        mesh
    }

    ///变换mesh
    pub fn transform_by(&self, t: &DMat4) -> Self {
        let mut vertices = Vec::with_capacity(self.vertices.len());
        let mut normals = Vec::with_capacity(self.vertices.len());
        let len = self.vertices.len();
        for i in 0..len {
            vertices.push(t.transform_point3(self.vertices[i].as_dvec3()).as_vec3());
            if i < self.normals.len() {
                normals.push(
                    t.transform_vector3(self.normals[i].as_dvec3())
                        .normalize()
                        .as_vec3(),
                );
            }
        }
        // 变换边
        let transformed_edges: Edges = self
            .edges
            .iter()
            .map(|edge| {
                Edge::new(
                    edge.vertices
                        .iter()
                        .map(|v| t.transform_point3(v.as_dvec3()).as_vec3())
                        .collect(),
                )
            })
            .collect();
        let mut mesh = Self {
            indices: self.indices.clone(),
            vertices,
            normals,
            uvs: self.uvs.clone(),
            wire_vertices: vec![],
            edges: transformed_edges,
            aabb: None,
        };
        mesh.sync_wire_vertices_from_edges();
        mesh
    }

    ///缩放mesh
    pub fn scale_by(&mut self, scale: f32) {
        self.vertices.iter_mut().for_each(|v| {
            *v *= scale;
        });
    }

    ///序列化
    #[inline]
    pub fn ser_to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }

    ///序列化到文件
    #[inline]
    pub fn ser_to_file(&self, file_path: &dyn AsRef<Path>) -> anyhow::Result<()> {
        let bytes = bincode::serialize(self)?;
        let mut file = File::create(file_path).unwrap();
        file.write_all(&bytes)?;
        Ok(())
    }

    ///从文件反序列化
    pub fn des_mesh_file(file_path: &dyn AsRef<Path>) -> anyhow::Result<Self> {
        let mut file = File::open(file_path)?;
        let mut buf: Vec<u8> = Vec::new();
        file.read_to_end(&mut buf).ok();
        let r: Self = bincode::deserialize(&buf)?;
        Ok(r)
    }

    ///从bytes反序列化
    pub fn des_from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        let r: Self = bincode::deserialize(bytes)?;
        Ok(r)
    }

    ///压缩bytes
    #[inline]
    pub fn into_compress_bytes(&self) -> Vec<u8> {
        use flate2::Compression;
        use flate2::write::DeflateEncoder;
        let mut e = DeflateEncoder::new(Vec::new(), Compression::default());
        e.write_all(&bincode::serialize(&self).unwrap());
        e.finish().unwrap_or_default()
    }

    ///从压缩bytes反序列化
    #[inline]
    pub fn from_compress_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        use flate2::write::DeflateDecoder;
        let writer = Vec::new();
        let mut deflater = DeflateDecoder::new(writer);
        deflater.write_all(bytes)?;
        Ok(bincode::deserialize(&deflater.finish()?)?)
    }

    ///导出obj
    pub fn export_obj(&self, reverse: bool, file_path: &str) -> std::io::Result<()> {
        let mut buffer = BufWriter::new(File::create(file_path)?);
        buffer.write_all(b"# List of geometric vertices, with (x, y, z [,w]) coordinates, w is optional and defaults to 1.0.\n")?;
        for (vd, n) in self.vertices.iter().zip(self.normals.iter()) {
            buffer.write_all(
                format!(
                    "v {:.3} {:.3} {:.3}\n",
                    vd[0] as f32, vd[1] as f32, vd[2] as f32
                )
                .as_ref(),
            )?;
            buffer.write_all(
                format!(
                    "vn {:.3} {:.3} {:.3}\n",
                    n[0] as f32, n[1] as f32, n[2] as f32
                )
                .as_ref(),
            )?;
        }
        buffer.write_all(b"# Polygonal face element\n")?;
        for id in self.indices.chunks(3) {
            if reverse {
                buffer.write_all(
                    format!("f {} {} {}\n", id[2] + 1, id[1] + 1, id[0] + 1,).as_ref(),
                )?;
            } else {
                buffer.write_all(
                    format!("f {} {} {}\n", id[0] + 1, id[1] + 1, id[2] + 1,).as_ref(),
                )?;
            }
        }

        buffer.flush()?;
        Ok(())
    }
}

/// 三角形容差
pub const TRI_TOL: f32 = 0.001;
/// 长度容差
pub const LEN_TOL: f32 = 0.001;
/// 角度容差(弧度)
pub const ANGLE_RAD_TOL: f32 = 0.001;
/// 角度容差(弧度,f64)
pub const ANGLE_RAD_F64_TOL: f64 = 0.001;
/// 最小尺寸容差
pub const MIN_SIZE_TOL: f32 = 0.01;
/// 最大尺寸容差
pub const MAX_SIZE_TOL: f32 = 1.0e5;
/// 为BrepShapeTrait实现Clone特征
dyn_clone::clone_trait_object!(BrepShapeTrait);

#[derive(
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
    Serialize,
    Deserialize,
    Deref,
    DerefMut,
    Clone,
    Default,
    Debug,
)]
pub struct RsVec3(pub Vec3);

impl RsVec3 {
    pub fn gen_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::default();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

impl Hash for RsVec3 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        format!("{:.3}", self.x).hash(state);
        format!("{:.3}", self.y).hash(state);
        format!("{:.3}", self.z).hash(state);
    }
}

impl PartialEq<Self> for RsVec3 {
    fn eq(&self, other: &Self) -> bool {
        self.distance(other.0) < 1.0E-5
    }
}

impl Eq for RsVec3 {}

impl From<Vec3> for RsVec3 {
    fn from(value: Vec3) -> Self {
        Self(value)
    }
}

impl From<RsVec3> for Vec3 {
    fn from(value: RsVec3) -> Self {
        value.0
    }
}

impl AsRef<Vec3> for RsVec3 {
    fn as_ref(&self) -> &Vec3 {
        &self.0
    }
}

impl AsMut<Vec3> for RsVec3 {
    fn as_mut(&mut self) -> &mut Vec3 {
        &mut self.0
    }
}

// 运算符重载: RsVec3 * f32
impl std::ops::Mul<f32> for RsVec3 {
    type Output = RsVec3;
    fn mul(self, rhs: f32) -> Self::Output {
        RsVec3(self.0 * rhs)
    }
}

// 运算符重载: &RsVec3 * f32
impl std::ops::Mul<f32> for &RsVec3 {
    type Output = RsVec3;
    fn mul(self, rhs: f32) -> Self::Output {
        RsVec3(self.0 * rhs)
    }
}

// 运算符重载: f32 * RsVec3
impl std::ops::Mul<RsVec3> for f32 {
    type Output = RsVec3;
    fn mul(self, rhs: RsVec3) -> Self::Output {
        RsVec3(self * rhs.0)
    }
}

// 运算符重载: f32 * &RsVec3
impl std::ops::Mul<&RsVec3> for f32 {
    type Output = RsVec3;
    fn mul(self, rhs: &RsVec3) -> Self::Output {
        RsVec3(self * rhs.0)
    }
}

// 运算符重载: RsVec3 + RsVec3
impl std::ops::Add for RsVec3 {
    type Output = RsVec3;
    fn add(self, rhs: Self) -> Self::Output {
        RsVec3(self.0 + rhs.0)
    }
}

// 运算符重载: &RsVec3 + &RsVec3
impl std::ops::Add for &RsVec3 {
    type Output = RsVec3;
    fn add(self, rhs: Self) -> Self::Output {
        RsVec3(self.0 + rhs.0)
    }
}

// 运算符重载: RsVec3 - RsVec3
impl std::ops::Sub for RsVec3 {
    type Output = RsVec3;
    fn sub(self, rhs: Self) -> Self::Output {
        RsVec3(self.0 - rhs.0)
    }
}

// 运算符重载: &RsVec3 - &RsVec3
impl std::ops::Sub for &RsVec3 {
    type Output = RsVec3;
    fn sub(self, rhs: Self) -> Self::Output {
        RsVec3(self.0 - rhs.0)
    }
}

// 运算符重载: -RsVec3 (取负)
impl std::ops::Neg for RsVec3 {
    type Output = RsVec3;
    fn neg(self) -> Self::Output {
        RsVec3(-self.0)
    }
}

// 运算符重载: -&RsVec3
impl std::ops::Neg for &RsVec3 {
    type Output = RsVec3;
    fn neg(self) -> Self::Output {
        RsVec3(-self.0)
    }
}

// ============ RsVec3 与 Vec3 混合运算 ============

// RsVec3 + Vec3
impl std::ops::Add<Vec3> for RsVec3 {
    type Output = RsVec3;
    fn add(self, rhs: Vec3) -> Self::Output {
        RsVec3(self.0 + rhs)
    }
}

// &RsVec3 + Vec3
impl std::ops::Add<Vec3> for &RsVec3 {
    type Output = RsVec3;
    fn add(self, rhs: Vec3) -> Self::Output {
        RsVec3(self.0 + rhs)
    }
}

// RsVec3 + &Vec3
impl std::ops::Add<&Vec3> for RsVec3 {
    type Output = RsVec3;
    fn add(self, rhs: &Vec3) -> Self::Output {
        RsVec3(self.0 + *rhs)
    }
}

// &RsVec3 + &Vec3
impl std::ops::Add<&Vec3> for &RsVec3 {
    type Output = RsVec3;
    fn add(self, rhs: &Vec3) -> Self::Output {
        RsVec3(self.0 + *rhs)
    }
}

// Vec3 + RsVec3
impl std::ops::Add<RsVec3> for Vec3 {
    type Output = RsVec3;
    fn add(self, rhs: RsVec3) -> Self::Output {
        RsVec3(self + rhs.0)
    }
}

// &Vec3 + RsVec3
impl std::ops::Add<RsVec3> for &Vec3 {
    type Output = RsVec3;
    fn add(self, rhs: RsVec3) -> Self::Output {
        RsVec3(*self + rhs.0)
    }
}

// Vec3 + &RsVec3
impl std::ops::Add<&RsVec3> for Vec3 {
    type Output = RsVec3;
    fn add(self, rhs: &RsVec3) -> Self::Output {
        RsVec3(self + rhs.0)
    }
}

// &Vec3 + &RsVec3
impl std::ops::Add<&RsVec3> for &Vec3 {
    type Output = RsVec3;
    fn add(self, rhs: &RsVec3) -> Self::Output {
        RsVec3(*self + rhs.0)
    }
}

// RsVec3 - Vec3
impl std::ops::Sub<Vec3> for RsVec3 {
    type Output = RsVec3;
    fn sub(self, rhs: Vec3) -> Self::Output {
        RsVec3(self.0 - rhs)
    }
}

// &RsVec3 - Vec3
impl std::ops::Sub<Vec3> for &RsVec3 {
    type Output = RsVec3;
    fn sub(self, rhs: Vec3) -> Self::Output {
        RsVec3(self.0 - rhs)
    }
}

// RsVec3 - &Vec3
impl std::ops::Sub<&Vec3> for RsVec3 {
    type Output = RsVec3;
    fn sub(self, rhs: &Vec3) -> Self::Output {
        RsVec3(self.0 - *rhs)
    }
}

// &RsVec3 - &Vec3
impl std::ops::Sub<&Vec3> for &RsVec3 {
    type Output = RsVec3;
    fn sub(self, rhs: &Vec3) -> Self::Output {
        RsVec3(self.0 - *rhs)
    }
}

// Vec3 - RsVec3
impl std::ops::Sub<RsVec3> for Vec3 {
    type Output = RsVec3;
    fn sub(self, rhs: RsVec3) -> Self::Output {
        RsVec3(self - rhs.0)
    }
}

// &Vec3 - RsVec3
impl std::ops::Sub<RsVec3> for &Vec3 {
    type Output = RsVec3;
    fn sub(self, rhs: RsVec3) -> Self::Output {
        RsVec3(*self - rhs.0)
    }
}

// Vec3 - &RsVec3
impl std::ops::Sub<&RsVec3> for Vec3 {
    type Output = RsVec3;
    fn sub(self, rhs: &RsVec3) -> Self::Output {
        RsVec3(self - rhs.0)
    }
}

// &Vec3 - &RsVec3
impl std::ops::Sub<&RsVec3> for &Vec3 {
    type Output = RsVec3;
    fn sub(self, rhs: &RsVec3) -> Self::Output {
        RsVec3(*self - rhs.0)
    }
}

impl SurrealValue for RsVec3 {
    fn kind_of() -> surrealdb::types::Kind {
        surrealdb::types::Kind::Array(Box::new(surrealdb::types::Kind::Number), None)
    }

    fn into_value(self) -> surrealdb::types::Value {
        surrealdb::types::Value::Array(surrealdb::types::Array::from(vec![
            surrealdb::types::Value::Number(surrealdb::types::Number::Float(self.0.x as f64)),
            surrealdb::types::Value::Number(surrealdb::types::Number::Float(self.0.y as f64)),
            surrealdb::types::Value::Number(surrealdb::types::Number::Float(self.0.z as f64)),
        ]))
    }

    fn from_value(value: surrealdb::types::Value) -> anyhow::Result<Self> {
        match value {
            surrealdb::types::Value::Array(arr) => {
                if arr.len() != 3 {
                    return Err(anyhow::anyhow!("数组长度必须为 3 才能转换为 RsVec3"));
                }
                let x = match &arr[0] {
                    surrealdb::types::Value::Number(n) => n.to_f64().unwrap_or(0.0) as f32,
                    _ => return Err(anyhow::anyhow!("数组第一个元素必须是数字")),
                };
                let y = match &arr[1] {
                    surrealdb::types::Value::Number(n) => n.to_f64().unwrap_or(0.0) as f32,
                    _ => return Err(anyhow::anyhow!("数组第二个元素必须是数字")),
                };
                let z = match &arr[2] {
                    surrealdb::types::Value::Number(n) => n.to_f64().unwrap_or(0.0) as f32,
                    _ => return Err(anyhow::anyhow!("数组第三个元素必须是数字")),
                };
                Ok(RsVec3(Vec3::new(x, y, z)))
            }
            _ => Err(anyhow::anyhow!("值必须是数组类型才能转换为 RsVec3")),
        }
    }
}

/// 将truck库的Point3类型转换为RsVec3类型
///
/// 当启用truck特性时，实现从Point3到RsVec3的转换
/// 将Point3的x,y,z坐标转换为f32类型并构造Vec3
#[cfg(feature = "truck")]
impl From<Point3> for RsVec3 {
    fn from(value: Point3) -> Self {
        Self(Vec3::new(value.x as f32, value.y as f32, value.z as f32))
    }
}

///brep形状trait
pub trait BrepShapeTrait: Downcast + VerifiedShape + Debug + Send + Sync + DynClone {
    fn is_reuse_unit(&self) -> bool {
        false
    }

    //拷贝函数
    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait>;

    #[cfg(feature = "truck")]
    ///生成shell
    fn gen_brep_shell(&self) -> Option<Shell> {
        return None;
    }

    ///获得关键点
    fn key_points(&self) -> Vec<RsVec3> {
        #[cfg(feature = "truck")]
        {
            return self
                .gen_unit_shape()
                .gen_brep_shell()
                .map(|x| {
                    x.vertex_iter()
                        .map(|x| RsVec3::from(x.point()))
                        .into_iter()
                        .unique()
                        .collect()
                })
                .unwrap_or(vec![Vec3::ZERO.into()]);
        }
        { Default::default() }
    }

    /// 【新增】获得增强的关键点（带类型分类和优先级）
    ///
    /// 返回：(点位置, 点类型字符串, 吸附优先级)
    ///
    /// 点类型字符串：
    /// - "Endpoint" - 端点（优先级最高）
    /// - "Midpoint" - 中点
    /// - "Center" - 中心点
    /// - "Intersection" - 交点
    /// - "SurfacePoint" - 表面点
    ///
    /// 默认实现：调用 key_points() 并标记为 SurfacePoint
    ///
    /// 各几何体可以重写此方法以提供更精确的关键点分类
    fn enhanced_key_points(
        &self,
        transform: &bevy_transform::prelude::Transform,
    ) -> Vec<(Vec3, String, u8)> {
        // 默认实现：将所有关键点标记为表面点，优先级50
        self.key_points()
            .into_iter()
            .map(|pt| {
                let world_pos = transform.transform_point(*pt);
                (world_pos, "SurfacePoint".to_string(), 50)
            })
            .collect()
    }

    ///限制参数大小，主要是对负实体的不合理进行限制
    fn apply_limit_by_size(&mut self, _limit_size: f32) {}

    fn gen_csg_shape(&self) -> anyhow::Result<crate::prim_geo::basic::CsgSharedMesh> {
        return Err(anyhow!("不存在该csg shape"));
    }

    //计算单元模型的参数hash值，也就是做成被可以复用的模型后的hash
    fn hash_unit_mesh_params(&self) -> u64 {
        0
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait>;

    ///生成对应的单位长度的模型，比如Dish，就是以R为1的情况生成模型
    /// box
    /// cylinder
    /// sphere
    #[cfg(not(target_arch = "wasm32"))]
    fn gen_unit(&self, tol_ratio: Option<f32>) -> anyhow::Result<PlantGeoData> {
        // self.gen_unit_shape().gen_plant_geo_data(tol_ratio)
        todo!("not support")
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn gen_unit_occ_shape(&self, tol_ratio: Option<f32>) -> anyhow::Result<PlantGeoData> {
        // self.gen_unit_shape().gen_plant_occ_geo(tol_ratio)
        todo!("wasm32 not support")
    }

    #[cfg(target_arch = "wasm32")]
    fn gen_unit(&self, tol_ratio: Option<f32>) -> anyhow::Result<PlantGeoData> {
        todo!("wasm32 not support")
    }

    ///获得缩放向量
    #[inline]
    fn get_scaled_vec3(&self) -> Vec3 {
        Vec3::ONE
    }

    ///获得变换矩阵
    #[inline]
    fn get_trans(&self) -> Transform {
        Transform {
            rotation: Default::default(),
            translation: Default::default(),
            scale: self.get_scaled_vec3(),
        }
    }

    #[inline]
    fn tol(&self) -> f32 {
        TRI_TOL
    }

    fn gen_plant_geo_data(&self, tol_ratio: Option<f32>) -> anyhow::Result<PlantGeoData> {
        let geo_hash = self.hash_unit_mesh_params();

        // 使用 CSG 生成网格
        if let Some(csg_mesh) = self.gen_csg_mesh() {
            let mut aabb = Aabb::new_invalid();
            for vertex in &csg_mesh.vertices {
                aabb.take_point((*vertex).into());
            }
            Ok(PlantGeoData {
                geo_hash,
                aabb: Some(aabb),
            })
        } else {
            // 尝试使用 gen_csg_shape
            let csg_shape = self.gen_csg_shape()?;
            let mesh = csg_shape.as_ref();
            let mut aabb = Aabb::new_invalid();
            for vertex in &mesh.vertices {
                aabb.take_point((*vertex).into());
            }
            Ok(PlantGeoData {
                geo_hash,
                aabb: Some(aabb),
            })
        }
    }

    ///生成mesh
    #[cfg(feature = "truck")]
    fn gen_plant_geo_data_truck(&self, tol_ratio: Option<f32>) -> Option<PlantGeoData> {
        let mut aabb = Aabb::new_invalid();
        let geo_hash = self.hash_unit_mesh_params();
        // 优先尝试使用 CSG 生成网格
        if let Some(csg_mesh) = self.gen_csg_mesh() {
            for vertex in &csg_mesh.vertices {
                aabb.take_point((*vertex).into());
            }
            return Some(PlantGeoData {
                geo_hash,
                mesh: Some(csg_mesh),
                aabb: Some(aabb),
            });
        }
        if let Some(brep) = self.gen_brep_shell() {
            let brep_bbox = gen_bounding_box(&brep);
            let d = brep_bbox.diameter() as f32;
            if d < MIN_SIZE_TOL || d > MAX_SIZE_TOL {
                return None;
            }
            let (_size, c) = (brep_bbox.diameter(), brep_bbox.center());
            let d = brep_bbox.diagonal() / 2.0;
            aabb = Aabb::from_half_extents(
                Point::<f32>::new(c[0] as f32, c[1] as f32, c[2] as f32),
                Vector::<f32>::new(d[0] as f32, d[1] as f32, d[2] as f32),
            );
            let tolerance = self.tol() as f64 * tol_ratio.unwrap_or(2.0) as f64;
            let meshed_shape = brep.triangulation(tolerance);
            let mut polygon_mesh = meshed_shape.to_polygon();
            if polygon_mesh.positions().is_empty() {
                return None;
            }
            let vertices = polygon_mesh
                .positions()
                .iter()
                .map(|&x| x.vec3())
                .collect::<Vec<_>>();
            let normals = polygon_mesh
                .normals()
                .iter()
                .map(|&x| x.vec3())
                .collect::<Vec<_>>();
            let uvs = polygon_mesh
                .uv_coords()
                .iter()
                .map(|x| [x[0] as f32, x[1] as f32])
                .collect::<Vec<_>>();
            let indices = polygon_mesh
                .faces()
                .triangle_iter()
                .flatten()
                .map(|x| x.pos as u32)
                .collect::<Vec<_>>();

            let curves = meshed_shape
                .edge_iter()
                .map(|edge| edge.curve())
                .collect::<Vec<_>>();
            let wire_vertices: Vec<Vec<Vec3>> = curves
                .iter()
                .map(|poly| poly.iter().map(|x| x.vec3()).collect::<Vec<_>>())
                .collect();

            let edges: Edges = wire_vertices
                .iter()
                .map(|v| Edge::from_vec(v.clone()))
                .collect();
            return Some(PlantGeoData {
                geo_hash,
                mesh: Some(PlantMesh {
                    indices,
                    vertices,
                    normals,
                    uvs,
                    wire_vertices,
                    edges,
                    aabb: Some(aabb),
                }),
                aabb: Some(aabb),
            });
        }
        None
    }

    fn convert_to_geo_param(&self) -> Option<PdmsGeoParam> {
        None
    }

    fn gen_csg_mesh(&self) -> Option<PlantMesh> {
        None
    }
}

impl_downcast!(BrepShapeTrait);

#[cfg(feature = "truck")]
pub trait BrepMathTrait {
    fn vector3(&self) -> Vector3;
    fn vector4(&self) -> Vector4;
    fn point3(&self) -> Point3;
    fn point3_without_z(&self) -> Point3;
}

#[cfg(feature = "truck")]
impl BrepMathTrait for Vec3 {
    #[inline]
    fn vector3(&self) -> Vector3 {
        Vector3::new(self[0] as f64, self[1] as f64, self[2] as f64)
    }

    #[inline]
    fn vector4(&self) -> Vector4 {
        Vector4::new(self[0] as f64, self[1] as f64, self[2] as f64, 0.0f64)
    }

    #[inline]
    fn point3(&self) -> Point3 {
        Point3::new(self[0] as f64, self[1] as f64, self[2] as f64)
    }

    #[inline]
    fn point3_without_z(&self) -> Point3 {
        Point3::new(
            f32_round_3(self[0]) as f64,
            f32_round_3(self[1]) as f64,
            0.0 as f64,
        )
    }
}

#[cfg(feature = "truck")]
impl BrepMathTrait for Vec4 {
    #[inline]
    fn vector3(&self) -> Vector3 {
        Vector3::new(self[0] as f64, self[1] as f64, self[2] as f64)
    }

    #[inline]
    fn vector4(&self) -> Vector4 {
        Vector4::new(
            self[0] as f64,
            self[1] as f64,
            self[2] as f64,
            self[3] as f64,
        )
    }

    #[inline]
    fn point3(&self) -> Point3 {
        Point3::new(self[0] as f64, self[1] as f64, self[2] as f64)
    }

    #[inline]
    fn point3_without_z(&self) -> Point3 {
        Point3::new(self[0] as f64, self[1] as f64, 0.0 as f64)
    }
}

pub trait BevyMathTrait {
    fn vec3(&self) -> Vec3;
    fn array(&self) -> [f32; 3];
}

#[cfg(feature = "truck")]
impl BevyMathTrait for Vector3 {
    #[inline]
    fn vec3(&self) -> Vec3 {
        Vec3::new(self[0] as f32, self[1] as f32, self[2] as f32)
    }

    #[inline]
    fn array(&self) -> [f32; 3] {
        [self[0] as f32, self[1] as f32, self[2] as f32]
    }
}

#[cfg(feature = "truck")]
impl BevyMathTrait for Point3 {
    #[inline]
    fn vec3(&self) -> Vec3 {
        Vec3::new(self[0] as f32, self[1] as f32, self[2] as f32)
    }

    #[inline]
    fn array(&self) -> [f32; 3] {
        [self[0] as f32, self[1] as f32, self[2] as f32]
    }
}

#[cfg(feature = "truck")]
#[inline]
pub fn convert_to_cg_matrix4(m: &Mat4) -> Matrix4 {
    Matrix4::from_cols(
        m.x_axis.vector4(),
        m.y_axis.vector4(),
        m.z_axis.vector4(),
        m.w_axis.vector4(),
    )
}
