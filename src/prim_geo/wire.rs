use serde_derive::{Deserialize, Serialize};
use truck_base::cgmath64::{InnerSpace, MetricSpace, Point3, Rad, Vector3};
use glam::{DVec3, Vec3};

use std::f32::consts::PI;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use anyhow::anyhow;
use crate::prim_geo::extrusion::Extrusion;
use crate::shape::pdms_shape::BrepMathTrait;
use crate::tool::float_tool::hash_vec3;
use approx::{abs_diff_eq, abs_diff_ne};


#[derive(Debug, Clone, Serialize, Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, )]
pub enum CurveType {
    Fill,
    Spline(f32),  //thick
}

pub fn cal_circus_center(pt0: Vec3, pt1: Vec3, pt2: Vec3) -> Vec3 {
    let vec0 = pt1 - pt0;
    let vec1 = pt2 - pt0;
    let a2 = vec0.dot(vec0);
    let ab = vec0.dot(vec1);
    let b2 = vec1.dot(vec1);
    let det = a2 * b2 - ab * ab;
    let u = (b2 * a2 - ab * b2) / (2.0 * det);
    let v = (-ab * a2 + b2 * a2) / (2.0 * det);
    pt0 + u * vec0 + v * vec1
}

pub fn circus_center(pt0: Point3, pt1: Point3, pt2: Point3) -> Point3 {
    let vec0 = pt1 - pt0;
    let vec1 = pt2 - pt0;
    let a2 = vec0.dot(vec0);
    let ab = vec0.dot(vec1);
    let b2 = vec1.dot(vec1);
    let det = a2 * b2 - ab * ab;
    let u = (b2 * a2 - ab * b2) / (2.0 * det);
    let v = (-ab * a2 + b2 * a2) / (2.0 * det);
    pt0 + u * vec0 + v * vec1
}



