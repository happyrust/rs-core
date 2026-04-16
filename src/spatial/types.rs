use parry3d::bounding_volume::Aabb;
use parry3d::math::Point;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Point3Dto {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Point3Dto {
    pub fn is_finite(&self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }

    pub fn to_point(self) -> Point<f32> {
        Point::new(self.x, self.y, self.z)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct AabbDto {
    pub min: Point3Dto,
    pub max: Point3Dto,
}

impl AabbDto {
    pub fn is_valid(&self) -> bool {
        self.min.is_finite()
            && self.max.is_finite()
            && self.min.x <= self.max.x
            && self.min.y <= self.max.y
            && self.min.z <= self.max.z
    }

    pub fn try_to_aabb(&self) -> Result<Aabb, String> {
        if !self.is_valid() {
            return Err("bbox 非法：坐标必须为有限数值，且 min 不能大于 max".to_string());
        }
        Ok(Aabb::new(self.min.to_point(), self.max.to_point()))
    }
}

impl From<&Aabb> for AabbDto {
    fn from(value: &Aabb) -> Self {
        Self {
            min: Point3Dto {
                x: value.mins.x,
                y: value.mins.y,
                z: value.mins.z,
            },
            max: Point3Dto {
                x: value.maxs.x,
                y: value.maxs.y,
                z: value.maxs.z,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QueryShape {
    #[default]
    Cube,
    Sphere,
}

fn default_include_self() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SpatialQueryTarget {
    PointRadius {
        center: Point3Dto,
        radius: f32,
        #[serde(default)]
        shape: QueryShape,
    },
    Bbox {
        bbox: AabbDto,
    },
    RefnoNeighborhood {
        refno: String,
        #[serde(default)]
        expand_distance: f32,
        #[serde(default = "default_include_self")]
        include_self: bool,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SpatialQueryFilter {
    #[serde(default)]
    pub nouns: Option<Vec<String>>,
    #[serde(default)]
    pub spec_values: Option<Vec<i64>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SpatialQueryOptions {
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub include_aabb: Option<bool>,
    #[serde(default)]
    pub include_distance: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialQueryRequest {
    pub target: SpatialQueryTarget,
    #[serde(default)]
    pub filter: Option<SpatialQueryFilter>,
    #[serde(default)]
    pub options: Option<SpatialQueryOptions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialQueryItem {
    pub refno: String,
    pub noun: String,
    #[serde(default)]
    pub spec_value: Option<i64>,
    #[serde(default)]
    pub aabb: Option<AabbDto>,
    #[serde(default)]
    pub distance: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialQueryResponse {
    pub success: bool,
    pub items: Vec<SpatialQueryItem>,
    pub total: usize,
    pub truncated: bool,
    #[serde(default)]
    pub query_aabb: Option<AabbDto>,
    pub backend: String,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialStatsResponse {
    pub success: bool,
    pub backend: String,
    pub total_elements: usize,
    pub index_type: String,
    #[serde(default)]
    pub error: Option<String>,
}
