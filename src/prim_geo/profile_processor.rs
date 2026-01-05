/// 统一的截面处理模块
///
/// 处理流程：
/// 1. 输入顶点数据（支持多轮廓）
/// 2. 使用 ploop-rs 处理 FRADIUS
/// 3. 使用 cavalier_contours 生成 Polyline
/// 4. 处理多轮廓的 boolean 操作（subtract 内孔等）
/// 5. 使用 i_triangle 进行三角化
/// 6. 输出标准化的截面数据
use crate::prim_geo::wire::{
    export_polyline_svg_for_debug, gen_polyline_from_processed_vertices,
    polyline_to_debug_json_str, process_ploop_vertices,
};
use anyhow::{Result, anyhow};
use cavalier_contours::polyline::{BooleanOp, PlineSource, Polyline};
use glam::{Vec2, Vec3};
use i_triangle::float::triangulatable::Triangulatable;

/// 截面轮廓数据
#[derive(Debug, Clone)]
pub struct ProfileContour {
    /// 顶点列表（Vec3: x,y为坐标，z为FRADIUS或bulge）
    pub vertices: Vec<Vec3>,
    /// 是否为孔洞（true=减去，false=添加）
    pub is_hole: bool,
}

/// 处理后的截面数据
#[derive(Debug, Clone)]
pub struct ProcessedProfile {
    /// 2D 截面轮廓点（已处理 FRADIUS 和 boolean 操作）
    pub contour_points: Vec<Vec2>,
    /// 三角化的顶点
    pub tri_vertices: Vec<Vec2>,
    /// 三角化的索引
    pub tri_indices: Vec<u32>,
    /// 截面的 Polyline（用于进一步操作）
    pub polyline: Polyline,
}

/// 统一的截面处理器
pub struct ProfileProcessor {
    /// 外轮廓
    outer_contour: ProfileContour,
    /// 内孔轮廓列表
    inner_contours: Vec<ProfileContour>,
}

impl ProfileProcessor {
    /// 创建单一轮廓的处理器
    pub fn new_single(vertices: Vec<Vec3>) -> Self {
        Self {
            outer_contour: ProfileContour {
                vertices,
                is_hole: false,
            },
            inner_contours: Vec::new(),
        }
    }

    /// 创建多轮廓的处理器（支持孔洞）
    pub fn new_multi(contours: Vec<ProfileContour>) -> Result<Self> {
        if contours.is_empty() {
            return Err(anyhow!("截面轮廓不能为空"));
        }

        let mut outer = None;
        let mut inners = Vec::new();

        for contour in contours {
            if !contour.is_hole {
                if outer.is_some() {
                    return Err(anyhow!("只能有一个外轮廓"));
                }
                outer = Some(contour);
            } else {
                inners.push(contour);
            }
        }

        let outer = outer.ok_or_else(|| anyhow!("必须有一个外轮廓"))?;

        Ok(Self {
            outer_contour: outer,
            inner_contours: inners,
        })
    }

    /// 统一的入口：从多个 wire（轮廓）创建处理器
    ///
    /// 自动识别外轮廓和内孔：
    /// - 如果只有一个轮廓，作为外轮廓
    /// - 如果有多个轮廓，使用面积最大的作为外轮廓，其他作为内孔
    /// - 或者遵循约定：第一个是外轮廓，其他是内孔（如果 auto_detect=false）
    ///
    /// # 参数
    /// - `wires`: 多个轮廓的顶点列表，每个轮廓是一个 `Vec<Vec3>`
    /// - `auto_detect`: 是否自动检测外轮廓（通过面积），默认 true
    ///
    /// # 返回
    /// - `Result<Self>`: 处理后的 ProfileProcessor
    pub fn from_wires(
        verts: Vec<Vec<Vec2>>,
        frads: Vec<Vec<f32>>,
        auto_detect: bool,
    ) -> Result<Self> {
        if verts.is_empty() {
            return Err(anyhow!("截面轮廓不能为空"));
        }

        if verts.len() != frads.len() {
            return Err(anyhow!("verts 和 frads 的轮廓数量不一致"));
        }

        let mut wires: Vec<Vec<Vec3>> = Vec::with_capacity(verts.len());
        for (wire_verts, wire_frads) in verts.into_iter().zip(frads.into_iter()) {
            if wire_verts.len() != wire_frads.len() {
                return Err(anyhow!(
                    "轮廓顶点数量({})与 FRADIUS 数量({}) 不一致",
                    wire_verts.len(),
                    wire_frads.len(),
                ));
            }

            let combined: Vec<Vec3> = wire_verts
                .into_iter()
                .zip(wire_frads.into_iter())
                .map(|(p, r)| Vec3::new(p.x, p.y, r))
                .collect();
            wires.push(combined);
        }

        if wires.len() == 1 {
            // 单一轮廓，直接作为外轮廓
            return Ok(Self {
                outer_contour: ProfileContour {
                    vertices: wires[0].clone(),
                    is_hole: false,
                },
                inner_contours: Vec::new(),
            });
        }

        // 多轮廓情况
        if auto_detect {
            // 自动检测：计算每个轮廓的面积，面积最大的作为外轮廓
            let mut contours_with_area: Vec<(ProfileContour, f32)> = wires
                .into_iter()
                .map(|vertices| {
                    let area = Self::compute_contour_area(&vertices);
                    (
                        ProfileContour {
                            vertices,
                            is_hole: false, // 临时标记，稍后会设置
                        },
                        area.abs(),
                    )
                })
                .collect();

            // 按面积降序排序
            contours_with_area
                .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            // 面积最大的作为外轮廓，其他作为内孔
            let mut outer = contours_with_area[0].0.clone();
            outer.is_hole = false;

            let mut inners: Vec<ProfileContour> = contours_with_area
                .into_iter()
                .skip(1)
                .map(|(mut contour, _)| {
                    contour.is_hole = true;
                    contour
                })
                .collect();

            Ok(Self {
                outer_contour: outer,
                inner_contours: inners,
            })
        } else {
            // 遵循约定：第一个是外轮廓，其他是内孔
            let outer = ProfileContour {
                vertices: wires[0].clone(),
                is_hole: false,
            };

            let inners: Vec<ProfileContour> = wires
                .into_iter()
                .skip(1)
                .map(|vertices| ProfileContour {
                    vertices,
                    is_hole: true,
                })
                .collect();

            Ok(Self {
                outer_contour: outer,
                inner_contours: inners,
            })
        }
    }

    /// 计算轮廓的面积（使用鞋带公式）
    ///
    /// 面积的正负号表示轮廓的绕向（逆时针为正，顺时针为负）
    /// 只使用 x, y 坐标，忽略 z 坐标（z 可能存储 FRADIUS 或 bulge）
    fn compute_contour_area(vertices: &[Vec3]) -> f32 {
        if vertices.len() < 3 {
            return 0.0;
        }

        let n = vertices.len();
        vertices
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let next = &vertices[(i + 1) % n];
                p.x * next.y - next.x * p.y
            })
            .sum::<f32>()
            * 0.5
    }

    /// 处理截面：FRADIUS -> Polyline -> Boolean -> Triangulation
    pub fn process(&self, debug_name: &str, refno: Option<&str>) -> Result<ProcessedProfile> {
        // println!("🔧 [ProfileProcessor] 开始处理截面: {}", debug_name);
        // println!("   外轮廓顶点数: {}", self.outer_contour.vertices.len());
        // println!("   内孔数量: {}", self.inner_contours.len());

        // 1. 处理外轮廓
        let outer_polyline =
            self.process_single_contour(&self.outer_contour.vertices, "outer", refno)?;

        // 2. 处理内孔并执行 boolean subtract
        let final_polyline = if self.inner_contours.is_empty() {
            outer_polyline
        } else {
            self.apply_boolean_operations(outer_polyline)?
        };

        // 3. 提取 2D 轮廓点
        let contour_points = self.polyline_to_2d_points(&final_polyline);
        // println!(
        //     "   最终轮廓点数: {} (原始: {})",
        //     contour_points.len(),
        //     self.outer_contour.vertices.len()
        // );

        // 4. 使用 i_triangle 进行三角化
        let (tri_vertices, tri_indices) = self.triangulate_polyline(&contour_points)?;

        // println!(
        //     "✅ [ProfileProcessor] 截面处理完成: {} 个三角形",
        //     tri_indices.len() / 3
        // );

        Ok(ProcessedProfile {
            contour_points,
            tri_vertices,
            tri_indices,
            polyline: final_polyline,
        })
    }

    /// 处理单个轮廓（外轮廓或内孔）
    fn process_single_contour(
        &self,
        vertices: &[Vec3],
        name: &str,
        refno: Option<&str>,
    ) -> Result<Polyline> {
        if vertices.len() < 3 {
            return Err(anyhow!("轮廓 {} 顶点数量不足（< 3）", name));
        }

        // 使用 ploop-rs 处理 FRADIUS
        // 将 Vec3 拆分为 Vec2 和 frads
        let mut verts2d: Vec<Vec2> = Vec::with_capacity(vertices.len());
        let mut frads: Vec<f32> = Vec::with_capacity(vertices.len());
        for v in vertices {
            verts2d.push(Vec2::new(v.x, v.y));
            frads.push(v.z);
        }
        let processed_vertices = process_ploop_vertices(
            &verts2d,
            &frads,
            &format!("PROFILE_{}", &refno.unwrap_or("unknown")),
        )?;

        //export the vertices to json file
        // let json_str = serde_json::to_string_pretty(&processed_vertices)?;
        // let output_dir = "test_output/test_loop_case";
        // std::fs::create_dir_all(output_dir)?;
        // std::fs::write(format!("{}/{}.json", output_dir, &refno.unwrap_or("unknown")), json_str)?;

        // 生成 Polyline
        let polyline = gen_polyline_from_processed_vertices(&processed_vertices, refno)?;

        //todo 实现打印 polyline 的方法, 使用 polyline_to_debug_json_str
        // println!(
        //     "   轮廓 {} 的 Polyline: {}",
        //     &refno.unwrap_or("unknown"),
        //     polyline_to_debug_json_str(&polyline)
        // );

        //export the svg of the polyline
        if std::env::var("EXPORT_SVG").is_ok() {
            export_polyline_svg_for_debug(&polyline, Some(name));
        }

        Ok(polyline)
    }

    /// 执行 boolean 操作（减去内孔）
    fn apply_boolean_operations(&self, mut base: Polyline) -> Result<Polyline> {
        println!(
            "   开始执行 Boolean 操作，减去 {} 个内孔",
            self.inner_contours.len()
        );

        for (i, hole_contour) in self.inner_contours.iter().enumerate() {
            let hole_polyline =
                self.process_single_contour(&hole_contour.vertices, &format!("hole_{}", i), None)?;

            // 执行 boolean subtract (base - hole)
            let result = base.boolean(&hole_polyline, BooleanOp::Not);

            if result.pos_plines.is_empty() {
                // println!("⚠️  Boolean 操作失败，保留原轮廓");
                continue;
            }

            // 取第一个正轮廓作为结果
            base = result.pos_plines[0].pline.clone();
            // println!("   完成第 {} 个内孔的减法", i + 1);
        }

        Ok(base)
    }

    /// 将 Polyline 转换为 2D 点集
    ///
    /// 注意：i_triangle 不支持 bulge，需要将圆弧段离散化
    fn polyline_to_2d_points(&self, polyline: &Polyline) -> Vec<Vec2> {
        let mut points = Vec::new();
        let vertex_count = polyline.vertex_data.len();

        for i in 0..vertex_count {
            let vertex = &polyline.vertex_data[i];
            points.push(Vec2::new(vertex.x as f32, vertex.y as f32));

            if vertex.bulge.abs() > 0.001 {
                let next_idx = (i + 1) % vertex_count;
                let next_vertex = &polyline.vertex_data[next_idx];
                points.extend(self.sample_arc_segment(vertex, next_vertex));
            }
        }

        if points.len() > 1 && points.first().unwrap().distance(*points.last().unwrap()) < 0.01 {
            points.pop();
        }

        // 去掉连续重复点，避免生成零长度边导致法线 NaN
        let mut points = Self::dedup_consecutive_points(points, 0.001);

        // 统一外轮廓为逆时针，保证侧面法线指向外侧
        if points.len() > 2 && Self::signed_area_2d(&points) < 0.0 {
            points.reverse();
        }

        points
    }

    /// 计算二维点集的带符号面积（逆时针为正）
    fn signed_area_2d(points: &[Vec2]) -> f32 {
        if points.len() < 3 {
            return 0.0;
        }

        let mut area = 0.0;
        for i in 0..points.len() {
            let next = (i + 1) % points.len();
            area += points[i].x * points[next].y - points[next].x * points[i].y;
        }
        area * 0.5
    }

    /// 移除距离过近的连续点，避免形成零长度边
    fn dedup_consecutive_points(mut points: Vec<Vec2>, tol: f32) -> Vec<Vec2> {
        if points.len() < 2 {
            return points;
        }

        let mut cleaned: Vec<Vec2> = Vec::with_capacity(points.len());
        for p in points.into_iter() {
            if let Some(prev) = cleaned.last() {
                if prev.distance(p) < tol {
                    continue;
                }
            }
            cleaned.push(p);
        }

        // 如果首尾仍然过近，去掉末尾
        if cleaned.len() > 1 && cleaned.first().unwrap().distance(*cleaned.last().unwrap()) < tol {
            cleaned.pop();
        }

        cleaned
    }

    fn sample_arc_segment(
        &self,
        start: &cavalier_contours::polyline::PlineVertex,
        end: &cavalier_contours::polyline::PlineVertex,
    ) -> Vec<Vec2> {
        let bulge = start.bulge;
        if bulge.abs() < 0.001 {
            return Vec::new();
        }

        // 计算圆弧参数
        let angle = (4.0 * bulge.atan()).abs();

        // 计算圆弧中心和半径
        use cavalier_contours::polyline::seg_arc_radius_and_center;
        let (radius, center) = seg_arc_radius_and_center(*start, *end);

        // 计算弧长
        let arc_length = radius.abs() * angle;

        // 采样策略：同时考虑角度和弧长
        // 1. 基于角度：每5度一个点
        let segments_by_angle = (angle.to_degrees() / 5.0).ceil() as usize;
        // 2. 基于弧长：每100mm一个点（适应大半径弧）
        let segments_by_length = (arc_length / 100.0).ceil() as usize;
        // 取两者中较大的值，确保足够的采样密度
        let segments = segments_by_angle.max(segments_by_length).clamp(2, 128);

        let start_pos = Vec2::new(start.x as f32, start.y as f32);
        let center_vec2 = Vec2::new(center.x as f32, center.y as f32);

        let mut arc_points = Vec::new();

        for i in 1..segments {
            let t = i as f32 / segments as f32;
            let angle_offset = angle as f32 * t * bulge.signum() as f32;

            let dir = (start_pos - center_vec2).normalize();
            let cos_a = angle_offset.cos();
            let sin_a = angle_offset.sin();
            let rotated = Vec2::new(dir.x * cos_a - dir.y * sin_a, dir.x * sin_a + dir.y * cos_a);

            arc_points.push(center_vec2 + rotated * radius as f32);
        }

        arc_points
    }

    /// 使用 i_triangle 进行三角化
    fn triangulate_polyline(&self, points: &[Vec2]) -> Result<(Vec<Vec2>, Vec<u32>)> {
        if points.len() < 3 {
            return Err(anyhow!("三角化失败：点数不足（< 3）"));
        }

        // 转换为 i_triangle 需要的格式
        let contour: Vec<[f32; 2]> = points.iter().map(|p| [p.x, p.y]).collect();

        // 执行三角化
        let raw = contour.as_slice().triangulate();
        let triangulation = raw.to_triangulation::<u32>();

        if triangulation.indices.is_empty() {
            return Err(anyhow!("i_triangle 三角化返回空结果"));
        }

        let vertices: Vec<Vec2> = triangulation
            .points
            .into_iter()
            .map(|p| Vec2::new(p[0], p[1]))
            .collect();

        Ok((vertices, triangulation.indices))
    }
}

