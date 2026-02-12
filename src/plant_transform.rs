//! 自有 Transform 类型，替代外部引擎的 Transform
//!
//! 包含 translation (Vec3), rotation (Quat), scale (Vec3)，
//! 支持 rkyv + serde 序列化。

use glam::{Affine3A, Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};

#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    Serialize,
    Deserialize,
    Clone,
    Copy,
    Debug,
    PartialEq,
)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {
    pub const IDENTITY: Self = Self {
        translation: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    };

    #[inline]
    pub fn from_translation(translation: Vec3) -> Self {
        Self {
            translation,
            ..Self::IDENTITY
        }
    }

    #[inline]
    pub fn from_rotation(rotation: Quat) -> Self {
        Self {
            rotation,
            ..Self::IDENTITY
        }
    }

    #[inline]
    pub fn from_scale(scale: Vec3) -> Self {
        Self {
            scale,
            ..Self::IDENTITY
        }
    }

    /// 从 4x4 矩阵分解为 Transform（scale, rotation, translation）
    #[inline]
    pub fn from_matrix(matrix: Mat4) -> Self {
        let (scale, rotation, translation) = matrix.to_scale_rotation_translation();
        Self {
            translation,
            rotation,
            scale,
        }
    }

    /// 转换为 4x4 仿射变换矩阵
    #[inline]
    pub fn compute_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    /// compute_matrix 的别名（兼容历史 API）
    #[inline]
    pub fn to_matrix(&self) -> Mat4 {
        self.compute_matrix()
    }

    /// 转换为 Affine3A
    #[inline]
    pub fn compute_affine(&self) -> Affine3A {
        Affine3A::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    /// 检查所有分量是否为有限值（兼容旧调用）
    #[inline]
    pub fn is_finite(&self) -> bool {
        self.translation.is_finite() && self.rotation.is_finite() && self.scale.is_finite()
    }

    /// 变换一个点
    #[inline]
    pub fn transform_point(&self, mut point: Vec3) -> Vec3 {
        point = self.scale * point;
        point = self.rotation * point;
        point += self.translation;
        point
    }

    /// 组合两个 Transform（self * transform）
    #[inline]
    pub fn mul_transform(&self, transform: Self) -> Self {
        let translation = self.transform_point(transform.translation);
        let rotation = self.rotation * transform.rotation;
        let scale = self.scale * transform.scale;
        Self {
            translation,
            rotation,
            scale,
        }
    }
}

impl Default for Transform {
    #[inline]
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl std::ops::Mul<Transform> for Transform {
    type Output = Transform;
    #[inline]
    fn mul(self, rhs: Transform) -> Transform {
        self.mul_transform(rhs)
    }
}

impl std::ops::Mul<Vec3> for Transform {
    type Output = Vec3;
    #[inline]
    fn mul(self, rhs: Vec3) -> Vec3 {
        self.transform_point(rhs)
    }
}
