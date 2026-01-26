use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;

/// 预设的 LOD 等级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Ord, PartialOrd)]
pub enum LodLevel {
    #[serde(rename = "L0")]
    L0,
    #[serde(rename = "L1")]
    L1,
    #[serde(rename = "L2")]
    L2,
    #[serde(rename = "L3")]
    L3,
    #[serde(rename = "L4")]
    L4,
}

impl Default for LodLevel {
    fn default() -> Self {
        LodLevel::L2
    }
}

/// CSG 网格生成的细分配置
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct LodMeshSettings {
    #[serde(default = "LodMeshSettings::default_radial_segments")]
    pub radial_segments: u16,
    #[serde(default = "LodMeshSettings::default_height_segments")]
    pub height_segments: u16,
    #[serde(default = "LodMeshSettings::default_cap_segments")]
    pub cap_segments: u16,
    #[serde(default = "LodMeshSettings::default_error_tolerance")]
    pub error_tolerance: f32,
    /// 动态分段的最小圆周段数
    #[serde(default = "LodMeshSettings::default_min_radial_segments")]
    pub min_radial_segments: u16,
    /// 动态分段的最大圆周段数（None 表示不限制）
    #[serde(default)]
    pub max_radial_segments: Option<u16>,
    /// 动态分段的最小高度段数
    #[serde(default = "LodMeshSettings::default_min_height_segments")]
    pub min_height_segments: u16,
    /// 动态分段的最大高度段数（None 表示不限制）
    #[serde(default)]
    pub max_height_segments: Option<u16>,
    /// 目标段长（mm），用于按尺寸自适应计算分段；None 表示关闭
    #[serde(default)]
    pub target_segment_length: Option<f32>,
    /// 不可缩放体的段长调整系数（<1 表示更高精度）
    #[serde(default = "LodMeshSettings::default_non_scalable_factor")]
    pub non_scalable_factor: f32,
}

impl LodMeshSettings {
    const fn default_radial_segments() -> u16 {
        24
    }

    const fn default_height_segments() -> u16 {
        1
    }

    const fn default_cap_segments() -> u16 {
        1
    }

    const fn default_error_tolerance() -> f32 {
        0.002
    }

    const fn default_min_radial_segments() -> u16 {
        8
    }

    const fn default_min_height_segments() -> u16 {
        1
    }

    const fn default_non_scalable_factor() -> f32 {
        1.0
    }
}

impl Default for LodMeshSettings {
    fn default() -> Self {
        Self {
            radial_segments: Self::default_radial_segments(),
            height_segments: Self::default_height_segments(),
            cap_segments: Self::default_cap_segments(),
            error_tolerance: Self::default_error_tolerance(),
            min_radial_segments: Self::default_min_radial_segments(),
            max_radial_segments: None,
            min_height_segments: Self::default_min_height_segments(),
            max_height_segments: None,
            target_segment_length: None,
            non_scalable_factor: Self::default_non_scalable_factor(),
        }
    }
}

impl LodMeshSettings {
    const EPS: f32 = 1e-4;

    pub fn adaptive_radial_segments(
        &self,
        radius: f32,
        circumference: Option<f32>,
        non_scalable: bool,
    ) -> u16 {
        let base = self.radial_segments.max(self.min_radial_segments.max(3));
        let max_allowed = self
            .max_radial_segments
            .unwrap_or(base)
            .max(self.min_radial_segments);

        let effective_radius = radius.abs();
        if effective_radius <= Self::EPS {
            return base;
        }

        if let Some(mut target_len) = self.target_segment_length {
            if non_scalable {
                target_len *= self.non_scalable_factor.max(0.1);
            }
            // 允许 < 1mm 的段长：单位网格（unit mesh）会在实例侧被大倍率缩放，
            // 需要把目标段长映射到单位空间后继续生效。
            target_len = target_len.max(0.001);
            let circumference =
                circumference.unwrap_or(2.0 * std::f32::consts::PI * effective_radius);
            if circumference <= Self::EPS {
                return base;
            }
            let ideal = (circumference / target_len).ceil() as u16;
            ideal
                .max(self.min_radial_segments.max(3))
                .min(max_allowed.max(self.min_radial_segments))
        } else {
            base
        }
    }

