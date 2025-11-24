/// SPINE 策略实现模块

use super::{TransformStrategy, NposHandler, BangHandler};
use crate::rs_surreal::spatial::{
    construct_basis_z_opdir, construct_basis_z_y_exact, construct_basis_z_ref_y,
    construct_basis_z_y_hint, construct_basis_z_y_raw, get_spline_line_dir,
};
use crate::prim_geo::spine::{Spine3D, SpineCurveType, SegmentPath};
use crate::{NamedAttrMap, RefnoEnum, get_type_name, get_children_refnos, get_named_attmap, get_children_named_attmaps, transform::get_effective_parent_att};
use async_trait::async_trait;
use glam::{DMat4, DMat3, DQuat, DVec3, Vec3};

#[derive(Debug, Default, Clone)]
pub struct SpineStrategy {
    // POSSP  属性
    att: NamedAttrMap,
    parent_att: NamedAttrMap,
}

impl SpineStrategy {
    pub fn new(att: NamedAttrMap, parent_att: NamedAttrMap) -> Self {
        Self { att, parent_att }
    }

    /// 从 GENSEC refno 创建 SpineStrategy
    /// 自动获取 GENSEC 下的 SPINE 和第一个 POINSP
    pub async fn from_gensec(gensec_refno: RefnoEnum) -> anyhow::Result<Self> {
        // 首先需要获取到GENSEC 下的 SPINE， 然后获取到 SPINE 下的第一个 POINSP
        let spine_refnos = get_children_refnos(gensec_refno).await?;
        let spine_refno = spine_refnos.first().cloned().unwrap_or_default();
        let poinsp_refnos = get_children_refnos(spine_refno).await?;
        let poinsp_refno = poinsp_refnos.first().cloned().unwrap_or_default();
        
        let poinsp_att = get_named_attmap(poinsp_refno).await?;
        let parent_att = get_effective_parent_att(spine_refno).await?;
        
        Ok(SpineStrategy::new(poinsp_att, parent_att))
    }

    /// 处理 GENSEC 的特殊挤出方向逻辑
    pub async fn extract_spine_extrusion(
        &self,
    ) -> anyhow::Result<(Option<DVec3>, Option<DVec3>)> {
        if let Ok(spine_paths) = self.get_spline_path().await {
            if let Some(first_spine) = spine_paths.first() {
                let dir = (first_spine.pt1 - first_spine.pt0).normalize();
                let pos_extru_dir = Some(dir.as_dvec3());
                let ydir = first_spine.preferred_dir;
                let spine_ydir = if ydir.length_squared() > 0.01 {
                    Some(ydir.as_dvec3())
                } else {
                    None
                };
                return Ok((pos_extru_dir, spine_ydir));
            }
        }

        if let Some(end) = self.att.get_dpose()
            && let Some(start) = self.att.get_dposs()
        {
            return Ok((Some((end - start).normalize()), None));
        }

        Ok((None, None))
    }
}



#[async_trait]
impl TransformStrategy for SpineStrategy {
    async fn get_local_transform(
        &mut self,
    ) -> anyhow::Result<Option<DMat4>> {
        
        let cur_type = self.att.get_type_str();
        // let parent_type = self.parent_att.get_type_str();
        
        let mut pos = self.att.get_position().unwrap_or_default().as_dvec3();
        let mut quat = DQuat::IDENTITY;
        let mut is_world_quat = false;

        // 1. 处理 NPOS 属性
        NposHandler::apply_npos_offset(&mut pos, &self.att);

        // 2. 处理 GENSEC 特有的挤出方向 (对于 GENSEC/SPINE 自身)
        //    或者 POINSP 的切线方向 (对于 SPINE 上的点)
        let (tangent, spine_ydir) = if cur_type == "POINSP" {
            self.calculate_self_tangent().await?
        } else {
            self.extract_spine_extrusion().await?
        };
        
        let spine_ydir = spine_ydir.unwrap_or(DVec3::Z);

        // 3. 处理旋转初始化
        if let Some(dir) = tangent {
            quat = Self::initialize_rotation(
                dir,
                Some(spine_ydir),
            );
        } else {
            // 如果无法计算方向（孤立点？），可能需要默认处理
            // return Ok(None); 
            // 保持 Identity 旋转
        }


        // 4. 处理 BETA 角度
        if self.parent_att.contains_key("BANG") {
            BangHandler::apply_bang(&mut quat, &self.parent_att);
        } else {
            BangHandler::apply_bang(&mut quat, &self.att);
        }

        // 5. 处理 ORI 属性
        if let Some(ori_quat) = self.att.get_rotation() {
            if is_world_quat {
                quat = quat * ori_quat;
            } else {
                quat = ori_quat * quat;
            }
        }

        // 7. 构建最终的变换矩阵
        let mat4 = DMat4::from_rotation_translation(quat, pos);
        Ok(Some(mat4))
    }
}