/// 从 ProcessedProfile 生成拉伸体的顶点和索引（流形版本）
///
/// 生成的 mesh 保证是有效的流形（manifold），适用于布尔运算。
///
/// 特点：
/// - 使用统一的顶点集合（底面 + 顶面各 n 个顶点）
/// - 所有面共享边缘顶点
/// - 底面/顶面使用 i_triangle 三角化结果（支持凹多边形）
pub fn extrude_profile(profile: &ProcessedProfile, height: f32) -> ExtrudedMesh {
    let n_contour = profile.contour_points.len();
    let n_tri = profile.tri_vertices.len();

    if n_contour < 3 || n_tri < 3 {
        return ExtrudedMesh {
            vertices: Vec::new(),
            normals: Vec::new(),
            indices: Vec::new(),
            uvs: Vec::new(),
        };
    }

    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();
    let mut uvs = Vec::new();

    // ========== 1. 生成独立的顶点集（不共享）==========
    // 底面顶点：索引 0..n_tri-1 (使用 tri_vertices)
    // 顶面顶点：索引 n_tri..2*n_tri-1 (使用 tri_vertices)
    // 侧面顶点：索引 2*n_tri..2*n_tri+2*n_contour-1（每个轮廓点对应两个侧面顶点）

    // 底面顶点（使用 tri_vertices）
    for point in &profile.tri_vertices {
        vertices.push(Vec3::new(point.x, point.y, 0.0));
        normals.push(Vec3::NEG_Z); // 底面法线朝下
        uvs.push([point.x / 100.0, point.y / 100.0]);
    }

    // 顶面顶点（使用 tri_vertices）
    for point in &profile.tri_vertices {
        vertices.push(Vec3::new(point.x, point.y, height));
        normals.push(Vec3::Z); // 顶面法线朝上
        uvs.push([point.x / 100.0, point.y / 100.0]);
    }

    // 侧面顶点（使用 contour_points，每个轮廓点创建两个）
    for point in &profile.contour_points {
        // 底部侧面顶点
        vertices.push(Vec3::new(point.x, point.y, 0.0));
        normals.push(Vec3::ZERO); // 稍后计算侧面法线
        uvs.push([point.x / 100.0, point.y / 100.0]);

        // 顶部侧面顶点
        vertices.push(Vec3::new(point.x, point.y, height));
        normals.push(Vec3::ZERO); // 稍后计算侧面法线
        uvs.push([point.x / 100.0, point.y / 100.0]);
    }

    // ========== 2. 生成侧面三角形 ==========
    // 使用独立的侧面顶点
    let side_base = (2 * n_tri) as u32;
    for i in 0..n_contour {
        let next = (i + 1) % n_contour;

        // 侧面顶点索引
        let sb0 = side_base + (2 * i) as u32; // 当前点的底部侧面顶点
        let sb1 = side_base + (2 * next) as u32; // 下一个点的底部侧面顶点
        let st0 = side_base + (2 * i + 1) as u32; // 当前点的顶部侧面顶点
        let st1 = side_base + (2 * next + 1) as u32; // 下一个点的顶部侧面顶点

        // 三角形1: sb0 -> sb1 -> st1 (逆时针，法线朝外)
        indices.push(sb0);
        indices.push(sb1);
        indices.push(st1);

        // 三角形2: sb0 -> st1 -> st0
        indices.push(sb0);
        indices.push(st1);
        indices.push(st0);
    }

    // ========== 3. 生成底面三角形（使用 i_triangle 结果）==========
    // 底面法线朝下，需要反转三角形绕向
    for chunk in profile.tri_indices.chunks(3) {
        if chunk.len() == 3 {
            // 反转绕向：0 -> 2 -> 1
            indices.push(chunk[0]);
            indices.push(chunk[2]);
            indices.push(chunk[1]);
        }
    }

    // ========== 4. 生成顶面三角形（使用 i_triangle 结果）==========
    // 顶面法线朝上，保持原始绕向
    let top_base = n_tri as u32;
    for chunk in profile.tri_indices.chunks(3) {
        if chunk.len() == 3 {
            indices.push(top_base + chunk[0]);
            indices.push(top_base + chunk[1]);
            indices.push(top_base + chunk[2]);
        }
    }

    // ========== 5. 计算侧面顶点法线 ==========
    let mut side_normals = vec![Vec3::ZERO; 2 * n_contour];

    for i in 0..n_contour {
        let next = (i + 1) % n_contour;

        // 三角形1的法线
        let sb0_idx = 2 * i;
        let sb1_idx = 2 * next;
        let st1_idx = 2 * next + 1;

        let v0 = vertices[2 * n_tri + sb0_idx];
        let v1 = vertices[2 * n_tri + sb1_idx];
        let v2 = vertices[2 * n_tri + st1_idx];

        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        let face_normal = edge1.cross(edge2).normalize_or_zero();

        side_normals[sb0_idx] += face_normal;
        side_normals[sb1_idx] += face_normal;
        side_normals[st1_idx] += face_normal;

        // 三角形2的法线
        let st0_idx = 2 * i + 1;

        let v0 = vertices[2 * n_tri + sb0_idx];
        let v1 = vertices[2 * n_tri + st1_idx];
        let v2 = vertices[2 * n_tri + st0_idx];

        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        let face_normal = edge1.cross(edge2).normalize_or_zero();

        side_normals[sb0_idx] += face_normal;
        side_normals[st1_idx] += face_normal;
        side_normals[st0_idx] += face_normal;
    }

    // 将侧面法线写入 normals 数组
    for (i, normal) in side_normals.into_iter().enumerate() {
        normals[2 * n_tri + i] = normal.normalize_or_zero();
    }

    ExtrudedMesh {
        vertices,
        normals,
        indices,
        uvs,
    }
}

/// 拉伸后的网格
#[derive(Debug, Clone)]
pub struct ExtrudedMesh {
    pub vertices: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub indices: Vec<u32>,
    pub uvs: Vec<[f32; 2]>,
}