    pub fn adaptive_height_segments(&self, span: f32, non_scalable: bool) -> u16 {
        let base = self.height_segments.max(self.min_height_segments.max(1));
        let max_allowed = self
            .max_height_segments
            .unwrap_or(base)
            .max(self.min_height_segments);

        let span = span.abs();
        if span <= Self::EPS {
            return base;
        }

        if let Some(mut target_len) = self.target_segment_length {
            if non_scalable {
                target_len *= self.non_scalable_factor.max(0.1);
            }
            target_len = target_len.max(0.001);
            let ideal = (span / target_len).ceil() as u16;
            ideal
                .max(self.min_height_segments.max(1))
                .min(max_allowed.max(self.min_height_segments))
        } else {
            base
        }
    }
}

/// 单个 LOD 档位对应的精度参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshPrecisionProfile {
    /// 该 LOD 对应的 mesh 子目录（基于 meshes_path）
    #[serde(default)]
    pub output_subdir: Option<String>,
    /// CSG 网格生成参数
    #[serde(default)]
    pub csg_settings: LodMeshSettings,
}

impl Default for MeshPrecisionProfile {
    fn default() -> Self {
        Self {
            output_subdir: None,
            csg_settings: LodMeshSettings::default(),
        }
    }
}

/// 精度配置集合
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshPrecisionSettings {
    #[serde(default)]
    pub default_lod: LodLevel,
    #[serde(default = "MeshPrecisionSettings::default_lod_profiles")]
    pub lod_profiles: HashMap<LodLevel, MeshPrecisionProfile>,
    #[serde(default)]
    pub overrides: PrecisionOverrides,
    #[serde(default)]
    pub non_scalable_geo_types: Vec<String>,
}

impl Default for MeshPrecisionSettings {
    fn default() -> Self {
        Self {
            default_lod: LodLevel::L2,
            lod_profiles: Self::default_lod_profiles(),
            overrides: PrecisionOverrides::default(),
            non_scalable_geo_types: Vec::new(),
        }
    }
}

impl MeshPrecisionSettings {
    fn default_lod_profiles() -> HashMap<LodLevel, MeshPrecisionProfile> {
        let mut map = HashMap::new();
        map.insert(
            LodLevel::L0,
            MeshPrecisionProfile {
                output_subdir: Some("lod_L0".to_string()),
                csg_settings: LodMeshSettings {
                    radial_segments: 12,
                    height_segments: 1,
                    cap_segments: 1,
                    error_tolerance: 0.01,
                    min_radial_segments: 6,
                    max_radial_segments: Some(16),
                    min_height_segments: 1,
                    max_height_segments: Some(2),
                    target_segment_length: Some(200.0),
                    non_scalable_factor: 0.9,
                },
            },
        );
        map.insert(
            LodLevel::L1,
            MeshPrecisionProfile {
                output_subdir: Some("lod_L1".to_string()),
                csg_settings: LodMeshSettings {
                    radial_segments: 24,
                    height_segments: 1,
                    cap_segments: 1,
                    error_tolerance: 0.008,
                    min_radial_segments: 8,
                    max_radial_segments: Some(64),
                    min_height_segments: 1,
                    max_height_segments: Some(2),
                    target_segment_length: Some(150.0),
                    non_scalable_factor: 0.85,
                },
            },
        );
        let mut default_l2 = MeshPrecisionProfile::default();
        default_l2.output_subdir = None;
        default_l2.csg_settings = LodMeshSettings {
            radial_segments: 24,
            height_segments: 2,
            cap_segments: 1,
            error_tolerance: 0.004,
            min_radial_segments: 12,
            max_radial_segments: Some(96),
            min_height_segments: 1,
            max_height_segments: Some(3),
            target_segment_length: Some(100.0),
            non_scalable_factor: 0.75,
        };
        map.insert(LodLevel::L2, default_l2);
        map.insert(
            LodLevel::L3,
            MeshPrecisionProfile {
                output_subdir: Some("lod_L3".to_string()),
                csg_settings: LodMeshSettings {
                    radial_segments: 32,
                    height_segments: 3,
                    cap_segments: 1,
                    error_tolerance: 0.002,
                    min_radial_segments: 16,
                    max_radial_segments: Some(128),
                    min_height_segments: 2,
                    max_height_segments: Some(5),
                    target_segment_length: Some(70.0),
                    non_scalable_factor: 0.7,
                },
            },
        );
        map.insert(
            LodLevel::L4,
            MeshPrecisionProfile {
                output_subdir: Some("lod_L4".to_string()),
                csg_settings: LodMeshSettings {
                    radial_segments: 48,
                    height_segments: 4,
                    cap_segments: 1,
                    error_tolerance: 0.001,
                    min_radial_segments: 24,
                    max_radial_segments: Some(192),
                    min_height_segments: 3,
                    max_height_segments: Some(8),
                    target_segment_length: Some(40.0),
                    non_scalable_factor: 0.65,
                },
            },
        );
        map
    }