impl SpineStrategy {
    /// 计算当前 POINSP 的切线方向
    async fn calculate_self_tangent(&self) -> anyhow::Result<(Option<DVec3>, Option<DVec3>)> {
        let owner_refno = self.parent_att.get_refno().unwrap();
        let ch_atts = get_children_named_attmaps(owner_refno)
            .await
            .unwrap_or_default();
        let self_refno = self.att.get_refno().unwrap_or_default();
        let ydir = self.parent_att.get_vec3("YDIR").unwrap_or(Vec3::Z);
        
        let idx = match ch_atts.iter().position(|a| a.get_refno().unwrap_or_default() == self_refno) {
            Some(i) => i,
            None => return Ok((None, Some(ydir.as_dvec3()))),
        };

        let len = ch_atts.len();
        println!("DEBUG: calculate_self_tangent for {:?}, idx={}, len={}", self_refno, idx, len);
        
        // 尝试作为线段/曲线的起点
        if idx < len - 1 {
            let att1 = &ch_atts[idx];
            let att2 = &ch_atts[idx + 1];
            let t1 = att1.get_type_str();
            let t2 = att2.get_type_str();
            println!("DEBUG: Checking Next Segment. t1={}, t2={}", t1, t2);

            if t1 == "POINSP" && t2 == "POINSP" {
                // 直线段起点
                let pt0 = att1.get_position().unwrap_or_default();
                let pt1 = att2.get_position().unwrap_or_default();
                let dir = (pt1 - pt0).normalize().as_dvec3();
                println!("DEBUG: Line Segment. pt0={:?}, pt1={:?}, dir={:?}", pt0, pt1, dir);
                return Ok((Some(dir), Some(ydir.as_dvec3())));
            } else if t1 == "POINSP" && t2 == "CURVE" && idx + 2 < len {
                // 曲线段起点
                let att3 = &ch_atts[idx + 2];
                let spine = Self::construct_spine_segment(att1, att2, att3, ydir);
                let (path, _) = spine.generate_paths();
                let dir = path.tangent_at(0.0).as_dvec3();
                println!("DEBUG: Curve Segment Start. pt0={:?}, mid={:?}, pt1={:?}, dir={:?}", spine.pt0, spine.center_pt, spine.pt1, dir);
                return Ok((Some(dir), Some(ydir.as_dvec3())));
            }
        }

        // 尝试作为线段/曲线的终点 (即最后一个点)
        if idx > 0 && idx == len - 1 {
            let att_end = &ch_atts[idx];
            let att_prev = &ch_atts[idx - 1];
            let t_end = att_end.get_type_str();
            let t_prev = att_prev.get_type_str();
            println!("DEBUG: Checking Prev Segment (End). t_prev={}, t_end={}", t_prev, t_end);

            if t_end == "POINSP" && t_prev == "POINSP" {
                // 直线段终点
                let pt0 = att_prev.get_position().unwrap_or_default();
                let pt1 = att_end.get_position().unwrap_or_default();
                let dir = (pt1 - pt0).normalize().as_dvec3();
                println!("DEBUG: Line Segment End. pt0={:?}, pt1={:?}, dir={:?}", pt0, pt1, dir);
                return Ok((Some(dir), Some(ydir.as_dvec3())));
            } else if t_end == "POINSP" && t_prev == "CURVE" && idx >= 2 {
                // 曲线段终点
                let att_mid = &ch_atts[idx - 1];
                let att_start = &ch_atts[idx - 2];
                let spine = Self::construct_spine_segment(att_start, att_mid, att_end, ydir);
                let (path, _) = spine.generate_paths();
                let dir = path.tangent_at(1.0).as_dvec3();
                println!("DEBUG: Curve Segment End. pt0={:?}, mid={:?}, pt1={:?}, dir={:?}", spine.pt0, spine.center_pt, spine.pt1, dir);
                return Ok((Some(dir), Some(ydir.as_dvec3())));
            }
        }

        Ok((None, Some(ydir.as_dvec3())))
    }

