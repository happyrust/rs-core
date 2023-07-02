use std::collections::hash_map::DefaultHasher;
use std::f32::EPSILON;
use std::f64::consts::PI;
use std::hash::Hash;
use std::hash::Hasher;
use crate::shape::pdms_shape::PlantMesh;
use approx::{abs_diff_eq, abs_diff_ne};
use glam::Vec3;
use bevy_ecs::prelude::*;
use bevy_transform::prelude::Transform;
use nom::Parser;
use serde::{Deserialize, Serialize};
use truck_topology::Face;
use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::shape::pdms_shape::VerifiedShape;
use crate::pdms_types::AttrMap;
use crate::prim_geo::CYLINDER_GEO_HASH;
use crate::prim_geo::helper::cal_ref_axis;
use crate::tool::float_tool::hash_f32;

#[derive(Component, Debug, Clone, Serialize, Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, )]
pub struct LCylinder {
    pub paxi_expr: String,
    pub paxi_pt: Vec3,
    //A Axis point
    pub paxi_dir: Vec3,   //A Axis Direction

    pub pbdi: f32,
    //dist to bottom
    pub ptdi: f32,
    //dist to top
    pub pdia: f32,
    //diameter
    pub negative: bool,
}


impl Default for LCylinder {
    fn default() -> Self {
        LCylinder {
            paxi_expr: "Z".to_string(),
            paxi_pt: Default::default(),
            paxi_dir: Vec3::new(0.0, 0.0, 1.0),
            pbdi: -0.5,
            ptdi: 0.5,
            pdia: 1.0,
            negative: false,
        }
    }
}

impl VerifiedShape for LCylinder {
    fn check_valid(&self) -> bool {
        self.pdia > f32::EPSILON && (self.pbdi - self.ptdi).abs() > f32::EPSILON
    }
}

pub fn gen_unit_cylinder() -> PlantMesh{
    let segments = 1;
    let resolution = 36;
    let height = 1.0;
    let radius = 0.5;
    let num_rings = segments + 1;
    let num_vertices = resolution * 2 + num_rings * (resolution + 1);
    let num_faces = resolution * (num_rings - 2);
    let num_indices = (2 * num_faces + 2 * (resolution - 1) * 2) * 3;
    let mut vertices: Vec<Vec3> = Vec::with_capacity(num_vertices as usize);
    let mut normals: Vec<Vec3> = Vec::with_capacity(num_vertices as usize);
    // let mut uvs = Vec::with_capacity(num_vertices as usize);
    let mut indices = Vec::with_capacity(num_indices as usize);

    let step_theta = std::f32::consts::TAU / resolution as f32;
    let step_z = height / segments as f32;

    // rings

    for ring in 0..num_rings {
        let z = 0.0 + ring as f32 * step_z;

        for segment in 0..=resolution {
            let theta = segment as f32 * step_theta;
            let (sin, cos) = theta.sin_cos();

            vertices.push([radius * cos,  radius * sin, z].into());
            normals.push([cos, sin, 0.0].into());
            // uvs.push([
            //     segment as f32 / resolution as f32,
            //     ring as f32 / segments as f32,
            // ]);
        }
    }

    // barrel skin

    for i in 0..segments {
        let ring = i * (resolution + 1);
        let next_ring = (i + 1) * (resolution + 1);

        for j in 0..resolution {
            indices.extend_from_slice(&[
                ring + j + 1,
                next_ring + j,
                ring + j,
                ring + j + 1,
                next_ring + j + 1,
                next_ring + j,
            ]);
        }
    }

    // caps

    let mut build_cap = |top: bool| {
        let offset = vertices.len() as u32;
        let (z, normal_z, winding) = if top {
            (height , 1., (1, 0))
        } else {
            (0.0, -1., (0, 1))
        };

        for i in 0..resolution {
            let theta = i as f32 * step_theta;
            let (sin, cos) = theta.sin_cos();

            vertices.push([cos * radius, sin * radius, z].into());
            normals.push([0.0,  0.0, normal_z].into());
            // uvs.push([0.5 * (cos + 1.0), 1.0 - 0.5 * (sin + 1.0)]);
        }

        for i in 1..(resolution - 1) {
            indices.extend_from_slice(&[
                offset,
                offset + i + winding.1,
                offset + i + winding.0,
            ]);
        }
    };

    // top

    build_cap(true);
    build_cap(false);

    PlantMesh{
        vertices,
        normals,
        indices,
        wire_vertices: vec![],
    }
}


impl From<&AttrMap> for LCylinder {
    fn from(m: &AttrMap) -> Self {
        let pdia = m.get_val("DIAM").unwrap().double_value().unwrap() as f32;
        let pbdi = m.get_val("PBDI").unwrap().double_value().unwrap() as f32;
        let ptdi = m.get_val("PTDI").unwrap().double_value().unwrap() as f32;
        LCylinder {
            paxi_expr: "Z".to_string(),
            paxi_pt: Default::default(),
            paxi_dir: Vec3::Z,
            pbdi,
            ptdi,
            negative: false,
            pdia,
        }
    }
}

impl From<AttrMap> for LCylinder {
    fn from(m: AttrMap) -> Self {
        (&m).into()
    }
}


#[derive(Component, Debug, Clone, Serialize, Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, )]
//
pub struct SCylinder {
    pub paxi_expr: String,
    pub paxi_pt: Vec3,
    //A Axis point
    pub paxi_dir: Vec3,   //A Axis Direction

    // pub pdis: f32,
    //dist to bottom
    pub phei: f32,
    // height
    pub pdia: f32,
    //diameter
    pub btm_shear_angles: [f32; 2],
    // x shear
    pub top_shear_angles: [f32; 2],
    // y shear
    pub negative: bool,

    pub center_in_mid: bool,
}

impl Default for SCylinder {
    fn default() -> Self {
        Self {
            paxi_expr: "Z".to_string(),
            paxi_dir: Vec3::Z,
            paxi_pt: Default::default(),
            phei: 1.0,
            pdia: 1.0,
            btm_shear_angles: [0.0f32; 2],
            top_shear_angles: [0.0f32; 2],
            negative: false,
            center_in_mid: false,
        }
    }
}

impl SCylinder {
    #[inline]
    pub fn is_sscl(&self) -> bool {
        self.btm_shear_angles[0].abs() > f32::EPSILON ||
            self.btm_shear_angles[1].abs() > f32::EPSILON ||
            self.top_shear_angles[0].abs() > f32::EPSILON ||
            self.top_shear_angles[1].abs() > f32::EPSILON
    }
}

impl VerifiedShape for SCylinder {
    #[inline]
    fn check_valid(&self) -> bool {
        self.pdia > f32::EPSILON && self.phei.abs() > f32::EPSILON
    }
}

impl From<&AttrMap> for SCylinder {
    fn from(m: &AttrMap) -> Self {
        let mut phei = m.get_f64("HEIG").unwrap_or_default() as f32;
        let pdia = m.get_f64("DIAM").unwrap_or_default() as f32;
        // dbg!(m);
        SCylinder {
            paxi_expr: "Z".to_string(),
            paxi_pt: Default::default(),
            paxi_dir: Vec3::Z,
            phei,
            pdia,
            btm_shear_angles: [0.0; 2],
            top_shear_angles: [0.0; 2],
            negative: false,
            center_in_mid: true,
        }
    }
}

impl From<AttrMap> for SCylinder {
    fn from(m: AttrMap) -> Self {
        (&m).into()
    }
}