/// LOD 路径拼接工具模块
///
/// 提供简单的网格文件路径拼接功能

/// 拼接指定LOD等级的网格文件路径
///
/// # 参数
/// * `geo_hash` - 几何体哈希值
/// * `lod_level` - LOD 等级，如 "L1", "L2", "L3"，空字符串表示无LOD
///
/// # 返回值
/// 返回拼接的路径：`meshes/lod_{lod_level}/{geo_hash}_{lod_level}.mesh`
/// 如果 lod_level 为空，返回：`meshes/{geo_hash}.mesh`
pub fn build_mesh_path(geo_hash: &str, lod_level: &str) -> String {
    if lod_level.is_empty() {
        format!("meshes/{}.mesh", geo_hash)
    } else {
        format!("meshes/lod_{}/{}_{}.mesh", lod_level, geo_hash, lod_level)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[test]
    fn test_build_mesh_path_no_lod() {
        let path = build_mesh_path("test_hash", "");
        assert_eq!(path, "meshes/test_hash.mesh");
    }

    #[test]
    fn test_build_mesh_path_with_lod() {
        let path = build_mesh_path("1", "L1");
        assert_eq!(path, "meshes/lod_L1/1_L1.mesh");

        let path = build_mesh_path("test_hash", "L2");
        assert_eq!(path, "meshes/lod_L2/test_hash_L2.mesh");
    }
}