    /// 构造单个 Spine3D 段
    fn construct_spine_segment(
        pt0_att: &NamedAttrMap,
        curve_att: &NamedAttrMap,
        pt1_att: &NamedAttrMap,
        ydir: Vec3,
    ) -> Spine3D {
        let pt0 = pt0_att.get_position().unwrap_or_default();
        let pt1 = pt1_att.get_position().unwrap_or_default();
        let mid_pt = curve_att.get_position().unwrap_or_default();
        let cur_type_str = curve_att.get_str("CURTYP").unwrap_or("unset");
        
        let curve_type = match cur_type_str {
            "CENT" => SpineCurveType::CENT,
            "THRU" => SpineCurveType::THRU,
            _ => SpineCurveType::UNKNOWN,
        };

        Spine3D {
            refno: curve_att.get_refno().unwrap_or_default(),
            pt0,
            pt1,
            thru_pt: mid_pt,
            center_pt: mid_pt,
            cond_pos: curve_att.get_vec3("CPOS").unwrap_or_default(),
            curve_type,
            preferred_dir: ydir,
            radius: curve_att.get_f32("RAD").unwrap_or_default(),
        }
    }

    /// 初始化 SPINE 的旋转逻辑：基于YDIR和两点相减方向计算方位
    fn initialize_rotation(
        pos_extru_dir: DVec3,
        spine_ydir: Option<DVec3>,
    ) ->  DQuat {
        // 优先使用YDIR属性
        if let Some(ydir) = spine_ydir {
            // 基于YDIR和挤出方向计算方位
            return construct_basis_z_y_hint(pos_extru_dir, Some(ydir), false);
        } else {
            // 没有YDIR时，仅基于两点相减的方向计算
            return construct_basis_z_ref_y(pos_extru_dir);
        }
    }

    /// 从 GENSEC/WALL 元素提取 SPINE 路径
    /// 
    /// 此函数从给定的 GENSEC 或 WALL 元素中提取所有子 SPINE 元素的路径信息。
    /// 每个 SPINE 由 POINSP 和 CURVE 点组成，支持直线和曲线两种类型。
    pub async fn get_spline_path(&self) -> anyhow::Result<Vec<Spine3D>> {
        let mut paths = vec![];
        
        // 确保 self.att 已经加载
        if self.att.get_refno().is_none() {
            return Ok(paths);
        }

        let owner_refno = self.parent_att.get_refno().unwrap(); 
        
        let ch_atts = get_children_named_attmaps(owner_refno)
            .await
            .unwrap_or_default();
        let len = ch_atts.len();
        if len < 1 {
            return Ok(paths);
        }
        let ydir = self.parent_att.get_vec3("YDIR").unwrap_or(Vec3::Z);

        let mut i = 0;
        while i < ch_atts.len() - 1 {
            let att1 = &ch_atts[i];
            let t1 = att1.get_type_str();
            let att2 = &ch_atts[(i + 1) % len];
            let t2 = att2.get_type_str();
            if t1 == "POINSP" && t2 == "POINSP" {
                paths.push(Spine3D {
                    refno: att1.get_refno().unwrap(),
                    pt0: att1.get_position().unwrap_or_default(),
                    pt1: att2.get_position().unwrap_or_default(),
                    curve_type: SpineCurveType::LINE,
                    preferred_dir: ydir,
                    ..Default::default()
                });
                // dbg!(&paths);
                i += 1;
            } else if t1 == "POINSP" && t2 == "CURVE" {
                let att3 = &ch_atts[(i + 2) % len];
                let pt0 = att1.get_position().unwrap_or_default();
                let pt1 = att3.get_position().unwrap_or_default();
                let mid_pt = att2.get_position().unwrap_or_default();
                let cur_type_str = att2.get_str("CURTYP").unwrap_or("unset");
                let curve_type = match cur_type_str {
                    "CENT" => SpineCurveType::CENT,
                    "THRU" => SpineCurveType::THRU,
                    _ => SpineCurveType::UNKNOWN,
                };
                paths.push(Spine3D {
                    refno: att2.get_refno().unwrap(),
                    pt0,
                    pt1,
                    thru_pt: mid_pt,
                    center_pt: mid_pt,
                    cond_pos: att2.get_vec3("CPOS").unwrap_or_default(),
                    curve_type,
                    preferred_dir: ydir,
                    radius: att2.get_f32("RAD").unwrap_or_default(),
                });
                i += 2;
            }
        }

        Ok(paths)
    }

