use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::prim_geo::wire::*;
#[cfg(feature = "truck")]
use crate::shape::pdms_shape::BrepMathTrait;
use crate::shape::pdms_shape::{BrepShapeTrait, PlantMesh, RsVec3, TRI_TOL, VerifiedShape};
use crate::tool::float_tool::{f32_round_3, hash_f32, hash_vec3};
use approx::AbsDiffEq;
use approx::abs_diff_eq;
use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
#[cfg(feature = "truck")]
use truck_meshalgo::prelude::*;
#[cfg(feature = "truck")]
use truck_modeling::{Surface, builder};
#[cfg(feature = "truck")]
use truck_stepio::out;

#[derive(
    Debug, Clone, Serialize, Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize,
)]
pub struct Revolution {
    pub verts: Vec<Vec<Vec3>>,
    pub angle: f32,
    pub rot_dir: Vec3,
    pub rot_pt: Vec3,
}

impl Default for Revolution {
    fn default() -> Self {
        Self {
            verts: vec![],
            angle: 90.0,
            rot_dir: Vec3::X,   //默认绕X轴旋转
            rot_pt: Vec3::ZERO, //默认旋转点
        }
    }
}

impl VerifiedShape for Revolution {
    fn check_valid(&self) -> bool {
        //add some other restrictions
        true
        // self.angle.abs() > std::f32::EPSILON
    }
}

impl BrepShapeTrait for Revolution {
    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    ///revolve 有些问题，暂时用manifold来代替
    ///如果是沿自己的一条边旋转，需要弄清楚为啥三角化出来的不对
    #[cfg(feature = "truck")]
    fn gen_brep_shell(&self) -> Option<truck_modeling::Shell> {
        if !self.check_valid() {
            return None;
        }
        let wire = gen_wire(&self.verts, &self.fradius_vec).unwrap();

        //如果截面包含了原点，就考虑用分成两块的办法
        // let contains_origin = polygon.contains(&point!{ x: 0.0, y: 0.0 });
        if let Ok(mut face) = builder::try_attach_plane(&[wire]) {
            if let Surface::Plane(plane) = face.surface() {
                let mut rot_dir = self.rot_dir.normalize().vector3();
                let rot_pt = self.rot_pt.point3();
                //避免精度的误差
                let mut angle = (f32_round_3(self.angle) as f64).to_radians();
                let mut axis_reversed = false;
                if angle < 0.0 {
                    angle = -angle;
                    rot_dir = -rot_dir;
                    axis_reversed = true;
                }
                let z_flag = plane.normal().z > 0.0;
                // //如果两者一致，就不需要reverse
                // if z_flag && axis_reversed {
                face = face.inverse();
                // }

                //check if exist any point on axis
                let axis_on_edge = self.verts.iter().any(|x| {
                    x.y.abs().abs_diff_eq(&0.0, 0.01) && x.z.abs().abs_diff_eq(&0.0, 0.01)
                });
                {
                    let s = builder::rsweep(&face, rot_pt, rot_dir, Rad(angle as f64));
                    let output_step_file = "revo.stp";
                    let step_string = out::CompleteStepDisplay::new(
                        out::StepModel::from(&s.compress()),
                        out::StepHeaderDescriptor {
                            organization_system: "shape-to-step".to_owned(),
                            ..Default::default()
                        },
                    )
                    .to_string();
                    let mut step_file = std::fs::File::create(&output_step_file).unwrap();
                    std::io::Write::write_all(&mut step_file, step_string.as_ref()).unwrap();

                    let new_s = builder::transformed(&s, Matrix4::from_scale(0.01));
                    let json = serde_json::to_vec_pretty(&new_s).unwrap();
                    std::fs::write("revo.json", json).unwrap();

                    let shell = s.into_boundaries().pop();
                    if shell.is_none() {
                        dbg!(&self);
                    }

                    return shell;
                }
            }
        }
        None
    }

