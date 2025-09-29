use glam::Vec3;
use std::fs;
use std::path::Path;

/// SPINE路径的SVG生成器
pub struct SpineSvgGenerator {
    width: f32,
    height: f32,
    margin: f32,
    scale: f32,
    points: Vec<(String, Vec3, Option<f32>)>, // (type, position, radius)
    show_labels: bool,     // 是否显示长度标签
    show_coordinates: bool, // 是否显示坐标
    show_legend: bool,     // 是否显示图例
}

impl SpineSvgGenerator {
    pub fn new() -> Self {
        Self {
            width: 800.0,
            height: 600.0,
            margin: 50.0,
            scale: 1.0,
            points: Vec::new(),
            show_labels: true,
            show_coordinates: true,
            show_legend: true,
        }
    }

    /// 设置是否显示标签和图例
    pub fn set_display_options(&mut self, show_labels: bool, show_coordinates: bool, show_legend: bool) {
        self.show_labels = show_labels;
        self.show_coordinates = show_coordinates;
        self.show_legend = show_legend;
    }

    /// 添加路径点
    pub fn add_point(&mut self, point_type: String, position: Vec3, radius: Option<f32>) {
        self.points.push((point_type, position, radius));
    }

    /// 设置画布尺寸
    pub fn set_canvas_size(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;
    }

    /// 计算合适的缩放比例和偏移
    fn calculate_transform(&self) -> (f32, Vec3) {
        if self.points.is_empty() {
            return (1.0, Vec3::ZERO);
        }

        // 找到边界框
        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;

        for (_, pos, _) in &self.points {
            min_x = min_x.min(pos.x);
            max_x = max_x.max(pos.x);
            min_y = min_y.min(pos.y);
            max_y = max_y.max(pos.y);
        }

        let data_width = max_x - min_x;
        let data_height = max_y - min_y;

        // 计算缩放比例
        let canvas_width = self.width - 2.0 * self.margin;
        let canvas_height = self.height - 2.0 * self.margin;

        let scale_x = if data_width > 0.0 { canvas_width / data_width } else { 1.0 };
        let scale_y = if data_height > 0.0 { canvas_height / data_height } else { 1.0 };
        let scale = scale_x.min(scale_y);

        // 计算偏移量，使图形居中
        let offset_x = self.margin + (canvas_width - data_width * scale) / 2.0 - min_x * scale;
        let offset_y = self.margin + (canvas_height - data_height * scale) / 2.0 - min_y * scale;

        (scale, Vec3::new(offset_x, offset_y, 0.0))
    }

    /// 将世界坐标转换为SVG坐标
    fn world_to_svg(&self, pos: Vec3, scale: f32, offset: Vec3) -> (f32, f32) {
        let x = pos.x * scale + offset.x;
        // SVG的Y轴向下，所以需要翻转
        let y = self.height - (pos.y * scale + offset.y);
        (x, y)
    }

    /// 生成SVG内容
    pub fn generate_svg(&self) -> String {
        if self.points.is_empty() {
            return self.empty_svg();
        }

        let (scale, offset) = self.calculate_transform();
        let mut svg = String::new();

        // SVG头部
        svg.push_str(&format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<svg width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\" xmlns=\"http://www.w3.org/2000/svg\">\n<defs>\n    <marker id=\"arrowhead\" markerWidth=\"10\" markerHeight=\"7\" refX=\"10\" refY=\"3.5\" orient=\"auto\">\n      <polygon points=\"0 0, 10 3.5, 0 7\" fill=\"#666\" />\n    </marker>\n</defs>\n<style>\n    .point-poinsp {{ fill: #2196F3; stroke: #1976D2; stroke-width: 2; }}\n    .point-curve {{ fill: #FF9800; stroke: #F57F17; stroke-width: 2; }}\n    .path-line {{ stroke: #4CAF50; stroke-width: 3; fill: none; marker-end: url(#arrowhead); }}\n    .path-arc {{ stroke: #E91E63; stroke-width: 3; fill: none; marker-end: url(#arrowhead); }}\n    .label {{ font-family: Arial, sans-serif; font-size: 12px; fill: #333; }}\n    .coord-label {{ font-family: Arial, sans-serif; font-size: 10px; fill: #666; }}\n    .grid {{ stroke: #eee; stroke-width: 1; }}\n    .axis {{ stroke: #ccc; stroke-width: 2; }}\n</style>\n",
            self.width, self.height, self.width, self.height
        ));