    //按照 pkdi 比例和 zdis 距离，沿着 extrusion dir 方向获取位置
    pub async fn cal_trans_by_pkdi_zdis(&self, pkdi: f64, zdis: f64) -> Option<DMat4> {
        // pkdi 是 0-1 之间的比例值，zdis 是沿 spine 方向的手动距离
        if self.att.is_empty() || self.parent_att.is_empty() {
            return None;
        }
        
        // 使用已有的路径生成逻辑
        let paths: Vec<Spine3D> = match self.get_spline_path().await {
            Ok(paths) => paths,
            Err(_) => return None,
        };
        
        if paths.is_empty() {
            return None;
        }
        
        let sweep_path = paths[0].generate_paths().0;
        let lens: Vec<f32> = sweep_path
            .segments
            .iter()
            .map(|x| x.length())
            .collect::<Vec<_>>();
        let total_len: f32 = lens.iter().sum();
        
        if total_len <= 0.0 {
            return None;
        }
        
        let spine_ydir = paths[0].preferred_dir.as_dvec3();
        // pkdi 给了一个比例的距离，加上 zdis 手动距离
        let start_len = (total_len * pkdi.clamp(0.0, 1.0) as f32) as f64;
        let mut tmp_dist = start_len + zdis;
        let mut cur_len = 0.0;
        let mut pos = DVec3::default();
        let mut quat = DQuat::IDENTITY;
        
        for (i, segment) in sweep_path.segments.into_iter().enumerate() {
            tmp_dist -= cur_len;
            cur_len = lens[i] as f64;
            
            // 在当前段范围内，或者是最后一段
            if tmp_dist < cur_len || i == lens.len() - 1 {
                match segment {
                    SegmentPath::Line(_) => {
                        let mut z_dir = get_spline_line_dir(self.parent_att.get_refno().unwrap())
                            .await
                            .unwrap_or_default()
                            .normalize_or_zero();
                        
                        if z_dir.length() == 0.0 {
                            // 使用路径几何直接计算方向作为回退方案
                            let spine = &paths[i];
                            z_dir = (spine.pt1 - spine.pt0).normalize().as_dvec3();
                            
                            if z_dir.length() == 0.0 {
                                quat = DQuat::IDENTITY;
                            } else {
                                quat = construct_basis_z_y_raw(z_dir, spine_ydir);
                            }
                        } else {
                            quat = construct_basis_z_y_raw(z_dir, spine_ydir);
                        }
                        
                        let spine = &paths[i];
                        pos = spine.pt0.as_dvec3() + z_dir * tmp_dist;
                    }
                    SegmentPath::Arc(arc) => {
                        // 使用弧长计算当前点的位置
                        if arc.radius > 1e-6 {
                            let arc_center = arc.center.as_dvec3();
                            let arc_radius = arc.radius as f64;
                            let v = (arc.start_pt.as_dvec3() - arc_center).normalize();
                            let mut start_angle = DVec3::X.angle_between(v);
                            if DVec3::X.cross(v).z < 0.0 {
                                start_angle = -start_angle;
                            }
                            let mut theta = tmp_dist / arc_radius;
                            if arc.clock_wise {
                                theta = -theta;
                            }
                            theta = start_angle + theta;
                            pos = arc_center + arc_radius * DVec3::new(theta.cos(), theta.sin(), 0.0);
                            
                            // 计算弧线在该点的切线方向作为朝向
                            let y_axis = DVec3::Z;
                            let mut x_axis = (arc_center - pos).normalize();
                            if arc.clock_wise {
                                x_axis = -x_axis;
                            }
                            let z_axis = x_axis.cross(y_axis).normalize();
                            quat = DQuat::from_mat3(&DMat3::from_cols(x_axis, y_axis, z_axis));
                        } else {
                            // 半径太小，使用默认朝向
                            quat = DQuat::IDENTITY;
                            pos = arc.start_pt.as_dvec3();
                        }
                    }
                }
                break;
            }
        }
        
        Some(DMat4::from_rotation_translation(quat, pos))
    }

    //获取脊椎的总长度
    pub async fn get_spline_len(&self) -> f64 {
        if self.att.is_empty() || self.parent_att.is_empty() {
            return 0.0;
        }
        
        // 使用已有的路径生成逻辑
        let paths: Vec<Spine3D> = match self.get_spline_path().await {
            Ok(paths) => paths,
            Err(_) => return 0.0,
        };
        
        if paths.is_empty() {
            return 0.0;
        }
        
        let sweep_path = paths[0].generate_paths().0;
        let total_len: f32 = sweep_path
            .segments
            .iter()
            .map(|x: &SegmentPath| x.length())
            .sum();
            
        total_len as f64
    }



}


