use std::collections::hash_map::DefaultHasher;
use std::f32::EPSILON;
use std::hash::{Hash, Hasher};
use bevy::prelude::*;
use glam::Vec3;
use lyon::path::builder::PathBuilder;
use lyon::path::Path;
use lyon::tessellation::*;
use parry3d::bounding_volume::AABB;
use parry3d::math::{Point, Vector};
use truck_modeling::Shell;
use serde::{Serialize,Deserialize};
use crate::shape::pdms_shape::{BrepMathTrait, BrepShapeTrait, PdmsMesh, VerifiedShape};

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Default)]
pub struct Facet {
    pub polygons: Vec<Polygon>,
}


#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Default)]
pub struct Polygon {
    pub contours: Vec<Contour>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Default)]
pub struct Contour {
    pub vertices: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
}



impl VerifiedShape for Facet {
    fn check_valid(&self) -> bool { true }
}

//#[typetag::serde]
impl BrepShapeTrait for Facet {

    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    fn hash_unit_mesh_params(&self) -> u64{
        let bytes = bincode::serialize(self).unwrap();
        let mut hasher = DefaultHasher::default();
        bytes.hash(&mut hasher);
        hasher.finish()
    }

    fn gen_unit_mesh(&self) -> Option<PdmsMesh>{
        self.gen_mesh(None)
    }


    #[inline]
    fn get_scaled_vec3(&self) -> Vec3 {
        Vec3::ONE
    }

    fn gen_brep_shell(& self) -> Option<Shell> {
        None
    }

    fn gen_mesh(&self, tol: Option<f32>) -> Option<PdmsMesh>{
        let mut vertices = vec![];
        let mut normals = vec![];
        let mut indices = vec![];
        let mut uvs = vec![];
        let delta = 0.001;
        let mut aabb = AABB::new_invalid();
        for p in self.polygons.iter(){
            if p.contours.len() == 0{ continue; }
            let mut path = Path::builder();
            let mut tess = FillTessellator::new();
            let mut outbuf: VertexBuffers<usize, u16> = VertexBuffers::new();
            let vert_cnt = vertices.len();
            let mut coord_sys = [Vec3::ZERO; 3];
            for c in p.contours.iter() {
                let tmp_vert_cnt = vertices.len();
                if c.vertices.len() >= 3{
                    for i in 0..c.vertices.len(){
                        let v = c.vertices[i];
                        aabb.take_point(Point::new(v[0], v[1] , v[2] ));
                        vertices.push(v);
                        normals.push(c.normals[i]);
                        uvs.push([0.0, 0.0]);
                    }
                    let pt_2d = Self::to2d(&vertices[tmp_vert_cnt..], normals[tmp_vert_cnt], &mut coord_sys);
                    path.add_polygon(lyon::path::Polygon{
                        points: &pt_2d,
                        closed: true
                    });
                }else{
                    // //dbg!(&d);   //暂时不考虑直线的情况
                }
            }
            let path = path.build();
            tess.tessellate_with_ids(
                path.id_iter(),
                &path,
                None,
                &FillOptions::default()/*.with_tolerance(0.5)*/.with_intersections(true),
                &mut BuffersBuilder::new(&mut outbuf,
                                         |vertex: FillVertex| {
                                             if let Some(id) = vertex.as_endpoint_id(){
                                                 return id.to_usize();
                                             }
                                             return 0usize;
                                         },
                ),
            ).ok();
            for index in outbuf.indices {
                let endpoint_id = outbuf.vertices[index as usize];
                indices.push((endpoint_id as usize + vert_cnt) as u32);
            }
        }

        //对于facet，可以绘制convex hull的线框
        return  Some(PdmsMesh{
            indices,
            vertices,
            normals,
            wf_indices: vec![],
            wf_vertices: vec![],
            aabb: Some(aabb),
            unit_shape: Default::default(),
            // shape_data: self.gen_unit_shape(),
        });
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }
}

impl Facet {
    fn to2d(pts: &[[f32; 3]], normal: [f32;3], coord_sys: &mut [Vec3; 3]) -> Vec<lyon::math::Point> {
        let mut polygon2d = Vec::new();
        let mut x_n: Vec3;
        let mut y_n: Vec3;
        let mut v0: Vec3;
        if coord_sys[1].length_squared() < f32::EPSILON{
            v0 = Vec3::from_slice(&pts[0]);
            let v1 = Vec3::from_slice(&pts[1]);
            let mut loc_x = (v1 - v0).normalize();
            let mut n = Vec3::from_slice(&normal).normalize();

            let loc_y = n.cross(loc_x);
            x_n = loc_x.normalize();
            y_n = loc_y.normalize();

            coord_sys[0] = v0;
            coord_sys[1] = x_n;
            coord_sys[2] = y_n;
        }else{
            v0 = coord_sys[0];
            x_n = coord_sys[1];
            y_n = coord_sys[2];
        }

        for idx in 0..pts.len() {
            let to_p = Vec3::from_slice(&pts[idx]) - v0;
            polygon2d.push(lyon::math::Point::new(to_p.dot(x_n) as f32, to_p.dot(y_n) as f32));
        }
        polygon2d
    }
}


