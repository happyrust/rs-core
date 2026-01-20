/// LOD 路径拼接工具模块
///
/// 提供简单的网格文件路径拼接功能

/// 拼接指定LOD等级的网格文件路径
///
/// # 参数
/// * `geo_hash` - 几何体哈希值
/// * `lod_level` - LOD 等级，如 "L1", "L2", "L3"，空字符串表示无LOD
/// * `manifold` - 是否为 manifold 格式（用于布尔运算），如果是则添加 `_m` 后缀
///
/// # 返回值
/// - 普通 mesh: `meshes/lod_{lod_level}/{geo_hash}_{lod_level}.mesh`
/// - Manifold mesh: `meshes/lod_{lod_level}/{geo_hash}_{lod_level}_m.mesh`
/// - 无 LOD: `meshes/{geo_hash}.mesh` 或 `meshes/{geo_hash}_m.mesh`
pub fn build_mesh_path(geo_hash: &str, lod_level: &str, manifold: bool) -> String {
    let m_suffix = if manifold { "_m" } else { "" };
    if lod_level.is_empty() {
        format!("meshes/{}{}.mesh", geo_hash, m_suffix)
    } else {
        format!("meshes/lod_{}/{}_{}{}.mesh", lod_level, geo_hash, lod_level, m_suffix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_mesh_path_no_lod() {
        let path = build_mesh_path("test_hash", "", false);
        assert_eq!(path, "meshes/test_hash.mesh");
    }

    #[test]
    fn test_build_mesh_path_with_lod() {
        let path = build_mesh_path("1", "L1", false);
        assert_eq!(path, "meshes/lod_L1/1_L1.mesh");

        let path = build_mesh_path("test_hash", "L2", false);
        assert_eq!(path, "meshes/lod_L2/test_hash_L2.mesh");
    }

    #[test]
    fn test_build_mesh_path_manifold() {
        let path = build_mesh_path("1", "L1", true);
        assert_eq!(path, "meshes/lod_L1/1_L1_m.mesh");

        let path = build_mesh_path("test_hash", "", true);
        assert_eq!(path, "meshes/test_hash_m.mesh");
    }
}