    fn hash_unit_mesh_params(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.verts.iter().flatten().for_each(|v| {
            hash_vec3::<DefaultHasher>(v, &mut hasher);
        });
        "Revolution".hash(&mut hasher);
        hash_f32(self.angle, &mut hasher);
        hasher.finish()
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    #[inline]
    fn tol(&self) -> f32 {
        use parry2d::bounding_volume::Aabb;
        let pts = self
            .verts
            .iter()
            .flatten()
            .map(|x| nalgebra::Point2::from(nalgebra::Vector2::from(x.truncate())))
            .collect::<Vec<_>>();
        let profile_aabb = Aabb::from_points(pts.iter().copied());
        0.001 * profile_aabb.bounding_sphere().radius.max(1.0)
    }

    fn convert_to_geo_param(&self) -> Option<PdmsGeoParam> {
        Some(PdmsGeoParam::PrimRevolution(self.clone()))
    }

    /// 使用统一的 ProfileProcessor 生成旋转体的mesh
    /// 
    /// 统一流程：cavalier_contours + i_triangle
    fn gen_csg_mesh(&self) -> Option<PlantMesh> {
        if !self.check_valid() {
            return None;
        }
        if self.verts.is_empty() || self.verts[0].len() < 3 {
            return None;
        }

        // 使用统一的 ProfileProcessor 处理截面（支持多轮廓和孔洞）
        use crate::prim_geo::profile_processor::{ProfileProcessor, revolve_profile};

        let mut verts2d: Vec<Vec<Vec2>> = Vec::with_capacity(self.verts.len());
        let mut frads: Vec<Vec<f32>> = Vec::with_capacity(self.verts.len());
        for wire in &self.verts {
            let mut v2 = Vec::with_capacity(wire.len());
            let mut r = Vec::with_capacity(wire.len());
            for p in wire {
                v2.push(Vec2::new(p.x, p.y));
                r.push(p.z);
            }
            verts2d.push(v2);
            frads.push(r);
        }

        let processor = ProfileProcessor::from_wires(verts2d, frads, true)
            .map_err(|e| {
                println!("⚠️  [Revolution] ProfileProcessor 创建失败: {}", e);
                e
            })
            .ok()?;
        let profile = processor.process("REVOLUTION", None).ok()?;

        // 计算旋转参数
        let segments = ((self.angle.abs() / 10.0).ceil() as usize).clamp(8, 64);
        let rot_axis = self.rot_dir.normalize();
        let rot_center = self.rot_pt;

        // 旋转截面
        let revolved = revolve_profile(&profile, self.angle, segments, rot_axis, rot_center);

        // 计算 UV 坐标（简化版）
        let uvs = revolved
            .vertices
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let seg_idx = i / profile.contour_points.len();
                let prof_idx = i % profile.contour_points.len();
                [
                    prof_idx as f32 / profile.contour_points.len() as f32,
                    seg_idx as f32 / segments as f32,
                ]
            })
            .collect();

        Some(PlantMesh {
            vertices: revolved.vertices,
            normals: revolved.normals,
            uvs,
            indices: revolved.indices,
            wire_vertices: Vec::new(),
            edges: Vec::new(),
            aabb: None,
        })
    }

    fn gen_csg_shape(&self) -> anyhow::Result<crate::prim_geo::basic::CsgSharedMesh> {
        if let Some(mesh) = self.gen_csg_mesh() {
            Ok(crate::prim_geo::basic::CsgSharedMesh::new(mesh))
        } else {
            Err(anyhow::anyhow!(
                "Failed to generate CSG mesh for Revolution"
            ))
        }
    }

    fn enhanced_key_points(
        &self,
        transform: &bevy_transform::prelude::Transform,
    ) -> Vec<(Vec3, String, u8)> {
        // Revolution 是复杂的 Mesh 类型，只返回旋转中心点
        vec![(
            transform.transform_point(self.rot_pt),
            "Center".to_string(),
            100,
        )]
    }
}
