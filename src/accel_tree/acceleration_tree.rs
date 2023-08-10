use std::fs::File;
use std::io::Write;
use std::ops::{Deref, DerefMut};
use bevy_ecs::prelude::Resource;
use glam::{Mat4, Vec3};
use nalgebra::Point3;
use parry3d::bounding_volume::Aabb;
use rstar::Envelope;
use serde_derive::{Deserialize, Serialize};
use crate::pdms_types::RefU64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RStarBoundingBox {
    pub aabb: rstar::AABB<[f32; 3]>,
    pub refno: RefU64,
}

impl RStarBoundingBox {
    pub fn from_aabb(bounds: &Aabb, refno: RefU64) -> Self {
        Self {
            aabb: rstar::AABB::from_corners([bounds.mins[0], bounds.mins[1], bounds.mins[2]],
                                            [bounds.maxs[0], bounds.maxs[1], bounds.maxs[2]]),
            refno,
        }
    }

    pub fn new(min: Vec3, max: Vec3, transform: Mat4, refno: RefU64) -> Self {
        let min = transform.transform_point3(min);
        let max = transform.transform_point3(max);

        Self {
            aabb: rstar::AABB::from_corners(min.into(), max.into()),
            refno,
        }
    }
}


// impl rstar::SelectionFunction<RStarBoundingBox> for &Ray {
//     fn should_unpack_parent(&self, envelope: &rstar::AABB<[f32; 3]>) -> bool {
//         self.bounding_box_intersection(envelope.lower().into(), envelope.upper().into())
//             .is_some()
//     }
//
//     fn should_unpack_leaf(&self, bounding_box: &ShipBoundingBox) -> bool {
//         self.bounding_box_intersection(
//             bounding_box.aabb.lower().into(),
//             bounding_box.aabb.upper().into(),
//         ).is_some()
//     }
// }

impl rstar::RTreeObject for RStarBoundingBox {
    type Envelope = rstar::AABB<[f32; 3]>;

    fn envelope(&self) -> Self::Envelope {
        self.aabb
    }
}

impl rstar::PointDistance for RStarBoundingBox {
    fn distance_2(&self, point: &[f32; 3]) -> f32 {
        self.aabb.distance_2(point)
    }
}

#[derive(Clone, Default, Serialize, Deserialize, Resource)]
pub struct AccelerationTree {
    pub tree: rstar::RTree<RStarBoundingBox>,
}

impl Deref for AccelerationTree {
    type Target = rstar::RTree<RStarBoundingBox>;

    fn deref(&self) -> &Self::Target {
        &self.tree
    }
}

impl DerefMut for AccelerationTree {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.tree
    }
}

impl AccelerationTree {
    #[inline]
    pub fn size(&self) -> usize {
        self.tree.size()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.tree.size() == 0
    }

    pub fn load(bounding_boxes: Vec<RStarBoundingBox>) -> Self {
        Self {
            tree: rstar::RTree::bulk_load(bounding_boxes)
        }
    }

    pub fn replace(&mut self, bounding_boxes: Vec<RStarBoundingBox>) {
        self.tree = rstar::RTree::bulk_load(bounding_boxes);
    }

    pub fn query_within_distance<'a>(&'a self, loc: Vec3, distance: f32) -> impl Iterator<Item=(RefU64, [f32; 3])> + 'a {
        self.tree
            .locate_within_distance([loc.x, loc.y, loc.z], distance.powi(2))
            .map(|bb| (bb.refno, bb.aabb.center()))
    }

    pub fn locate_intersecting_bounds<'a>(&'a self, bounds: &Aabb) -> impl Iterator<Item=(RefU64, [f32; 3])> + 'a {
        self.tree
            .locate_in_envelope_intersecting(&rstar::AABB::from_corners([bounds.mins[0], bounds.mins[1], bounds.mins[2]],
                                                                        [bounds.maxs[0], bounds.maxs[1], bounds.maxs[2]]))
            .map(|bb| (bb.refno, bb.aabb.center()))
    }

    pub fn locate_contain_bounds<'a>(&'a self, bounds: &Aabb) -> impl Iterator<Item=RefU64> + 'a {
        self.tree
            .locate_in_envelope(&rstar::AABB::from_corners([bounds.mins[0], bounds.mins[1], bounds.mins[2]],
                                                           [bounds.maxs[0], bounds.maxs[1], bounds.maxs[2]]))
            .map(|bb| bb.refno)
    }

    pub fn serialize_to_bin_file(&self) -> bool {
        let mut file = File::create(format!(r"accel_tree.bin{}", "")).unwrap();
        let serialized = bincode::serialize(&self).unwrap();
        file.write_all(serialized.as_slice()).unwrap();
        true
    }

    // pub fn keys_intersecting_bounds(&self, bounds: AABB) -> Vec<StrokeKey> {
    //     self.0
    //         .locate_in_envelope_intersecting(&rstar::AABB::from_corners(
    //             [bounds.mins[0], bounds.mins[1]],
    //             [bounds.maxs[0], bounds.maxs[1]],
    //         ))
    //         .map(|object| object.data)
    //         .collect()
    // }

    // pub fn locate<'a>(&'a self, ray: &'a Ray) -> impl Iterator<Item = Entity> + 'a {
    //     self.tree
    //         .locate_with_selection_function(ray)
    //         .map(|bb| bb.entity)
    // }
}