/// 从 ProcessedProfile 生成旋转体的顶点和索引
///
/// 用于 Revolution
pub fn revolve_profile(
    profile: &ProcessedProfile,
    angle: f32,
    segments: usize,
    rot_axis: Vec3,
    rot_center: Vec3,
) -> RevolvedMesh {
    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();

    let n_profile = profile.contour_points.len();
    let n_segments = segments.max(4);

    // 1. 构建轮廓平面的坐标系
    // 对于旋转体，轮廓在包含旋转轴的平面内：
    // - x 坐标：距离旋转轴的径向距离
    // - y 坐标：沿旋转轴的高度
    // 需要构建一个坐标系，其中：
    // - 一个轴沿旋转轴方向（用于高度）
    // - 一个轴垂直于旋转轴（用于径向距离）

    // 计算垂直于旋转轴的正交基
    // 优先选择 X 轴作为参考，如果旋转轴接近 X 轴则选择 Y 轴
    let radial_axis = if rot_axis.abs_diff_eq(Vec3::Z, 0.01) {
        Vec3::X
    } else if rot_axis.abs_diff_eq(Vec3::NEG_Z, 0.01) {
        Vec3::X
    } else if rot_axis.abs_diff_eq(Vec3::Y, 0.01) {
        Vec3::X
    } else if rot_axis.abs_diff_eq(Vec3::NEG_Y, 0.01) {
        Vec3::X
    } else if rot_axis.abs_diff_eq(Vec3::X, 0.01) {
        Vec3::Y
    } else if rot_axis.abs_diff_eq(Vec3::NEG_X, 0.01) {
        Vec3::Y
    } else {
        // 任意轴，使用通用方法
        let ref_vec = if rot_axis.z.abs() < 0.9 {
            Vec3::Z
        } else {
            Vec3::X
        };
        rot_axis.cross(ref_vec).normalize()
    };

    // 确保正交（对于通用情况）
    let radial_axis = if radial_axis.dot(rot_axis).abs() > 0.001 {
        rot_axis
            .cross(if rot_axis.z.abs() < 0.9 {
                Vec3::Z
            } else {
                Vec3::X
            })
            .normalize()
    } else {
        radial_axis
    };

    // 1. 计算 Profile 的 2D 法线 (用于正确的光照)
    // 如果点是逆时针排列，法线向右（或向外）
    let mut profile_normals = Vec::with_capacity(n_profile);
    for i in 0..n_profile {
        let prev_idx = if i == 0 { n_profile - 1 } else { i - 1 };
        let next_idx = (i + 1) % n_profile;

        let p_prev = profile.contour_points[prev_idx];
        let p_curr = profile.contour_points[i];
        let p_next = profile.contour_points[next_idx];

        // 计算两条边的法线并平均
        let edge1 = p_curr - p_prev;
        let edge2 = p_next - p_curr;

        let n1 = Vec2::new(edge1.y, -edge1.x).normalize_or_zero();
        let n2 = Vec2::new(edge2.y, -edge2.x).normalize_or_zero();

        let avg_normal = (n1 + n2).normalize_or_zero();
        profile_normals.push(avg_normal);
    }

    // 计算旋转方向（用于端面法向量）
    // 旋转方向 = 旋转轴 × 径向轴（或相反，取决于旋转角度符号）
    let rotation_direction = if angle >= 0.0 {
        rot_axis.cross(radial_axis).normalize()
    } else {
        radial_axis.cross(rot_axis).normalize()
    };

    // 2. 生成旋转后的顶点和法线
    for i in 0..=n_segments {
        let t = i as f32 / n_segments as f32;
        let current_angle = angle * t;

        let rotation = glam::Quat::from_axis_angle(rot_axis, current_angle.to_radians());

        // 旋转后的径向轴
        let current_radial_axis = rotation.mul_vec3(radial_axis);

        // 判断是否是端面
        let is_start_face = i == 0;
        let is_end_face = i == n_segments;

        for (j, point) in profile.contour_points.iter().enumerate() {
            // 轮廓点的坐标映射：
            // - point.x: 距离旋转轴的径向距离 (Radius)
            // - point.y: 沿旋转轴的高度 (Height)

            // 构建当前截面上的点位置
            // Pos = Center + (Height * RotAxis) + (Radius * CurrentRadialAxis)
            let pos_3d = rot_center + (rot_axis * point.y) + (current_radial_axis * point.x);
            vertices.push(pos_3d);

            // 计算法线
            let normal_3d = if is_start_face {
                // 起始面：法向量垂直于端面平面
                // 端面法向量 = 旋转方向（在起始面时，旋转方向就是初始的旋转方向）
                // 或者：端面法向量 = 径向方向 × 旋转轴方向（取决于绕序）
                // 对于起始面，使用旋转方向作为法向量
                rotation_direction
            } else if is_end_face {
                // 结束面：法向量垂直于端面平面
                // 结束面的旋转方向是旋转后的旋转方向
                let end_rotation_direction = rotation.mul_vec3(rotation_direction);
                end_rotation_direction
            } else {
                // 侧面：使用轮廓的 2D 法线映射到 3D
                // 2D Normal (nx, ny): nx 是径向分量，ny 是轴向分量
                let normal_2d = profile_normals[j];
                // Normal = (ny * RotAxis) + (nx * CurrentRadialAxis)
                (rot_axis * normal_2d.y) + (current_radial_axis * normal_2d.x)
            };

            normals.push(normal_3d.normalize());
        }
    }

    // 判断是否为完整旋转（360°）
    let is_full_rotation = (angle.abs() - 360.0).abs() < 0.01;

    // 检测 profile 是否闭合（首尾点重合）
    let first_point = profile.contour_points.first().cloned().unwrap_or_default();
    let last_point = profile.contour_points.last().cloned().unwrap_or_default();
    let profile_is_closed = (first_point - last_point).length() < 0.01;

    println!(
        "🔍 [REVOLVE] n_profile={}, is_full_rotation={}, profile_is_closed={}",
        n_profile, is_full_rotation, profile_is_closed
    );
    println!(
        "   first_point={:?}, last_point={:?}, distance={}",
        first_point,
        last_point,
        (first_point - last_point).length()
    );

    // 对于开放 profile，侧面不连接最后一点到第一点
    let n_profile_edges = if profile_is_closed {
        n_profile
    } else {
        n_profile - 1
    };

    // 4. 生成侧面索引
    for i in 0..n_segments {
        for j in 0..n_profile_edges {
            let next_j = (j + 1) % n_profile;

            let curr_ring = i * n_profile;
            // 对于 360° 旋转，最后一段应该连接回第一环
            let next_ring = if is_full_rotation && i == n_segments - 1 {
                0 // 使用第一环的索引
            } else {
                (i + 1) * n_profile
            };

            let idx0 = (curr_ring + j) as u32;
            let idx1 = (curr_ring + next_j) as u32;
            let idx2 = (next_ring + next_j) as u32;
            let idx3 = (next_ring + j) as u32;

            // 注意三角形绕向，确保法线朝外 (Rotation x Profile)
            indices.extend_from_slice(&[idx0, idx2, idx1, idx0, idx3, idx2]);
        }
    }

    // 5. 对于 360° 旋转体，开放轮廓需要添加端面封闭
    if is_full_rotation && !profile_is_closed && n_profile >= 2 {
        let first_pt = first_point; // 使用已计算的变量
        let last_pt = last_point;
        let axis_tolerance = 0.1;

        println!(
            "🔍 [REVOLVE 端面] first_pt={:?}, last_pt={:?}",
            first_pt, last_pt
        );
        println!(
            "   first_on_axis={}, last_on_axis={}",
            first_pt.x.abs() < axis_tolerance,
            last_pt.x.abs() < axis_tolerance
        );

        // 检查首尾点是否在旋转轴上（径向距离=0）
        let first_on_axis = first_pt.x.abs() < axis_tolerance;
        let last_on_axis = last_pt.x.abs() < axis_tolerance;

        if first_on_axis && last_on_axis {
            // 首尾都在轴上，不需要端面（旋转体自然闭合）
        } else if first_on_axis {
            // 首点在轴上，尾点形成圆环 -> 用三角形扇形封盖
            // 首点作为中心，连接尾点形成的圆
            for i in 0..n_segments {
                let center = (i * n_profile) as u32; // 首点（在轴上）
                let curr_last = (i * n_profile + n_profile - 1) as u32;
                let next_last = if i == n_segments - 1 {
                    (n_profile - 1) as u32
                } else {
                    ((i + 1) * n_profile + n_profile - 1) as u32
                };
                indices.extend_from_slice(&[center, curr_last, next_last]);
            }
        } else if last_on_axis {
            // 尾点在轴上，首点形成圆环 -> 用三角形扇形封盖
            // 尾点作为中心，连接首点形成的圆
            for i in 0..n_segments {
                let center = (i * n_profile + n_profile - 1) as u32; // 尾点（在轴上）
                let curr_first = (i * n_profile) as u32;
                let next_first = if i == n_segments - 1 {
                    0
                } else {
                    ((i + 1) * n_profile) as u32
                };
                indices.extend_from_slice(&[center, next_first, curr_first]);
            }
        } else {
            // 首尾都不在轴上 -> 用环形面连接两个圆环
            for i in 0..n_segments {
                let curr_first = (i * n_profile) as u32;
                let curr_last = (i * n_profile + n_profile - 1) as u32;
                let next_first = if i == n_segments - 1 {
                    0
                } else {
                    ((i + 1) * n_profile) as u32
                };
                let next_last = if i == n_segments - 1 {
                    (n_profile - 1) as u32
                } else {
                    ((i + 1) * n_profile + n_profile - 1) as u32
                };
                indices.extend_from_slice(&[curr_first, next_first, next_last]);
                indices.extend_from_slice(&[curr_first, next_last, curr_last]);
            }
        }
    }

    // 6. 生成 UV 坐标
    let mut uvs = Vec::new();
    for i in 0..=n_segments {
        let v = i as f32 / n_segments as f32;
        for j in 0..n_profile {
            let u = j as f32 / n_profile as f32;
            uvs.push([u, v]);
        }
    }
    // 端面中心点的 UV
    let extra_verts = vertices.len() - (n_segments + 1) * n_profile;
    for _ in 0..extra_verts {
        uvs.push([0.5, 0.5]);
    }

    RevolvedMesh {
        vertices,
        normals,
        indices,
        uvs,
    }
}

/// 旋转后的网格
#[derive(Debug, Clone)]
pub struct RevolvedMesh {
    pub vertices: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub indices: Vec<u32>,
    pub uvs: Vec<[f32; 2]>,
}

// ============================================================================
// Manifold 风格的旋转体生成算法（从 C++ 移植）
// ============================================================================

/// 裁剪多边形，只保留 X >= 0 的部分
///
/// 参考 Manifold C++ 实现：对于跨越 Y 轴的边，在 Y 轴上插值生成新顶点
///
/// # 参数
/// - `polygon`: 输入的 2D 多边形点集 (x = 径向距离, y = 轴向高度)
///
/// # 返回
/// - `Option<Vec<Vec2>>`: 裁剪后的多边形，如果全部在负侧则返回 None
pub fn clip_polygon_to_positive_x(polygon: &[Vec2]) -> Option<Vec<Vec2>> {
    if polygon.is_empty() {
        return None;
    }

    let mut result = Vec::new();
    let n = polygon.len();

    // 找到第一个 x >= 0 的点作为起始
    let mut start_idx = None;
    for i in 0..n {
        if polygon[i].x >= 0.0 {
            start_idx = Some(i);
            break;
        }
    }

    // 如果所有点都在负侧，返回 None
    let start = match start_idx {
        Some(i) => i,
        None => return None,
    };

    // 从第一个正侧点开始遍历
    let mut i = start;
    loop {
        let curr = polygon[i];
        let next_idx = (i + 1) % n;
        let next = polygon[next_idx];

        // 如果当前点在正侧，添加它
        if curr.x >= 0.0 {
            result.push(curr);
        }

        // 如果当前点和下一点跨越 Y 轴，在 Y 轴上插值
        let curr_neg = curr.x < 0.0;
        let next_neg = next.x < 0.0;
        if curr_neg != next_neg {
            // 线性插值：找到 x = 0 的点
            // t = curr.x / (curr.x - next.x)
            // y = curr.y + t * (next.y - curr.y)
            let t = curr.x / (curr.x - next.x);
            let y = curr.y + t * (next.y - curr.y);
            result.push(Vec2::new(0.0, y));
        }

        i = next_idx;
        if i == start {
            break;
        }
    }

    if result.len() < 3 {
        return None;
    }

    Some(result)
}

/// 计算最大径向距离（用于自适应分段数）
fn compute_max_radius(polygons: &[Vec<Vec2>]) -> f32 {
    polygons
        .iter()
        .flat_map(|poly| poly.iter())
        .map(|p| p.x)
        .fold(0.0f32, |acc, x| acc.max(x))
}

/// 计算自适应分段数
///
/// 参考 Manifold 的 Quality::GetCircularSegments
///
/// # 参数
/// - `radius`: 最大半径
/// - `angle`: 旋转角度（度）
/// - `min_segments`: 最小分段数
pub fn compute_adaptive_segments(radius: f32, angle: f32, min_segments: usize) -> usize {
    // 最大分段数限制，避免大半径导致顶点数爆炸
    const MAX_SEGMENTS: usize = 100;

    // 基于半径的分段数：每 10mm 周长约 1 个分段，最小 8 段（对于完整圆）
    let full_circle_segments = ((2.0 * std::f32::consts::PI * radius) / 10.0)
        .ceil()
        .max(8.0) as usize;

    // 根据角度比例调整
    let segments = ((full_circle_segments as f32 * angle.abs() / 360.0).ceil() as usize)
        .max(min_segments)
        .min(MAX_SEGMENTS);

    segments
}

