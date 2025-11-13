/// LOD 路径检测工具模块
/// 
/// 提供智能的网格文件路径检测功能，支持多种LOD目录结构和文件命名格式

/// 智能检测并获取最佳的网格文件路径
/// 
/// # 参数
/// * `geo_hash` - 几何体哈希值
/// * `lod_level` - LOD 等级，如 "L1", "L2", "L3"，空字符串表示无LOD
/// 
/// # 返回值
/// 返回指定LOD等级的路径，如果不存在则返回默认路径
/// 
/// # 支持的路径格式
/// 1. 新结构：`meshes/lod_{lod_level}/{geo_hash}_{lod_level}.mesh`
/// 2. 新结构（无后缀）：`meshes/lod_{lod_level}/{geo_hash}.mesh`
/// 3. 兼容结构：`meshes/lod_L3/lod_{lod_level}/{geo_hash}_{lod_level}.mesh`
/// 4. 兼容结构（无后缀）：`meshes/lod_L3/lod_{lod_level}/{geo_hash}.mesh`
/// 5. 默认结构：`meshes/{geo_hash}.mesh`
/// 
/// # 检测策略
/// - 有指定LOD时：严格按照指定LOD等级访问，不回退到其他级别
/// - 无指定LOD时：使用默认路径
pub fn detect_best_mesh_path(geo_hash: &str, lod_level: &str) -> String {
    // 构建候选路径列表，按优先级排序
    let mut candidate_paths = Vec::new();
    
    if lod_level.is_empty() {
        // 无LOD配置时，使用默认路径
        candidate_paths.push(format!("meshes/{}.mesh", geo_hash));
    } else {
        // 有指定LOD时，严格按照指定级别访问，不回退
        // 指定级别的新结构
        candidate_paths.push(format!("meshes/lod_{}/{}_{}.mesh", lod_level, geo_hash, lod_level));
        candidate_paths.push(format!("meshes/lod_{}/{}.mesh", lod_level, geo_hash));
        
        // 指定级别的兼容结构
        candidate_paths.push(format!("meshes/lod_L3/lod_{}/{}_{}.mesh", lod_level, geo_hash, lod_level));
        candidate_paths.push(format!("meshes/lod_L3/lod_{}/{}.mesh", lod_level, geo_hash));
        
        // 最后尝试默认路径作为回退
        candidate_paths.push(format!("meshes/{}.mesh", geo_hash));
    }

    // 返回第一个存在的路径
    for path in &candidate_paths {
        if std::path::Path::new(path).exists() {
            return path.clone();
        }
    }
    
    // 所有候选路径都不存在，返回第一个尝试的路径（这样会触发错误日志）
    candidate_paths.first().unwrap().clone()
}

/// 获取指定LOD等级的所有可能网格文件路径候选列表
/// 
/// # 参数
/// * `geo_hash` - 几何体哈希值
/// * `lod_level` - LOD 等级
/// 
/// # 返回值
/// 返回指定LOD等级的所有可能路径候选列表，按优先级排序
pub fn get_mesh_path_candidates(geo_hash: &str, lod_level: &str) -> Vec<String> {
    let mut candidate_paths = Vec::new();
    
    if lod_level.is_empty() {
        candidate_paths.push(format!("meshes/{}.mesh", geo_hash));
    } else {
        // 指定级别的新结构
        candidate_paths.push(format!("meshes/lod_{}/{}_{}.mesh", lod_level, geo_hash, lod_level));
        candidate_paths.push(format!("meshes/lod_{}/{}.mesh", lod_level, geo_hash));
        
        // 指定级别的兼容结构
        candidate_paths.push(format!("meshes/lod_L3/lod_{}/{}_{}.mesh", lod_level, geo_hash, lod_level));
        candidate_paths.push(format!("meshes/lod_L3/lod_{}/{}.mesh", lod_level, geo_hash));
        
        // 默认路径作为最后回退
        candidate_paths.push(format!("meshes/{}.mesh", geo_hash));
    }
    
    candidate_paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_best_mesh_path_no_lod() {
        let path = detect_best_mesh_path("test_hash", "");
        assert!(path.contains("test_hash"));
    }

    #[test]
    fn test_detect_best_mesh_path_with_lod() {
        let path = detect_best_mesh_path("test_hash", "L2");
        // 应该包含L2或默认路径，不会包含L1或L3
        assert!(path.contains("test_hash"));
        assert!(path.contains("L2") || path == "meshes/test_hash.mesh");
        // 确保不会回退到其他LOD级别
        assert!(!path.contains("L1"));
        assert!(!path.contains("L3"));
    }

    #[test]
    fn test_get_mesh_path_candidates() {
        let candidates = get_mesh_path_candidates("test_hash", "L1");
        // 应该只有L1相关的路径和默认路径
        assert_eq!(candidates.len(), 5); // L1新结构(2) + L1兼容结构(2) + 默认(1)
        
        // 验证所有路径要么包含L1，要么是默认路径
        for candidate in &candidates {
            assert!(candidate.contains("L1") || candidate == "meshes/test_hash.mesh");
        }
    }
}