        // 绘制网格
        svg.push_str(&self.generate_grid(scale, offset));

        // 绘制路径
        svg.push_str(&self.generate_paths(scale, offset));

        // 绘制点
        svg.push_str(&self.generate_points(scale, offset));

        // 绘制标题和信息
        svg.push_str(&self.generate_info());

        svg.push_str("</svg>");
        svg
    }

    /// 生成网格
    fn generate_grid(&self, scale: f32, offset: Vec3) -> String {
        let mut grid = String::new();

        // 简单的网格线
        let grid_step = 100.0; // 每100单位画一条网格线
        let (min_x, max_x, min_y, max_y) = self.get_bounds();

        let start_x = (min_x / grid_step).floor() * grid_step;
        let end_x = (max_x / grid_step).ceil() * grid_step;
        let start_y = (min_y / grid_step).floor() * grid_step;
        let end_y = (max_y / grid_step).ceil() * grid_step;

        // 垂直网格线
        let mut x = start_x;
        while x <= end_x {
            let (svg_x, _) = self.world_to_svg(Vec3::new(x, 0.0, 0.0), scale, offset);
            grid.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" class=\"grid\" />\n",
                svg_x, self.margin, svg_x, self.height - self.margin
            ));
            x += grid_step;
        }

        // 水平网格线
        let mut y = start_y;
        while y <= end_y {
            let (_, svg_y) = self.world_to_svg(Vec3::new(0.0, y, 0.0), scale, offset);
            grid.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" class=\"grid\" />\n",
                self.margin, svg_y, self.width - self.margin, svg_y
            ));
            y += grid_step;
        }

        grid
    }

    /// 获取数据边界
    fn get_bounds(&self) -> (f32, f32, f32, f32) {
        if self.points.is_empty() {
            return (0.0, 0.0, 0.0, 0.0);
        }

        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;

        for (_, pos, _) in &self.points {
            min_x = min_x.min(pos.x);
            max_x = max_x.max(pos.x);
            min_y = min_y.min(pos.y);
            max_y = max_y.max(pos.y);
        }

        (min_x, max_x, min_y, max_y)
    }

    /// 生成路径
    fn generate_paths(&self, scale: f32, offset: Vec3) -> String {
        let mut paths = String::new();
        let mut i = 0;

        while i < self.points.len() {
            if i + 1 >= self.points.len() {
                break;
            }

            let current = &self.points[i];
            let next = &self.points[i + 1];

            // POINSP to POINSP: 直线
            if current.0 == "POINSP" && next.0 == "POINSP" {
                let (x1, y1) = self.world_to_svg(current.1, scale, offset);
                let (x2, y2) = self.world_to_svg(next.1, scale, offset);

                paths.push_str(&format!(
                    "<line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" class=\"path-line\" />\n",
                    x1, y1, x2, y2
                ));

                // 添加长度标签（可选）
                if self.show_labels {
                    let length = current.1.distance(next.1);
                    let mid_x = (x1 + x2) / 2.0;
                    let mid_y = (y1 + y2) / 2.0;
                    paths.push_str(&format!(
                        "<text x=\"{:.1}\" y=\"{:.1}\" class=\"label\" text-anchor=\"middle\">{:.1}mm</text>\n",
                        mid_x, mid_y - 5.0, length
                    ));
                }

                i += 1;
            }
            // POINSP to CURVE to POINSP: 弧线
            else if current.0 == "POINSP" && next.0 == "CURVE" && i + 2 < self.points.len() {
                let after_curve = &self.points[i + 2];
                if after_curve.0 == "POINSP" {
                    let (x1, y1) = self.world_to_svg(current.1, scale, offset);
                    let (x2, y2) = self.world_to_svg(next.1, scale, offset);
                    let (x3, y3) = self.world_to_svg(after_curve.1, scale, offset);

                    // 绘制弧线 - 计算正确的弧形控制点
                    if let Some(radius) = next.2 {
                        // 计算弧线的正确控制点
                        let (control_x, control_y) = calculate_arc_control_point(
                            (x1, y1), (x3, y3), radius, scale
                        );

                        paths.push_str(&format!(
                            "<path d=\"M {:.1} {:.1} Q {:.1} {:.1} {:.1} {:.1}\" class=\"path-arc\" />\n",
                            x1, y1, control_x, control_y, x3, y3
                        ));
                    } else {
                        // 如果没有半径信息，使用CURVE点作为控制点
                        paths.push_str(&format!(
                            "<path d=\"M {:.1} {:.1} Q {:.1} {:.1} {:.1} {:.1}\" class=\"path-arc\" />\n",
                            x1, y1, x2, y2, x3, y3
                        ));
                    }

                    // 添加弧线信息（可选）
                    if self.show_labels {
                        if let Some(radius) = next.2 {
                            let chord_length = current.1.distance(after_curve.1);
                            let angle = 2.0 * (chord_length / (2.0 * radius)).asin();
                            let arc_length = radius * angle;

                            paths.push_str(&format!(
                                "<text x=\"{:.1}\" y=\"{:.1}\" class=\"label\" text-anchor=\"middle\">Arc: {:.1}mm</text>\n<text x=\"{:.1}\" y=\"{:.1}\" class=\"coord-label\" text-anchor=\"middle\">R={:.1}</text>\n",
                                x2, y2 - 15.0, arc_length,
                                x2, y2 + 10.0, radius
                            ));
                        }
                    }

                    i += 2;
                } else {
                    i += 1;
                }
            } else {
                i += 1;
            }
        }

        paths
    }

    /// 生成点
    fn generate_points(&self, scale: f32, offset: Vec3) -> String {
        let mut points = String::new();

        for (i, (point_type, pos, radius)) in self.points.iter().enumerate() {
            let (x, y) = self.world_to_svg(*pos, scale, offset);

            let class = match point_type.as_str() {
                "POINSP" => "point-poinsp",
                "CURVE" => "point-curve",
                _ => "point-poinsp",
            };

            // 绘制点
            points.push_str(&format!(
                "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"6\" class=\"{}\" />\n",
                x, y, class
            ));

            // 添加点标签（可选）
            if self.show_labels {
                points.push_str(&format!(
                    "<text x=\"{:.1}\" y=\"{:.1}\" class=\"label\" text-anchor=\"middle\">{}</text>\n",
                    x, y - 12.0, i
                ));
            }

            // 添加坐标（可选）
            if self.show_coordinates {
                points.push_str(&format!(
                    "<text x=\"{:.1}\" y=\"{:.1}\" class=\"coord-label\" text-anchor=\"middle\">({:.0},{:.0})</text>\n",
                    x, y + 20.0, pos.x, pos.y
                ));
            }
        }

        points
    }

    /// 生成信息面板
    fn generate_info(&self) -> String {
        let mut info = String::new();

        if !self.show_legend {
            return info;
        }

        // 计算总长度
        let total_length = self.calculate_total_length();

        info.push_str(&format!(
            "<rect x=\"10\" y=\"10\" width=\"200\" height=\"80\" fill=\"white\" stroke=\"#ccc\" stroke-width=\"1\" rx=\"5\" />\n<text x=\"20\" y=\"30\" class=\"label\">SPINE路径信息</text>\n<text x=\"20\" y=\"50\" class=\"coord-label\">总长度: {:.3} mm</text>\n<text x=\"20\" y=\"70\" class=\"coord-label\">点数: {} ({}个POINSP, {}个CURVE)</text>\n",
            total_length,
            self.points.len(),
            self.points.iter().filter(|(t, _, _)| t == "POINSP").count(),
            self.points.iter().filter(|(t, _, _)| t == "CURVE").count()
        ));

        // 图例（只显示点类型，不显示路径类型文字）
        info.push_str(
            "<circle cx=\"230\" cy=\"30\" r=\"6\" class=\"point-poinsp\" />\n<text x=\"245\" y=\"35\" class=\"coord-label\">POINSP</text>\n<circle cx=\"230\" cy=\"50\" r=\"6\" class=\"point-curve\" />\n<text x=\"245\" y=\"55\" class=\"coord-label\">CURVE</text>\n<line x1=\"230\" y1=\"70\" x2=\"260\" y2=\"70\" class=\"path-line\" />\n<path d=\"M 230 90 Q 245 85 260 90\" class=\"path-arc\" />\n"
        );

        info
    }

    /// 计算总路径长度
    fn calculate_total_length(&self) -> f32 {
        let mut total_length = 0.0;
        let mut i = 0;

        while i < self.points.len() {
            if i + 1 >= self.points.len() {
                break;
            }

            let current = &self.points[i];
            let next = &self.points[i + 1];

            // POINSP to POINSP: 直线
            if current.0 == "POINSP" && next.0 == "POINSP" {
                total_length += current.1.distance(next.1);
                i += 1;
            }
            // POINSP to CURVE to POINSP: 弧线
            else if current.0 == "POINSP" && next.0 == "CURVE" && i + 2 < self.points.len() {
                let after_curve = &self.points[i + 2];
                if after_curve.0 == "POINSP" {
                    if let Some(radius) = next.2 {
                        let chord_length = current.1.distance(after_curve.1);
                        if chord_length <= 2.0 * radius {
                            let angle = 2.0 * (chord_length / (2.0 * radius)).asin();
                            let arc_length = radius * angle;
                            total_length += arc_length;
                        } else {
                            // 使用直线近似
                            total_length += current.1.distance(next.1) + next.1.distance(after_curve.1);
                        }
                    } else {
                        total_length += current.1.distance(next.1) + next.1.distance(after_curve.1);
                    }
                    i += 2;
                } else {
                    i += 1;
                }
            } else {
                i += 1;
            }
        }

        total_length
    }

    /// 空SVG
    fn empty_svg(&self) -> String {
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<svg width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\" xmlns=\"http://www.w3.org/2000/svg\">\n<text x=\"50%\" y=\"50%\" text-anchor=\"middle\" style=\"font-family: Arial; font-size: 16px;\">No SPINE data to display</text>\n</svg>",
            self.width, self.height, self.width, self.height
        )
    }

    /// 保存SVG到文件
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        let svg_content = self.generate_svg();
        fs::write(path, svg_content)
    }
}