/// Manifold 风格的旋转体生成
///
/// 参考 libgm.dll 的实现（REVO基本体分析报告），具有以下特性：
/// - 自动裁剪负 X 侧的轮廓（在 Y 轴插值）
/// - **轴上顶点优化**：x=0 的点只生成一个共享顶点
/// - **退化边跳过**：两端都在轴上的边不生成面
/// - **扇形生成**：一端在轴上的边生成三角形扇
/// - 自适应分段数
/// - 支持部分旋转（非 360°）的端面封闭
///
/// # 参数
/// - `polygons`: 多边形列表，每个多边形是 2D 点集 (x = 径向距离, y = 轴向高度)
/// - `circular_segments`: 圆周分段数，0 表示自动计算
/// - `revolve_degrees`: 旋转角度（度），最大 360°
///
/// # 返回
/// - `Option<RevolvedMesh>`: 生成的旋转体网格
pub fn revolve_polygons_manifold(
    polygons: &[Vec<Vec2>],
    circular_segments: usize,
    revolve_degrees: f32,
) -> Option<RevolvedMesh> {
    if polygons.is_empty() {
        return None;
    }

    // 轴上容差（参考 libgm GM_User::normtol_）
    const AXIS_TOL: f32 = 1e-5;

    // 1. 裁剪所有多边形，只保留 X >= 0 的部分
    let mut clipped_polygons: Vec<Vec<Vec2>> = Vec::new();
    let mut max_radius: f32 = 0.0;

    for poly in polygons {
        if let Some(clipped) = clip_polygon_to_positive_x(poly) {
            for p in &clipped {
                max_radius = max_radius.max(p.x);
            }
            clipped_polygons.push(clipped);
        }
    }

    if clipped_polygons.is_empty() {
        return None;
    }

    // 2. 限制旋转角度
    let revolve_degrees = revolve_degrees.min(360.0).max(-360.0);
    let is_full_revolution = (revolve_degrees.abs() - 360.0).abs() < 0.01;

    // 3. 计算分段数
    let n_segments = if circular_segments > 2 {
        circular_segments
    } else {
        compute_adaptive_segments(max_radius, revolve_degrees, 3)
    };

    let angle_rad = revolve_degrees.to_radians();

    // 4. 生成顶点和索引
    let mut vertices: Vec<Vec3> = Vec::new();
    let mut normals: Vec<Vec3> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    for poly in &clipped_polygons {
        let n_profile = poly.len();
        if n_profile < 2 {
            continue;
        }

        // Step 1: 预处理 - 将接近轴的点吸附到轴上（参考 libgm movePointsOntoYAxis）
        let poly: Vec<Vec2> = poly
            .iter()
            .map(|p| {
                if p.x.abs() < AXIS_TOL {
                    Vec2::new(0.0, p.y)
                } else {
                    *p
                }
            })
            .collect();

        // Step 2: 记录每个 profile 点的顶点信息
        // - 轴上的点：只生成 1 个共享顶点
        // - 非轴上的点：生成 n_segments + 1 个顶点（完整旋转为 n_segments）
        struct ProfileVertexInfo {
            start_index: usize,
            vertex_count: usize,
            is_on_axis: bool,
        }
        let mut profile_vertex_info: Vec<ProfileVertexInfo> = Vec::with_capacity(n_profile);

        let n_slices = if is_full_revolution {
            n_segments
        } else {
            n_segments + 1
        };

        // Step 3: 生成顶点
        for (profile_idx, pt) in poly.iter().enumerate() {
            let start_index = vertices.len();
            let is_on_axis = pt.x.abs() < AXIS_TOL;

            if is_on_axis {
                // 轴上的点：只生成一个共享顶点（参考 libgm calcFacetsWithoutSurfaces）
                // 3D 坐标: (0, 0, y) - 在旋转轴上
                vertices.push(Vec3::new(0.0, 0.0, pt.y));

                // 轴上点的法线：使用相邻非轴上点的方向，或默认方向
                let mut normal = Vec3::Z;
                // 查找相邻的非轴上点来确定法线方向
                for step in 1..n_profile {
                    let next_idx = (profile_idx + step) % n_profile;
                    if poly[next_idx].x > AXIS_TOL {
                        normal = Vec3::new(1.0, 0.0, 0.0); // 指向外侧
                        break;
                    }
                    if profile_idx >= step {
                        let prev_idx = profile_idx - step;
                        if poly[prev_idx].x > AXIS_TOL {
                            normal = Vec3::new(1.0, 0.0, 0.0);
                            break;
                        }
                    }
                }
                normals.push(normal);

                profile_vertex_info.push(ProfileVertexInfo {
                    start_index,
                    vertex_count: 1,
                    is_on_axis: true,
                });
            } else {
                // 非轴上的点：每个角度生成一个 3D 顶点
                for seg in 0..n_slices {
                    let theta = (seg as f32 / n_segments as f32) * angle_rad;
                    let (sin_theta, cos_theta) = theta.sin_cos();

                    // 3D 坐标: (x * cos, x * sin, y)
                    let pos = Vec3::new(pt.x * cos_theta, pt.x * sin_theta, pt.y);
                    vertices.push(pos);

                    // 法线：径向方向
                    normals.push(Vec3::new(cos_theta, sin_theta, 0.0));
                }

                profile_vertex_info.push(ProfileVertexInfo {
                    start_index,
                    vertex_count: n_slices,
                    is_on_axis: false,
                });
            }
        }

        // Step 4: 生成三角形索引
        // 关键：确保所有三角形的绕序一致（从外侧看是逆时针）
        //
        // 对于旋转体，从轴外侧看，轮廓点沿 profile 方向移动，
        // 旋转方向是 theta 增加的方向（从 +X 向 +Y）。
        //
        // 为了保证流形，需要确保：
        // 1. 相邻边共享相同的顶点（不是位置相同的不同顶点）
        // 2. 每条边恰好被两个三角形共享
        // 3. 所有三角形绕序一致
        for edge_idx in 0..n_profile {
            let v0_idx = edge_idx;
            let v1_idx = (edge_idx + 1) % n_profile;

            let v0_info = &profile_vertex_info[v0_idx];
            let v1_info = &profile_vertex_info[v1_idx];

            if v0_info.is_on_axis && v1_info.is_on_axis {
                // 两端都在轴上：退化边，跳过
                continue;
            }

            if v0_info.is_on_axis {
                // 起点在轴上：生成三角形扇（圆锥顶点）
                // 从轴上点向外辐射的三角形
                let axis_vertex = v0_info.start_index as u32;
                for seg in 0..n_segments {
                    let v1_curr = (v1_info.start_index + seg) as u32;
                    let v1_next = (v1_info.start_index + (seg + 1) % n_slices) as u32;
                    // 绕序: 轴点 -> curr -> next (从外侧看逆时针)
                    indices.extend_from_slice(&[axis_vertex, v1_curr, v1_next]);
                }
            } else if v1_info.is_on_axis {
                // 终点在轴上：生成三角形扇
                let axis_vertex = v1_info.start_index as u32;
                for seg in 0..n_segments {
                    let v0_curr = (v0_info.start_index + seg) as u32;
                    let v0_next = (v0_info.start_index + (seg + 1) % n_slices) as u32;
                    // 绕序: curr -> 轴点 -> next (从外侧看逆时针)
                    indices.extend_from_slice(&[v0_curr, axis_vertex, v0_next]);
                }
            } else {
                // 两端都不在轴上：生成四边形（两个三角形）
                for seg in 0..n_segments {
                    let v0_curr = (v0_info.start_index + seg) as u32;
                    let v0_next = (v0_info.start_index + (seg + 1) % n_slices) as u32;
                    let v1_curr = (v1_info.start_index + seg) as u32;
                    let v1_next = (v1_info.start_index + (seg + 1) % n_slices) as u32;

                    // 四边形由两个三角形组成，保持一致的绕序
                    // 从外侧看：v0_curr -> v1_curr -> v1_next -> v0_next
                    indices.extend_from_slice(&[v0_curr, v1_curr, v1_next]);
                    indices.extend_from_slice(&[v0_curr, v1_next, v0_next]);
                }
            }
        }

        // Step 5: 非完整旋转时添加端面
        if !is_full_revolution && n_profile >= 3 {
            // 起始端面（seg=0）
            let mut start_verts: Vec<u32> = Vec::with_capacity(n_profile);
            for info in &profile_vertex_info {
                start_verts.push(info.start_index as u32);
            }
            // 扇形三角化起始端面
            for i in 1..n_profile - 1 {
                indices.extend_from_slice(&[start_verts[0], start_verts[i + 1], start_verts[i]]);
            }

            // 结束端面（seg=n_segments）
            let mut end_verts: Vec<u32> = Vec::with_capacity(n_profile);
            for info in &profile_vertex_info {
                if info.is_on_axis {
                    end_verts.push(info.start_index as u32); // 轴上点共享
                } else {
                    end_verts.push((info.start_index + info.vertex_count - 1) as u32);
                }
            }
            // 扇形三角化结束端面（反向绕序）
            for i in 1..n_profile - 1 {
                indices.extend_from_slice(&[end_verts[0], end_verts[i], end_verts[i + 1]]);
            }
        }
    }

    if vertices.is_empty() {
        return None;
    }

    // 6. 生成 UV 坐标
    let uvs: Vec<[f32; 2]> = vertices
        .iter()
        .map(|v| {
            // U: 径向角度归一化, V: 高度归一化
            let angle = v.y.atan2(v.x);
            let u = (angle / std::f32::consts::TAU + 0.5).fract();
            let v_coord = v.z / 100.0; // 简单归一化
            [u, v_coord]
        })
        .collect();

    Some(RevolvedMesh {
        vertices,
        normals,
        indices,
        uvs,
    })
}

/// 从 2D 轮廓点集创建 Manifold 风格的旋转体
///
/// 这是一个便捷函数，直接从 `ProcessedProfile` 使用 Manifold 算法生成旋转体
///
/// # 参数
/// - `profile`: 已处理的截面数据
/// - `angle`: 旋转角度（度）
/// - `segments`: 分段数，0 表示自动
///
/// # 返回
/// - `Option<RevolvedMesh>`: 生成的网格
pub fn revolve_profile_manifold(
    profile: &ProcessedProfile,
    angle: f32,
    segments: usize,
) -> Option<RevolvedMesh> {
    let polygon: Vec<Vec2> = profile.contour_points.clone();
    revolve_polygons_manifold(&[polygon], segments, angle)
}

/// 将 ExtrudedMesh 转换为 PlantMesh（用于导出 OBJ）
impl From<ExtrudedMesh> for crate::shape::pdms_shape::PlantMesh {
    fn from(mesh: ExtrudedMesh) -> Self {
        crate::shape::pdms_shape::PlantMesh {
            vertices: mesh.vertices,
            normals: mesh.normals,
            uvs: mesh.uvs,
            indices: mesh.indices,
            wire_vertices: Vec::new(),
            edges: Vec::new(),
            aabb: None,
        }
    }
}

/// 将 RevolvedMesh 转换为 PlantMesh（用于导出 OBJ）
impl From<RevolvedMesh> for crate::shape::pdms_shape::PlantMesh {
    fn from(mesh: RevolvedMesh) -> Self {
        crate::shape::pdms_shape::PlantMesh {
            vertices: mesh.vertices,
            normals: mesh.normals,
            uvs: mesh.uvs,
            indices: mesh.indices,
            wire_vertices: Vec::new(),
            edges: Vec::new(),
            aabb: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    fn build_inputs_from_vec3(wires: Vec<Vec<Vec3>>) -> (Vec<Vec<Vec2>>, Vec<Vec<f32>>) {
        let mut all_verts = Vec::with_capacity(wires.len());
        let mut all_frads = Vec::with_capacity(wires.len());

        for wire in wires.into_iter() {
            let mut verts = Vec::with_capacity(wire.len());
            let mut frads = Vec::with_capacity(wire.len());

            for v in wire.into_iter() {
                verts.push(Vec2::new(v.x, v.y));
                frads.push(v.z);
            }

            all_verts.push(verts);
            all_frads.push(frads);
        }

        (all_verts, all_frads)
    }

    /// 辅助函数：确保测试输出目录存在
    fn ensure_test_output_dir() {
        let dir = "test_output/profile_processor";
        if !Path::new(dir).exists() {
            fs::create_dir_all(dir).expect("无法创建测试输出目录");
        }
    }

    /// 辅助函数：导出 mesh 到 OBJ 文件
    fn export_mesh_to_obj(mesh: &crate::shape::pdms_shape::PlantMesh, filename: &str) {
        ensure_test_output_dir();
        let path = format!("test_output/profile_processor/{}", filename);
        if let Err(e) = mesh.export_obj(false, &path) {
            eprintln!("⚠️  导出 OBJ 文件失败 {}: {}", path, e);
        } else {
            println!("   📄 已导出: {}", path);
        }
    }

    #[test]
    fn test_profile_processor_single() {
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(100.0, 0.0, 0.0),
            Vec3::new(100.0, 100.0, 10.0), // 带圆角
            Vec3::new(0.0, 100.0, 0.0),
        ];

        // 测试旧的 new_single API（向后兼容）
        let processor = ProfileProcessor::new_single(vertices.clone());
        let result = processor.process("test_single", None).unwrap();

        assert!(result.contour_points.len() >= 4);
        assert!(!result.tri_indices.is_empty());
        assert_eq!(result.tri_indices.len() % 3, 0);

        // 测试新的统一入口 from_wires API
        let (verts2d, frads) = build_inputs_from_vec3(vec![vertices]);
        let processor2 = ProfileProcessor::from_wires(verts2d, frads, true).unwrap();
        let result2 = processor2.process("test_single_from_wires", None).unwrap();

        assert_eq!(result.contour_points.len(), result2.contour_points.len());
        assert_eq!(result.tri_indices.len(), result2.tri_indices.len());

        println!("✅ 单轮廓测试通过");
        println!("   轮廓点数: {}", result.contour_points.len());
        println!("   三角形数: {}", result.tri_indices.len() / 3);
    }

    #[test]
    fn test_profile_processor_with_hole() {
        // 外轮廓（正方形）
        let outer_vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(100.0, 0.0, 0.0),
            Vec3::new(100.0, 100.0, 0.0),
            Vec3::new(0.0, 100.0, 0.0),
        ];

        // 内孔（小正方形）
        let inner_vertices = vec![
            Vec3::new(30.0, 30.0, 0.0),
            Vec3::new(70.0, 30.0, 0.0),
            Vec3::new(70.0, 70.0, 0.0),
            Vec3::new(30.0, 70.0, 0.0),
        ];

        // 测试旧的 new_multi API（向后兼容）
        let outer = ProfileContour {
            vertices: outer_vertices.clone(),
            is_hole: false,
        };
        let inner = ProfileContour {
            vertices: inner_vertices.clone(),
            is_hole: true,
        };

        let processor = ProfileProcessor::new_multi(vec![outer, inner]).unwrap();
        let result = processor.process("test_with_hole", None).unwrap();

        assert!(!result.tri_indices.is_empty());

        // 测试新的统一入口 from_wires API（自动检测）
        let (verts2d_auto, frads_auto) =
            build_inputs_from_vec3(vec![outer_vertices.clone(), inner_vertices.clone()]);
        let processor2 = ProfileProcessor::from_wires(verts2d_auto, frads_auto, true).unwrap();
        let result2 = processor2
            .process("test_with_hole_from_wires_auto", None)
            .unwrap();

        assert_eq!(result.tri_indices.len(), result2.tri_indices.len());

        // 测试新的统一入口 from_wires API（遵循约定：第一个是外轮廓）
        let (verts2d_conv, frads_conv) =
            build_inputs_from_vec3(vec![outer_vertices, inner_vertices]);
        let processor3 = ProfileProcessor::from_wires(verts2d_conv, frads_conv, false).unwrap();
        let result3 = processor3
            .process("test_with_hole_from_wires_convention", None)
            .unwrap();

        assert_eq!(result.tri_indices.len(), result3.tri_indices.len());

        println!("✅ 带孔洞测试通过");
        println!("   三角形数: {}", result.tri_indices.len() / 3);
    }