    /// 根据上下文选择合适的精度档位
    pub fn profile_for(
        &self,
        noun: Option<&str>,
        geo_type: Option<&str>,
        refno: Option<&str>,
    ) -> MeshPrecisionProfile {
        let lod = refno
            .and_then(|r| self.overrides.refno_lod.get(r))
            .or_else(|| noun.and_then(|n| self.overrides.noun_lod.get(n)))
            .or_else(|| geo_type.and_then(|g| self.overrides.geo_type_lod.get(g)))
            .copied()
            .unwrap_or(self.default_lod);

        self.lod_profiles
            .get(&lod)
            .cloned()
            .unwrap_or_else(MeshPrecisionProfile::default)
    }

    /// 仅通过几何类型获取档位
    pub fn profile_for_geo(&self, geo_type: &str) -> MeshPrecisionProfile {
        self.profile_for(None, Some(geo_type), None)
    }

    /// 获取指定 LOD 的 mesh 子目录（若未配置则返回 None）
    pub fn output_subdir(&self, lod: LodLevel) -> Option<&str> {
        self.lod_profiles
            .get(&lod)
            .and_then(|profile| profile.output_subdir.as_deref())
            .filter(|subdir| !subdir.is_empty())
    }

    /// 获取指定 LOD 的 CSG 网格设置
    pub fn lod_settings(&self, lod: LodLevel) -> LodMeshSettings {
        self.lod_profiles
            .get(&lod)
            .map(|profile| profile.csg_settings)
            .unwrap_or_else(LodMeshSettings::default)
    }

    /// 判断几何类型是否属于不可缩放集合
    pub fn is_non_scalable_geo(&self, geo_type: &str) -> bool {
        self.non_scalable_geo_types
            .iter()
            .any(|name| name == geo_type)
    }
}

/// 按类型的覆盖设定
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PrecisionOverrides {
    #[serde(default)]
    pub noun_lod: HashMap<String, LodLevel>,
    #[serde(default)]
    pub geo_type_lod: HashMap<String, LodLevel>,
    #[serde(default)]
    pub refno_lod: HashMap<String, LodLevel>,
}

static ACTIVE_PRECISION: Lazy<RwLock<MeshPrecisionSettings>> =
    Lazy::new(|| RwLock::new(MeshPrecisionSettings::default()));

/// 更新全局精度配置
pub fn set_active_precision(settings: MeshPrecisionSettings) {
    if let Ok(mut guard) = ACTIVE_PRECISION.write() {
        *guard = settings;
    }
}

/// 获取当前生效的精度设置副本
pub fn active_precision() -> MeshPrecisionSettings {
    ACTIVE_PRECISION
        .read()
        .map(|guard| guard.clone())
        .unwrap_or_else(|_| MeshPrecisionSettings::default())
}

/// 获取指定几何类型的精度档位
pub fn active_profile_for_geo(geo_type: &str) -> MeshPrecisionProfile {
    ACTIVE_PRECISION
        .read()
        .map(|guard| guard.profile_for_geo(geo_type))
        .unwrap_or_else(|_| MeshPrecisionProfile::default())
}