/// 计算弧线的贝塞尔控制点，使弧线更加明显
fn calculate_arc_control_point(
    start: (f32, f32),
    end: (f32, f32),
    world_radius: f32,
    scale: f32,
) -> (f32, f32) {
    let (x1, y1) = start;
    let (x3, y3) = end;

    // 计算弦的中点
    let mid_x = (x1 + x3) / 2.0;
    let mid_y = (y1 + y3) / 2.0;

    // 计算弦长
    let chord_length = ((x3 - x1).powi(2) + (y3 - y1).powi(2)).sqrt();

    // 如果弦长为0，返回中点
    if chord_length == 0.0 {
        return (mid_x, mid_y);
    }

    // 将世界坐标的半径转换为SVG坐标
    let svg_radius = world_radius * scale;

    // 计算弦心距（从弦中点到圆心的距离）
    let chord_half = chord_length / 2.0;

    // 避免半径小于弦长一半的情况（会导致无效的弧）
    if svg_radius < chord_half {
        // 如果半径太小，创建一个更突出的弧形控制点
        let perpendicular_distance = chord_half * 0.5; // 使用弦长的1/4作为突出距离

        // 计算垂直于弦的方向向量
        let chord_dir_x = (x3 - x1) / chord_length;
        let chord_dir_y = (y3 - y1) / chord_length;

        // 垂直向量（逆时针旋转90度）
        let perp_x = -chord_dir_y;
        let perp_y = chord_dir_x;

        // 控制点位于弦中点的垂直方向
        (mid_x + perp_x * perpendicular_distance,
         mid_y + perp_y * perpendicular_distance)
    } else {
        // 正常情况：计算真正的弧形
        let sagitta = svg_radius - (svg_radius.powi(2) - chord_half.powi(2)).sqrt();

        // 计算垂直于弦的方向向量
        let chord_dir_x = (x3 - x1) / chord_length;
        let chord_dir_y = (y3 - y1) / chord_length;

        // 垂直向量（逆时针旋转90度）
        let perp_x = -chord_dir_y;
        let perp_y = chord_dir_x;

        // 控制点位于弦中点加上矢高距离
        (mid_x + perp_x * sagitta * 1.5, // 乘以1.5让弧线更明显
         mid_y + perp_y * sagitta * 1.5)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_svg_generation() {
        let mut generator = SpineSvgGenerator::new();

        // 添加测试数据
        generator.add_point("POINSP".to_string(), Vec3::new(0.0, 0.0, 0.0), None);
        generator.add_point("POINSP".to_string(), Vec3::new(100.0, 0.0, 0.0), None);
        generator.add_point("CURVE".to_string(), Vec3::new(150.0, 50.0, 0.0), Some(50.0));
        generator.add_point("POINSP".to_string(), Vec3::new(200.0, 100.0, 0.0), None);

        let svg = generator.generate_svg();
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("path-line"));
        assert!(svg.contains("path-arc"));
    }
}