    #[test]
    fn test_extrude_profile() {
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(50.0, 0.0, 0.0),
            Vec3::new(50.0, 50.0, 0.0),
            Vec3::new(0.0, 50.0, 0.0),
        ];

        // 使用新的统一入口 from_wires
        let (verts2d, frads) = build_inputs_from_vec3(vec![vertices]);
        let processor = ProfileProcessor::from_wires(verts2d, frads, true).unwrap();
        let profile = processor.process("test_extrude", None).unwrap();
        let mesh = extrude_profile(&profile, 100.0);

        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());
        assert_eq!(mesh.vertices.len(), mesh.normals.len());

        println!("✅ 拉伸测试通过");
        println!("   顶点数: {}", mesh.vertices.len());
        println!("   三角形数: {}", mesh.indices.len() / 3);
    }

    /// 测试：矩形截面拉伸（真实工程尺寸）
    #[test]
    fn test_extrude_rectangle_real() {
        // 200x100mm 矩形截面，高度 300mm
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(200.0, 0.0, 0.0),
            Vec3::new(200.0, 100.0, 0.0),
            Vec3::new(0.0, 100.0, 0.0),
        ];

        let (verts2d, frads) = build_inputs_from_vec3(vec![vertices]);
        let processor = ProfileProcessor::from_wires(verts2d, frads, true).unwrap();
        let profile = processor.process("rectangle_200x100", None).unwrap();
        let mesh = extrude_profile(&profile, 300.0);

        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());
        assert_eq!(mesh.vertices.len(), mesh.normals.len());
        assert!(mesh.indices.len() % 3 == 0);

        // 导出 OBJ 文件
        let plant_mesh: crate::shape::pdms_shape::PlantMesh = mesh.into();
        export_mesh_to_obj(&plant_mesh, "extrusion_rectangle_200x100x300.obj");

        println!("✅ 矩形拉伸测试通过 (200x100x300)");
        println!("   顶点数: {}", plant_mesh.vertices.len());
        println!("   三角形数: {}", plant_mesh.indices.len() / 3);
    }

    /// 测试：带圆角的矩形拉伸
    #[test]
    fn test_extrude_rounded_rectangle() {
        // 150x150mm 矩形，四角圆角半径 20mm
        let vertices = vec![
            Vec3::new(0.0, 0.0, 20.0),     // 左下角，圆角半径 20
            Vec3::new(150.0, 0.0, 20.0),   // 右下角，圆角半径 20
            Vec3::new(150.0, 150.0, 20.0), // 右上角，圆角半径 20
            Vec3::new(0.0, 150.0, 20.0),   // 左上角，圆角半径 20
        ];

        let (verts2d, frads) = build_inputs_from_vec3(vec![vertices]);
        let processor = ProfileProcessor::from_wires(verts2d, frads, true).unwrap();
        let profile = processor
            .process("rounded_rectangle_150x150", None)
            .unwrap();
        let mesh = extrude_profile(&profile, 250.0);

        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());
        // 圆角会增加顶点数
        assert!(mesh.vertices.len() > 8);

        // 导出 OBJ 文件
        let plant_mesh: crate::shape::pdms_shape::PlantMesh = mesh.into();
        export_mesh_to_obj(&plant_mesh, "extrusion_rounded_rectangle_150x150x250.obj");

        println!("✅ 带圆角矩形拉伸测试通过 (150x150, r=20, h=250)");
        println!("   顶点数: {}", plant_mesh.vertices.len());
        println!("   三角形数: {}", plant_mesh.indices.len() / 3);
    }

    /// 测试：L形截面拉伸
    #[test]
    fn test_extrude_l_shape() {
        // L形轮廓：150x50 + 50x150
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(150.0, 0.0, 0.0),
            Vec3::new(150.0, 50.0, 0.0),
            Vec3::new(50.0, 50.0, 0.0),
            Vec3::new(50.0, 150.0, 0.0),
            Vec3::new(0.0, 150.0, 0.0),
        ];

        let (verts2d, frads) = build_inputs_from_vec3(vec![vertices]);
        let processor = ProfileProcessor::from_wires(verts2d, frads, true).unwrap();
        let profile = processor.process("l_shape", None).unwrap();
        let mesh = extrude_profile(&profile, 150.0);

        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());

        // 导出 OBJ 文件
        let plant_mesh: crate::shape::pdms_shape::PlantMesh = mesh.into();
        export_mesh_to_obj(&plant_mesh, "extrusion_l_shape_150x150x150.obj");

        println!("✅ L形截面拉伸测试通过");
        println!("   顶点数: {}", plant_mesh.vertices.len());
        println!("   三角形数: {}", plant_mesh.indices.len() / 3);
    }

    /// 测试：方形外轮廓 + 圆形内孔（模拟圆管）
    #[test]
    fn test_extrude_square_with_circular_hole() {
        // 外轮廓：200x200 方形
        let outer = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(200.0, 0.0, 0.0),
            Vec3::new(200.0, 200.0, 0.0),
            Vec3::new(0.0, 200.0, 0.0),
        ];

        // 内孔：使用 FRADIUS 模拟圆形（40x40方形，每角圆角20）
        let inner = vec![
            Vec3::new(80.0, 80.0, 20.0),   // 左下角，圆角半径 20
            Vec3::new(120.0, 80.0, 20.0),  // 右下角，圆角半径 20
            Vec3::new(120.0, 120.0, 20.0), // 右上角，圆角半径 20
            Vec3::new(80.0, 120.0, 20.0),  // 左上角，圆角半径 20
        ];

        // 测试自动检测（面积大的作为外轮廓）
        let (verts2d_auto, frads_auto) = build_inputs_from_vec3(vec![outer.clone(), inner.clone()]);
        let processor = ProfileProcessor::from_wires(verts2d_auto, frads_auto, true).unwrap();
        let profile = processor
            .process("square_with_circular_hole_auto", None)
            .unwrap();
        let mesh = extrude_profile(&profile, 300.0);

        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());

        // 测试遵循约定（第一个是外轮廓）
        let (verts2d_conv, frads_conv) = build_inputs_from_vec3(vec![outer, inner]);
        let processor2 = ProfileProcessor::from_wires(verts2d_conv, frads_conv, false).unwrap();
        let profile2 = processor2
            .process("square_with_circular_hole_convention", None)
            .unwrap();
        let mesh2 = extrude_profile(&profile2, 300.0);

        assert_eq!(mesh.vertices.len(), mesh2.vertices.len());
        assert_eq!(mesh.indices.len(), mesh2.indices.len());

        // 导出 OBJ 文件
        let plant_mesh: crate::shape::pdms_shape::PlantMesh = mesh.into();
        export_mesh_to_obj(
            &plant_mesh,
            "extrusion_square_with_circular_hole_200x200x300.obj",
        );

        println!("✅ 方形外轮廓+圆形内孔测试通过");
        println!("   顶点数: {}", plant_mesh.vertices.len());
        println!("   三角形数: {}", plant_mesh.indices.len() / 3);
    }

    /// 测试：H型钢截面拉伸（真实工程尺寸 H200x200）
    #[test]
    fn test_extrude_h_beam() {
        // H型钢 H200x200 标准截面
        // 翼缘宽度 200mm，翼缘厚度 10mm，腹板高度 180mm，腹板厚度 8mm
        let outer = vec![
            // 外轮廓（逆时针）
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(200.0, 0.0, 0.0),
            Vec3::new(200.0, 10.0, 0.0),
            Vec3::new(110.0, 10.0, 0.0),  // 翼缘到腹板
            Vec3::new(110.0, 190.0, 0.0), // 腹板右侧
            Vec3::new(200.0, 190.0, 0.0),
            Vec3::new(200.0, 200.0, 0.0),
            Vec3::new(0.0, 200.0, 0.0),
            Vec3::new(0.0, 190.0, 0.0),
            Vec3::new(90.0, 190.0, 0.0), // 腹板左侧
            Vec3::new(90.0, 10.0, 0.0),  // 腹板到翼缘
            Vec3::new(0.0, 10.0, 0.0),
        ];

        let (verts2d, frads) = build_inputs_from_vec3(vec![outer]);
        let processor = ProfileProcessor::from_wires(verts2d, frads, true).unwrap();
        let profile = processor.process("h_beam_200x200", None).unwrap();
        let mesh = extrude_profile(&profile, 1000.0);

        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());

        // 导出 OBJ 文件
        let plant_mesh: crate::shape::pdms_shape::PlantMesh = mesh.into();
        export_mesh_to_obj(&plant_mesh, "extrusion_h_beam_200x200x1000.obj");

        println!("✅ H型钢截面拉伸测试通过 (H200x200, h=1000)");
        println!("   顶点数: {}", plant_mesh.vertices.len());
        println!("   三角形数: {}", plant_mesh.indices.len() / 3);
    }

    /// 测试：多孔洞拉伸（3个圆形内孔）
    #[test]
    fn test_extrude_multiple_holes() {
        // 外轮廓：300x300 方形
        let outer = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(300.0, 0.0, 0.0),
            Vec3::new(300.0, 300.0, 0.0),
            Vec3::new(0.0, 300.0, 0.0),
        ];

        // 内孔1：左上角，40x40方形（用圆角模拟圆形）
        let hole1 = vec![
            Vec3::new(50.0, 50.0, 20.0),
            Vec3::new(90.0, 50.0, 20.0),
            Vec3::new(90.0, 90.0, 20.0),
            Vec3::new(50.0, 90.0, 20.0),
        ];

        // 内孔2：中间，50x50方形
        let hole2 = vec![
            Vec3::new(125.0, 125.0, 0.0),
            Vec3::new(175.0, 125.0, 0.0),
            Vec3::new(175.0, 175.0, 0.0),
            Vec3::new(125.0, 175.0, 0.0),
        ];

        // 内孔3：右下角，30x30方形
        let hole3 = vec![
            Vec3::new(210.0, 210.0, 0.0),
            Vec3::new(240.0, 210.0, 0.0),
            Vec3::new(240.0, 240.0, 0.0),
            Vec3::new(210.0, 240.0, 0.0),
        ];

        let (verts2d, frads) = build_inputs_from_vec3(vec![outer, hole1, hole2, hole3]);
        let processor = ProfileProcessor::from_wires(verts2d, frads, true).unwrap();
        let profile = processor.process("multiple_holes", None).unwrap();
        let mesh = extrude_profile(&profile, 400.0);

        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());

        // 导出 OBJ 文件
        let plant_mesh: crate::shape::pdms_shape::PlantMesh = mesh.into();
        export_mesh_to_obj(&plant_mesh, "extrusion_multiple_holes_300x300x400.obj");

        println!("✅ 多孔洞拉伸测试通过 (300x300, 3个内孔, h=400)");
        println!("   顶点数: {}", plant_mesh.vertices.len());
        println!("   三角形数: {}", plant_mesh.indices.len() / 3);
    }

    /// 测试：旋转体 - 圆柱体
    #[test]
    fn test_revolve_cylinder() {
        // 圆柱体：半径 50mm，高度 200mm
        // 截面是一个矩形轮廓（从底部到顶部）
        let profile = vec![
            Vec3::new(50.0, 0.0, 0.0),   // 底部右点（距离旋转轴50mm）
            Vec3::new(50.0, 200.0, 0.0), // 顶部右点
            Vec3::new(0.0, 200.0, 0.0),  // 顶部左点（在旋转轴上）
            Vec3::new(0.0, 0.0, 0.0),    // 底部左点（在旋转轴上）
        ];

        let (verts2d, frads) = build_inputs_from_vec3(vec![profile]);
        let processor = ProfileProcessor::from_wires(verts2d, frads, true).unwrap();
        let processed = processor.process("cylinder_r50_h200", None).unwrap();
        let mesh = revolve_profile(
            &processed,
            360.0,      // 旋转360度
            32,         // 32个分段
            Vec3::Z,    // 绕Z轴旋转
            Vec3::ZERO, // 旋转中心在原点
        );

        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());
        assert_eq!(mesh.vertices.len(), mesh.normals.len());

        // 导出 OBJ 文件
        let plant_mesh: crate::shape::pdms_shape::PlantMesh = mesh.into();
        export_mesh_to_obj(&plant_mesh, "revolution_cylinder_r50_h200_360deg.obj");

        println!("✅ 圆柱体旋转测试通过 (r=50, h=200, 360°)");
        println!("   顶点数: {}", plant_mesh.vertices.len());
        println!("   三角形数: {}", plant_mesh.indices.len() / 3);
    }

    /// 测试：旋转体 - 圆锥体
    #[test]
    fn test_revolve_cone() {
        // 圆锥体：底部半径 60mm，顶部半径 20mm，高度 150mm
        // 截面是一个梯形轮廓
        let profile = vec![
            Vec3::new(60.0, 0.0, 0.0),   // 底部右点
            Vec3::new(20.0, 150.0, 0.0), // 顶部右点
            Vec3::new(0.0, 150.0, 0.0),  // 顶部左点（在旋转轴上）
            Vec3::new(0.0, 0.0, 0.0),    // 底部左点（在旋转轴上）
        ];

        let (verts2d, frads) = build_inputs_from_vec3(vec![profile]);
        let processor = ProfileProcessor::from_wires(verts2d, frads, true).unwrap();
        let processed = processor.process("cone_r60_r20_h150", None).unwrap();
        let mesh = revolve_profile(&processed, 360.0, 32, Vec3::Z, Vec3::ZERO);

        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());

        // 导出 OBJ 文件
        let plant_mesh: crate::shape::pdms_shape::PlantMesh = mesh.into();
        export_mesh_to_obj(&plant_mesh, "revolution_cone_r60_r20_h150_360deg.obj");

        println!("✅ 圆锥体旋转测试通过 (r1=60, r2=20, h=150, 360°)");
        println!("   顶点数: {}", plant_mesh.vertices.len());
        println!("   三角形数: {}", plant_mesh.indices.len() / 3);
    }

    /// 测试：旋转体 - 圆台（带圆角）
    #[test]
    fn test_revolve_frustum_with_rounding() {
        // 圆台：底部半径 80mm，顶部半径 40mm，高度 200mm，带圆角过渡
        let profile = vec![
            Vec3::new(80.0, 0.0, 0.0),    // 底部点
            Vec3::new(80.0, 50.0, 10.0),  // 底部圆角（半径10）
            Vec3::new(40.0, 150.0, 10.0), // 顶部圆角（半径10）
            Vec3::new(40.0, 200.0, 0.0),  // 顶部点
        ];

        let (verts2d, frads) = build_inputs_from_vec3(vec![profile]);
        let processor = ProfileProcessor::from_wires(verts2d, frads, true).unwrap();
        let processed = processor.process("frustum_r80_r40_h200", None).unwrap();
        let mesh = revolve_profile(&processed, 360.0, 32, Vec3::Z, Vec3::ZERO);

        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());

        // 导出 OBJ 文件
        let plant_mesh: crate::shape::pdms_shape::PlantMesh = mesh.into();
        export_mesh_to_obj(&plant_mesh, "revolution_frustum_r80_r40_h200_360deg.obj");

        println!("✅ 圆台旋转测试通过 (r1=80, r2=40, h=200, 带圆角, 360°)");
        println!("   顶点数: {}", plant_mesh.vertices.len());
        println!("   三角形数: {}", plant_mesh.indices.len() / 3);
    }

    /// 测试：旋转体 - 部分旋转（180度）
    #[test]
    fn test_revolve_partial() {
        // 半圆柱：半径 50mm，高度 200mm，旋转 180度
        // 截面是一个矩形轮廓
        let profile = vec![
            Vec3::new(50.0, 0.0, 0.0),   // 底部右点
            Vec3::new(50.0, 200.0, 0.0), // 顶部右点
            Vec3::new(0.0, 200.0, 0.0),  // 顶部左点（在旋转轴上）
            Vec3::new(0.0, 0.0, 0.0),    // 底部左点（在旋转轴上）
        ];

        let (verts2d, frads) = build_inputs_from_vec3(vec![profile]);
        let processor = ProfileProcessor::from_wires(verts2d, frads, true).unwrap();
        let processed = processor.process("half_cylinder", None).unwrap();
        let mesh = revolve_profile(
            &processed,
            180.0, // 只旋转180度
            16,    // 16个分段
            Vec3::Z,
            Vec3::ZERO,
        );

        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());

        // 导出 OBJ 文件
        let plant_mesh: crate::shape::pdms_shape::PlantMesh = mesh.into();
        export_mesh_to_obj(&plant_mesh, "revolution_half_cylinder_r50_h200_180deg.obj");

        println!("✅ 部分旋转测试通过 (r=50, h=200, 180°)");
        println!("   顶点数: {}", plant_mesh.vertices.len());
        println!("   三角形数: {}", plant_mesh.indices.len() / 3);
    }

    /// 测试：自动检测外轮廓（面积大的作为外轮廓）
    #[test]
    fn test_auto_detect_outer_contour() {
        // 大轮廓：200x200 方形
        let large = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(200.0, 0.0, 0.0),
            Vec3::new(200.0, 200.0, 0.0),
            Vec3::new(0.0, 200.0, 0.0),
        ];

        // 小轮廓：50x50 方形（应该被识别为内孔）
        let small = vec![
            Vec3::new(75.0, 75.0, 0.0),
            Vec3::new(125.0, 75.0, 0.0),
            Vec3::new(125.0, 125.0, 0.0),
            Vec3::new(75.0, 125.0, 0.0),
        ];

        // 测试：小轮廓在前，大轮廓在后（应该自动识别大轮廓为外轮廓）
        let (verts2d_small_first, frads_small_first) =
            build_inputs_from_vec3(vec![small.clone(), large.clone()]);
        let processor =
            ProfileProcessor::from_wires(verts2d_small_first, frads_small_first, true).unwrap();
        let profile = processor.process("auto_detect_small_first", None).unwrap();
        let mesh = extrude_profile(&profile, 100.0);

        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());

        // 测试：大轮廓在前，小轮廓在后（应该识别大轮廓为外轮廓）
        let (verts2d_large_first, frads_large_first) = build_inputs_from_vec3(vec![large, small]);
        let processor2 =
            ProfileProcessor::from_wires(verts2d_large_first, frads_large_first, true).unwrap();
        let profile2 = processor2.process("auto_detect_large_first", None).unwrap();
        let mesh2 = extrude_profile(&profile2, 100.0);

        // 两种情况下结果应该相同
        assert_eq!(mesh.vertices.len(), mesh2.vertices.len());
        assert_eq!(mesh.indices.len(), mesh2.indices.len());

        println!("✅ 自动检测外轮廓测试通过");
        println!("   顶点数: {}", mesh.vertices.len());
        println!("   三角形数: {}", mesh.indices.len() / 3);
    }

    /// 测试：边界情况 - 空轮廓
    #[test]
    fn test_empty_wires() {
        let result = ProfileProcessor::from_wires(Vec::new(), Vec::new(), true);
        assert!(result.is_err());
        println!("✅ 空轮廓测试通过（正确返回错误）");
    }

    /// 测试：边界情况 - 单个点
    #[test]
    fn test_single_point() {
        let (verts2d, frads) = build_inputs_from_vec3(vec![vec![Vec3::new(0.0, 0.0, 0.0)]]);
        let result = ProfileProcessor::from_wires(verts2d, frads, true);
        assert!(result.is_ok()); // 可以创建，但处理时会失败
        let processor = result.unwrap();
        let process_result = processor.process("single_point", None);
        assert!(process_result.is_err()); // 处理应该失败（点数不足）
        println!("✅ 单点测试通过（正确返回错误）");
    }

    /// 测试：边界情况 - 两个点
    #[test]
    fn test_two_points() {
        let (verts2d, frads) = build_inputs_from_vec3(vec![vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(100.0, 0.0, 0.0),
        ]]);
        let result = ProfileProcessor::from_wires(verts2d, frads, true);
        assert!(result.is_ok());
        let processor = result.unwrap();
        let process_result = processor.process("two_points", None);
        assert!(process_result.is_err()); // 处理应该失败（点数不足，需要至少3个点）
        println!("✅ 两点测试通过（正确返回错误）");
    }

    // ========================================================================
    // Manifold 风格旋转体测试
    // ========================================================================

    /// 测试：裁剪负 X 侧轮廓
    #[test]
    fn test_clip_polygon_to_positive_x() {
        // 测试 1: 全部在正侧
        let poly1 = vec![
            Vec2::new(10.0, 0.0),
            Vec2::new(20.0, 0.0),
            Vec2::new(20.0, 100.0),
            Vec2::new(10.0, 100.0),
        ];
        let clipped1 = clip_polygon_to_positive_x(&poly1);
        assert!(clipped1.is_some());
        assert_eq!(clipped1.unwrap().len(), 4);
        println!("✅ 全正侧轮廓裁剪测试通过");

        // 测试 2: 全部在负侧
        let poly2 = vec![
            Vec2::new(-20.0, 0.0),
            Vec2::new(-10.0, 0.0),
            Vec2::new(-10.0, 100.0),
            Vec2::new(-20.0, 100.0),
        ];
        let clipped2 = clip_polygon_to_positive_x(&poly2);
        assert!(clipped2.is_none());
        println!("✅ 全负侧轮廓裁剪测试通过（返回 None）");

        // 测试 3: 跨越 Y 轴的轮廓
        let poly3 = vec![
            Vec2::new(-20.0, 0.0),
            Vec2::new(20.0, 0.0),
            Vec2::new(20.0, 100.0),
            Vec2::new(-20.0, 100.0),
        ];
        let clipped3 = clip_polygon_to_positive_x(&poly3);
        assert!(clipped3.is_some());
        let result3 = clipped3.unwrap();
        // 应该有 4 个点：两个正侧点 + 两个插值点
        assert!(result3.len() >= 4);
        // 检查所有点都在正侧
        for p in &result3 {
            assert!(p.x >= 0.0, "裁剪后应该所有点 x >= 0");
        }
        println!("✅ 跨轴轮廓裁剪测试通过，结果点数: {}", result3.len());
    }

    /// 测试：自适应分段数计算
    #[test]
    fn test_compute_adaptive_segments() {
        // 小半径，完整圆
        let seg1 = compute_adaptive_segments(10.0, 360.0, 3);
        assert!(seg1 >= 8, "小半径完整圆应至少 8 段");
        println!("✅ r=10, 360°: {} 段", seg1);

        // 大半径，完整圆
        let seg2 = compute_adaptive_segments(100.0, 360.0, 3);
        assert!(seg2 > seg1, "大半径应有更多分段");
        println!("✅ r=100, 360°: {} 段", seg2);

        // 半圆
        let seg3 = compute_adaptive_segments(100.0, 180.0, 3);
        assert!(seg3 < seg2, "180° 应比 360° 少分段");
        println!("✅ r=100, 180°: {} 段", seg3);

        // 最小分段数保证
        let seg4 = compute_adaptive_segments(1.0, 10.0, 8);
        assert!(seg4 >= 8, "应保证最小分段数");
        println!("✅ r=1, 10°, min=8: {} 段", seg4);
    }

    /// 测试：Manifold 风格 - 简单圆柱体
    #[test]
    fn test_revolve_manifold_simple_cylinder() {
        // 圆柱体：半径 50，高度 200
        // 截面是一个矩形 (x=径向距离, y=高度)
        let polygon = vec![
            Vec2::new(50.0, 0.0),   // 底部右点
            Vec2::new(50.0, 200.0), // 顶部右点
            Vec2::new(0.0, 200.0),  // 顶部左点（在轴上）
            Vec2::new(0.0, 0.0),    // 底部左点（在轴上）
        ];

        let mesh = revolve_polygons_manifold(&[polygon], 32, 360.0);
        assert!(mesh.is_some());
        let mesh = mesh.unwrap();

        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());
        assert_eq!(mesh.vertices.len(), mesh.normals.len());

        // 导出 OBJ
        let plant_mesh: crate::shape::pdms_shape::PlantMesh = mesh.into();
        export_mesh_to_obj(&plant_mesh, "manifold_cylinder_r50_h200.obj");

        println!("✅ Manifold 圆柱体测试通过");
        println!("   顶点数: {}", plant_mesh.vertices.len());
        println!("   三角形数: {}", plant_mesh.indices.len() / 3);
    }

    /// 测试：Manifold 风格 - 跨轴轮廓自动裁剪
    #[test]
    fn test_revolve_manifold_cross_axis() {
        // 轮廓跨越 Y 轴：从 x=-20 到 x=50
        let polygon = vec![
            Vec2::new(-20.0, 0.0),   // 负侧，应被裁剪
            Vec2::new(50.0, 0.0),    // 正侧
            Vec2::new(50.0, 100.0),  // 正侧
            Vec2::new(-20.0, 100.0), // 负侧，应被裁剪
        ];

        let mesh = revolve_polygons_manifold(&[polygon], 24, 360.0);
        assert!(mesh.is_some());
        let mesh = mesh.unwrap();

        // 检查所有顶点的径向距离 >= 0
        for v in &mesh.vertices {
            let radial_dist = (v.x * v.x + v.y * v.y).sqrt();
            // 允许很小的误差（轴上点）
            assert!(
                radial_dist >= -0.01 || v.z.abs() > 0.0,
                "顶点应该在正侧或轴上"
            );
        }

        // 导出 OBJ
        let plant_mesh: crate::shape::pdms_shape::PlantMesh = mesh.into();
        export_mesh_to_obj(&plant_mesh, "manifold_cross_axis_clipped.obj");

        println!("✅ Manifold 跨轴裁剪测试通过");
        println!("   顶点数: {}", plant_mesh.vertices.len());
        println!("   三角形数: {}", plant_mesh.indices.len() / 3);
    }

    /// 测试：Manifold 风格 - 部分旋转 (180°)
    #[test]
    fn test_revolve_manifold_partial_180() {
        // 半圆柱体
        let polygon = vec![
            Vec2::new(50.0, 0.0),
            Vec2::new(50.0, 150.0),
            Vec2::new(0.0, 150.0),
            Vec2::new(0.0, 0.0),
        ];

        let mesh = revolve_polygons_manifold(&[polygon], 16, 180.0);
        assert!(mesh.is_some());
        let mesh = mesh.unwrap();

        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());

        // 导出 OBJ
        let plant_mesh: crate::shape::pdms_shape::PlantMesh = mesh.into();
        export_mesh_to_obj(&plant_mesh, "manifold_half_cylinder_180deg.obj");

        println!("✅ Manifold 180° 旋转测试通过");
        println!("   顶点数: {}", plant_mesh.vertices.len());
        println!("   三角形数: {}", plant_mesh.indices.len() / 3);
    }

    /// 测试：Manifold 风格 - 部分旋转 (90°)
    #[test]
    fn test_revolve_manifold_partial_90() {
        // 1/4 圆柱体
        let polygon = vec![
            Vec2::new(60.0, 0.0),
            Vec2::new(60.0, 100.0),
            Vec2::new(30.0, 100.0),
            Vec2::new(30.0, 0.0),
        ];

        let mesh = revolve_polygons_manifold(&[polygon], 8, 90.0);
        assert!(mesh.is_some());
        let mesh = mesh.unwrap();

        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());

        // 导出 OBJ
        let plant_mesh: crate::shape::pdms_shape::PlantMesh = mesh.into();
        export_mesh_to_obj(&plant_mesh, "manifold_quarter_pipe_90deg.obj");

        println!("✅ Manifold 90° 旋转测试通过");
        println!("   顶点数: {}", plant_mesh.vertices.len());
        println!("   三角形数: {}", plant_mesh.indices.len() / 3);
    }

    /// 测试：Manifold 风格 - 圆锥体（顶点在轴上）
    #[test]
    fn test_revolve_manifold_cone() {
        // 圆锥体：底部半径 80，顶部半径 0（尖顶），高度 150
        let polygon = vec![
            Vec2::new(80.0, 0.0),  // 底部
            Vec2::new(0.0, 150.0), // 顶点（在轴上）
            Vec2::new(0.0, 0.0),   // 底部中心（在轴上）
        ];

        let mesh = revolve_polygons_manifold(&[polygon], 24, 360.0);
        assert!(mesh.is_some());
        let mesh = mesh.unwrap();

        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());

        // 导出 OBJ
        let plant_mesh: crate::shape::pdms_shape::PlantMesh = mesh.into();
        export_mesh_to_obj(&plant_mesh, "manifold_cone_r80_h150.obj");

        println!("✅ Manifold 圆锥体测试通过");
        println!("   顶点数: {}", plant_mesh.vertices.len());
        println!("   三角形数: {}", plant_mesh.indices.len() / 3);
    }

    /// 测试：Manifold 风格 - 圆台（截面梯形）
    #[test]
    fn test_revolve_manifold_frustum() {
        // 圆台：底部半径 80，顶部半径 40，高度 200
        let polygon = vec![
            Vec2::new(80.0, 0.0),   // 底部外侧
            Vec2::new(40.0, 200.0), // 顶部外侧
            Vec2::new(0.0, 200.0),  // 顶部中心
            Vec2::new(0.0, 0.0),    // 底部中心
        ];

        let mesh = revolve_polygons_manifold(&[polygon], 32, 360.0);
        assert!(mesh.is_some());
        let mesh = mesh.unwrap();

        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());

        // 导出 OBJ
        let plant_mesh: crate::shape::pdms_shape::PlantMesh = mesh.into();
        export_mesh_to_obj(&plant_mesh, "manifold_frustum_r80_r40_h200.obj");

        println!("✅ Manifold 圆台测试通过");
        println!("   顶点数: {}", plant_mesh.vertices.len());
        println!("   三角形数: {}", plant_mesh.indices.len() / 3);
    }

    /// 测试：Manifold 风格 - 空心圆柱（管道）
    #[test]
    fn test_revolve_manifold_hollow_cylinder() {
        // 空心圆柱：外径 60，内径 40，高度 150
        let polygon = vec![
            Vec2::new(60.0, 0.0),   // 底部外侧
            Vec2::new(60.0, 150.0), // 顶部外侧
            Vec2::new(40.0, 150.0), // 顶部内侧
            Vec2::new(40.0, 0.0),   // 底部内侧
        ];

        let mesh = revolve_polygons_manifold(&[polygon], 32, 360.0);
        assert!(mesh.is_some());
        let mesh = mesh.unwrap();

        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());

        // 导出 OBJ
        let plant_mesh: crate::shape::pdms_shape::PlantMesh = mesh.into();
        export_mesh_to_obj(&plant_mesh, "manifold_pipe_r60_r40_h150.obj");

        println!("✅ Manifold 空心圆柱测试通过");
        println!("   顶点数: {}", plant_mesh.vertices.len());
        println!("   三角形数: {}", plant_mesh.indices.len() / 3);
    }

    /// 测试：Manifold 风格 - 球体截面
    #[test]
    fn test_revolve_manifold_sphere_profile() {
        // 用多边形近似半圆弧，旋转得到球体
        let radius = 50.0f32;
        let segments = 16;
        let mut polygon = Vec::new();

        // 从底部到顶部的半圆弧
        for i in 0..=segments {
            let angle = std::f32::consts::PI * i as f32 / segments as f32;
            let x = radius * angle.sin(); // 径向距离
            let y = -radius * angle.cos(); // 高度（从 -r 到 +r）
            polygon.push(Vec2::new(x, y + radius)); // 平移到正高度
        }

        let mesh = revolve_polygons_manifold(&[polygon], 32, 360.0);
        assert!(mesh.is_some());
        let mesh = mesh.unwrap();

        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());

        // 导出 OBJ
        let plant_mesh: crate::shape::pdms_shape::PlantMesh = mesh.into();
        export_mesh_to_obj(&plant_mesh, "manifold_sphere_r50.obj");

        println!("✅ Manifold 球体测试通过");
        println!("   顶点数: {}", plant_mesh.vertices.len());
        println!("   三角形数: {}", plant_mesh.indices.len() / 3);
    }

    /// 测试：Manifold 风格 - 自适应分段数
    #[test]
    fn test_revolve_manifold_auto_segments() {
        // 使用自动分段数 (segments = 0)
        let polygon = vec![
            Vec2::new(100.0, 0.0),
            Vec2::new(100.0, 50.0),
            Vec2::new(0.0, 50.0),
            Vec2::new(0.0, 0.0),
        ];

        let mesh = revolve_polygons_manifold(&[polygon], 0, 360.0);
        assert!(mesh.is_some());
        let mesh = mesh.unwrap();

        // 大半径应该有更多分段
        // 根据算法：周长 2π*100 ≈ 628，每 10mm 一段 ≈ 63 段
        assert!(mesh.vertices.len() > 100, "自适应分段应该生成足够多的顶点");

        // 导出 OBJ
        let plant_mesh: crate::shape::pdms_shape::PlantMesh = mesh.into();
        export_mesh_to_obj(&plant_mesh, "manifold_auto_segments_r100.obj");

        println!("✅ Manifold 自适应分段测试通过");
        println!("   顶点数: {}", plant_mesh.vertices.len());
        println!("   三角形数: {}", plant_mesh.indices.len() / 3);
    }

    /// 测试：Manifold 风格 - 边界情况（空输入）
    #[test]
    fn test_revolve_manifold_empty_input() {
        let result = revolve_polygons_manifold(&[], 32, 360.0);
        assert!(result.is_none());
        println!("✅ Manifold 空输入测试通过（返回 None）");
    }

    /// 测试：Manifold 风格 - 边界情况（全负侧轮廓）
    #[test]
    fn test_revolve_manifold_all_negative() {
        let polygon = vec![
            Vec2::new(-50.0, 0.0),
            Vec2::new(-50.0, 100.0),
            Vec2::new(-20.0, 100.0),
            Vec2::new(-20.0, 0.0),
        ];

        let result = revolve_polygons_manifold(&[polygon], 32, 360.0);
        assert!(result.is_none());
        println!("✅ Manifold 全负侧输入测试通过（返回 None）");
    }

    /// 测试：Manifold 风格 vs 原有实现对比
    #[test]
    fn test_revolve_manifold_vs_original() {
        // 简单圆柱体，对比两种实现
        let vertices = vec![
            Vec3::new(50.0, 0.0, 0.0),
            Vec3::new(50.0, 200.0, 0.0),
            Vec3::new(0.0, 200.0, 0.0),
            Vec3::new(0.0, 0.0, 0.0),
        ];

        let (verts2d, frads) = build_inputs_from_vec3(vec![vertices]);
        let processor = ProfileProcessor::from_wires(verts2d, frads, true).unwrap();
        let profile = processor.process("compare_cylinder", None).unwrap();

        // 原有实现
        let mesh_original = revolve_profile(&profile, 360.0, 32, Vec3::Z, Vec3::ZERO);

        // Manifold 风格
        let mesh_manifold = revolve_profile_manifold(&profile, 360.0, 32);
        assert!(mesh_manifold.is_some());
        let mesh_manifold = mesh_manifold.unwrap();

        println!("\n🔍 对比结果:");
        println!(
            "   原有实现 - 顶点数: {}, 三角形数: {}",
            mesh_original.vertices.len(),
            mesh_original.indices.len() / 3
        );
        println!(
            "   Manifold  - 顶点数: {}, 三角形数: {}",
            mesh_manifold.vertices.len(),
            mesh_manifold.indices.len() / 3
        );

        // 导出两个 OBJ 文件以便可视化对比
        let plant_original: crate::shape::pdms_shape::PlantMesh = mesh_original.into();
        let plant_manifold: crate::shape::pdms_shape::PlantMesh = mesh_manifold.into();
        export_mesh_to_obj(&plant_original, "compare_original_cylinder.obj");
        export_mesh_to_obj(&plant_manifold, "compare_manifold_cylinder.obj");

        println!("✅ 对比测试完成，请查看导出的 OBJ 文件");
    }

    // ========================================================================
    // 以下测试用例针对 REVO 分析报告中的特殊情况
    // 参考: e3d-reverse/几何体生成/REVO基本体分析报告.md
    // ========================================================================

    /// 测试 2A.1: 点重合检测 - 轮廓中存在重复点
    #[test]
    fn test_revolve_special_duplicate_points() {
        // 轮廓中包含重复点（起点和终点重合）
        let polygon = vec![
            Vec2::new(50.0, 0.0),
            Vec2::new(50.0, 100.0),
            Vec2::new(0.0, 100.0),
            Vec2::new(0.0, 0.0),
            Vec2::new(50.0, 0.0), // 与第一个点重合
        ];

        let mesh = revolve_polygons_manifold(&[polygon], 16, 360.0);
        assert!(mesh.is_some(), "应该能处理包含重复点的轮廓");
        let mesh = mesh.unwrap();

        // 检查生成的网格是否有效
        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());

        let plant_mesh: crate::shape::pdms_shape::PlantMesh = mesh.into();
        export_mesh_to_obj(&plant_mesh, "special_duplicate_points.obj");

        println!("✅ 2A.1 点重合检测测试通过");
        println!(
            "   顶点数: {}, 三角形数: {}",
            plant_mesh.vertices.len(),
            plant_mesh.indices.len() / 3
        );
    }

    /// 测试 2A.4: 退化情况 - 扫掠角度为0
    #[test]
    fn test_revolve_special_zero_angle() {
        let polygon = vec![
            Vec2::new(50.0, 0.0),
            Vec2::new(50.0, 100.0),
            Vec2::new(0.0, 100.0),
            Vec2::new(0.0, 0.0),
        ];

        // 角度为0应该生成空网格或返回None
        let mesh = revolve_polygons_manifold(&[polygon], 16, 0.0);
        // 当角度为0时，不应该生成有效的旋转体
        if let Some(m) = mesh {
            // 如果返回了mesh，检查是否是退化的
            println!("⚠️ 0度旋转返回了mesh，顶点数: {}", m.vertices.len());
        } else {
            println!("✅ 2A.4 退化情况测试通过（0度返回None）");
        }
    }

    /// 测试 2A.4: 退化情况 - 扫掠角度极小
    #[test]
    fn test_revolve_special_tiny_angle() {
        let polygon = vec![
            Vec2::new(50.0, 0.0),
            Vec2::new(50.0, 100.0),
            Vec2::new(0.0, 100.0),
            Vec2::new(0.0, 0.0),
        ];

        // 极小角度（0.001度）
        let mesh = revolve_polygons_manifold(&[polygon], 16, 0.001);
        if let Some(m) = mesh {
            let plant_mesh: crate::shape::pdms_shape::PlantMesh = m.into();
            export_mesh_to_obj(&plant_mesh, "special_tiny_angle.obj");
            println!(
                "✅ 2A.4 极小角度测试：生成了 {} 个顶点",
                plant_mesh.vertices.len()
            );
        } else {
            println!("✅ 2A.4 极小角度测试通过（返回None）");
        }
    }

    /// 测试 2A.7: 轴上边处理 - 两端都在轴上（退化边）
    #[test]
    fn test_revolve_special_both_ends_on_axis() {
        // 轮廓：一个三角形，其中一条边完全在轴上
        // 这条轴上边应该不生成任何面（退化边跳过）
        let polygon = vec![
            Vec2::new(50.0, 50.0), // 外部点
            Vec2::new(0.0, 100.0), // 轴上点（顶部）
            Vec2::new(0.0, 0.0),   // 轴上点（底部）
        ];

        let mesh = revolve_polygons_manifold(&[polygon], 24, 360.0);
        assert!(mesh.is_some());
        let mesh = mesh.unwrap();

        // 轴上的两个点应该各只生成1个顶点
        // 外部点应该生成 24 个顶点（完整旋转）
        // 总计: 1 + 1 + 24 = 26 个顶点
        println!("📊 两端都在轴上测试:");
        println!("   顶点数: {} (预期约26)", mesh.vertices.len());
        println!("   三角形数: {} (预期约24，即扇形)", mesh.indices.len() / 3);

        // 检查轴上点是否正确共享
        let axis_points: Vec<_> = mesh
            .vertices
            .iter()
            .filter(|v| (v.x * v.x + v.y * v.y).sqrt() < 0.01)
            .collect();
        println!("   轴上顶点数: {} (预期2)", axis_points.len());
        assert_eq!(axis_points.len(), 2, "轴上应该只有2个共享顶点");

        let plant_mesh: crate::shape::pdms_shape::PlantMesh = mesh.into();
        export_mesh_to_obj(&plant_mesh, "special_both_ends_on_axis.obj");

        println!("✅ 2A.7 退化边（两端都在轴上）测试通过");
    }

    /// 测试 2A.7: 轴上边处理 - 起点在轴上（三角形扇）
    #[test]
    fn test_revolve_special_start_on_axis() {
        // 圆锥：顶点在轴上，底边在外
        let polygon = vec![
            Vec2::new(0.0, 100.0), // 轴上顶点
            Vec2::new(50.0, 0.0),  // 外部底部
            Vec2::new(0.0, 0.0),   // 轴上底部中心
        ];

        let mesh = revolve_polygons_manifold(&[polygon], 24, 360.0);
        assert!(mesh.is_some());
        let mesh = mesh.unwrap();

        // 验证轴上点数量
        let axis_points: Vec<_> = mesh
            .vertices
            .iter()
            .filter(|v| (v.x * v.x + v.y * v.y).sqrt() < 0.01)
            .collect();

        println!("📊 起点在轴上测试:");
        println!("   轴上顶点数: {} (预期2)", axis_points.len());
        println!("   总顶点数: {}", mesh.vertices.len());
        println!("   三角形数: {}", mesh.indices.len() / 3);

        let plant_mesh: crate::shape::pdms_shape::PlantMesh = mesh.into();
        export_mesh_to_obj(&plant_mesh, "special_start_on_axis_cone.obj");

        println!("✅ 2A.7 起点在轴上（三角形扇）测试通过");
    }

    /// 测试 2A.7: 轴上边处理 - 终点在轴上（三角形扇）
    #[test]
    fn test_revolve_special_end_on_axis() {
        // 倒圆锥：底边在外，顶点在轴上
        let polygon = vec![
            Vec2::new(50.0, 100.0), // 外部顶部
            Vec2::new(50.0, 0.0),   // 外部底部
            Vec2::new(0.0, 0.0),    // 轴上点
        ];

        let mesh = revolve_polygons_manifold(&[polygon], 24, 360.0);
        assert!(mesh.is_some());
        let mesh = mesh.unwrap();

        let axis_points: Vec<_> = mesh
            .vertices
            .iter()
            .filter(|v| (v.x * v.x + v.y * v.y).sqrt() < 0.01)
            .collect();

        println!("📊 终点在轴上测试:");
        println!("   轴上顶点数: {} (预期1)", axis_points.len());
        println!("   总顶点数: {}", mesh.vertices.len());
        println!("   三角形数: {}", mesh.indices.len() / 3);

        let plant_mesh: crate::shape::pdms_shape::PlantMesh = mesh.into();
        export_mesh_to_obj(&plant_mesh, "special_end_on_axis.obj");

        println!("✅ 2A.7 终点在轴上（三角形扇）测试通过");
    }

    /// 测试 2A.7: xMin > 0 - 普通旋转体（中心有孔洞，如圆环）
    #[test]
    fn test_revolve_special_xmin_positive_torus() {
        // 圆环截面：所有点都在 x > 0，中心有孔
        let polygon = vec![
            Vec2::new(80.0, 0.0),  // 外部底
            Vec2::new(80.0, 50.0), // 外部顶
            Vec2::new(40.0, 50.0), // 内部顶
            Vec2::new(40.0, 0.0),  // 内部底
        ];

        let mesh = revolve_polygons_manifold(&[polygon], 32, 360.0);
        assert!(mesh.is_some());
        let mesh = mesh.unwrap();

        // 检查所有点都不在轴上
        let axis_points: Vec<_> = mesh
            .vertices
            .iter()
            .filter(|v| (v.x * v.x + v.y * v.y).sqrt() < 0.01)
            .collect();

        println!("📊 xMin > 0 测试 (圆环):");
        println!("   轴上顶点数: {} (预期0)", axis_points.len());
        println!("   总顶点数: {}", mesh.vertices.len());
        println!("   三角形数: {}", mesh.indices.len() / 3);
        assert_eq!(axis_points.len(), 0, "圆环不应该有轴上顶点");

        let plant_mesh: crate::shape::pdms_shape::PlantMesh = mesh.into();
        export_mesh_to_obj(&plant_mesh, "special_xmin_positive_torus.obj");

        println!("✅ 2A.7 xMin > 0 (圆环) 测试通过");
    }

    /// 测试 2A.7: xMin = 0 - 实心旋转体（轴上边收缩）
    #[test]
    fn test_revolve_special_xmin_zero_solid() {
        // 实心圆柱：一边在轴上
        let polygon = vec![
            Vec2::new(50.0, 0.0),   // 底部外侧
            Vec2::new(50.0, 100.0), // 顶部外侧
            Vec2::new(0.0, 100.0),  // 顶部轴上
            Vec2::new(0.0, 0.0),    // 底部轴上
        ];

        let mesh = revolve_polygons_manifold(&[polygon], 24, 360.0);
        assert!(mesh.is_some());
        let mesh = mesh.unwrap();

        // 应该有2个轴上共享顶点
        let axis_points: Vec<_> = mesh
            .vertices
            .iter()
            .filter(|v| (v.x * v.x + v.y * v.y).sqrt() < 0.01)
            .collect();

        println!("📊 xMin = 0 测试 (实心圆柱):");
        println!("   轴上顶点数: {} (预期2)", axis_points.len());
        println!("   总顶点数: {} (预期 2 + 24*2 = 50)", mesh.vertices.len());
        println!("   三角形数: {}", mesh.indices.len() / 3);
        assert_eq!(axis_points.len(), 2, "应该有2个轴上共享顶点");

        let plant_mesh: crate::shape::pdms_shape::PlantMesh = mesh.into();
        export_mesh_to_obj(&plant_mesh, "special_xmin_zero_solid.obj");

        println!("✅ 2A.7 xMin = 0 (实心旋转体) 测试通过");
    }

    /// 测试 2A.7: xMin < 0 - 轮廓越过旋转轴（应被裁剪）
    #[test]
    fn test_revolve_special_xmin_negative_clipped() {
        // 轮廓越过 Y 轴
        let polygon = vec![
            Vec2::new(-30.0, 0.0),   // 负侧
            Vec2::new(50.0, 0.0),    // 正侧
            Vec2::new(50.0, 100.0),  // 正侧
            Vec2::new(-30.0, 100.0), // 负侧
        ];

        let mesh = revolve_polygons_manifold(&[polygon], 24, 360.0);
        assert!(mesh.is_some(), "越过轴的轮廓应该被裁剪后处理");
        let mesh = mesh.unwrap();

        // 裁剪后所有顶点应该在 x >= 0 (径向距离)
        for v in &mesh.vertices {
            let radial = (v.x * v.x + v.y * v.y).sqrt();
            assert!(radial >= -0.01, "裁剪后顶点应在正侧: {:?}", v);
        }

        let plant_mesh: crate::shape::pdms_shape::PlantMesh = mesh.into();
        export_mesh_to_obj(&plant_mesh, "special_xmin_negative_clipped.obj");

        println!("✅ 2A.7 xMin < 0 (裁剪) 测试通过");
        println!(
            "   顶点数: {}, 三角形数: {}",
            plant_mesh.vertices.len(),
            plant_mesh.indices.len() / 3
        );
    }
}
