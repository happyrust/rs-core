use crate::*;

/// 创建带有 BANG 属性的测试属性映射
pub fn create_test_attmap_with_bang(bang_value: f32) -> NamedAttrMap {
    let mut att = NamedAttrMap::new("TEST");
    att.insert(
        "BANG".to_string(),
        AttrVal::DoubleType(bang_value as f64).into(),
    );
    att.insert(
        "NOUN".to_string(),
        AttrVal::StringType("TEST".to_string()).into(),
    );
    att
}

/// 创建带有 ZDIS 属性的测试属性映射
pub fn create_test_attmap_with_zdis(zdis_value: f32, pkdi_value: f32) -> NamedAttrMap {
    let mut att = NamedAttrMap::new("TEST");
    att.insert(
        "ZDIS".to_string(),
        AttrVal::DoubleType(zdis_value as f64).into(),
    );
    att.insert(
        "PKDI".to_string(),
        AttrVal::DoubleType(pkdi_value as f64).into(),
    );
    att.insert(
        "NOUN".to_string(),
        AttrVal::StringType("TEST".to_string()).into(),
    );
    att
}

/// 创建带有 YDIR 属性的测试属性映射
pub fn create_test_attmap_with_ydir(ydir_value: glam::DVec3) -> NamedAttrMap {
    let mut att = NamedAttrMap::new("TEST");
    let ydir_f64 = [ydir_value.x, ydir_value.y, ydir_value.z];
    att.insert("YDIR".to_string(), AttrVal::Vec3Type(ydir_f64).into());
    att.insert(
        "NOUN".to_string(),
        AttrVal::StringType("TEST".to_string()).into(),
    );
    att
}

/// 创建带有位置属性的测试属性映射
pub fn create_test_attmap_with_position(pos: glam::DVec3) -> NamedAttrMap {
    let mut att = NamedAttrMap::new("TEST");
    let pos_f64 = [pos.x, pos.y, pos.z];
    att.insert("NPOS".to_string(), AttrVal::Vec3Type(pos_f64).into());
    att.insert(
        "NOUN".to_string(),
        AttrVal::StringType("TEST".to_string()).into(),
    );
    att
}

/// 创建完整的测试属性映射
pub fn create_complete_test_attmap() -> NamedAttrMap {
    let mut att = NamedAttrMap::new("TEST");

    // 基本属性
    att.insert(
        "NOUN".to_string(),
        AttrVal::StringType("TEST".to_string()).into(),
    );
    let pos_f64 = [1.0, 2.0, 3.0];
    att.insert("NPOS".to_string(), AttrVal::Vec3Type(pos_f64).into());

    // 方向属性
    let ydir_f64 = [0.0, 1.0, 0.0];
    att.insert("YDIR".to_string(), AttrVal::Vec3Type(ydir_f64).into());
    let opdi_f64 = [1.0, 0.0, 0.0];
    att.insert("OPDI".to_string(), AttrVal::Vec3Type(opdi_f64).into());

    // 旋转属性
    att.insert("BANG".to_string(), AttrVal::DoubleType(45.0).into());

    // 偏移属性
    att.insert("ZDIS".to_string(), AttrVal::DoubleType(100.0).into());
    att.insert("PKDI".to_string(), AttrVal::DoubleType(0.0).into());

    // 其他属性
    let delp_f64 = [0.5, 0.5, 0.5];
    att.insert("DELP".to_string(), AttrVal::Vec3Type(delp_f64).into());
    att.insert(
        "POSL".to_string(),
        AttrVal::StringType("".to_string()).into(),
    );

    att
}

/// 创建通用的测试属性映射
pub fn create_test_attmap_with_attributes() -> NamedAttrMap {
    let mut att = NamedAttrMap::new("ENDATU");

    // 基本属性
    att.insert(
        "NOUN".to_string(),
        AttrVal::StringType("ENDATU".to_string()).into(),
    );
    let pos_f64 = [0.0, 0.0, 0.0];
    att.insert("NPOS".to_string(), AttrVal::Vec3Type(pos_f64).into());

    // 设置所有者
    att.insert(
        "OWNER".to_string(),
        AttrVal::StringType("test_parent".to_string()).into(),
    );

    att
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_attmap() {
        let att = create_test_attmap_with_bang(45.0);
        assert_eq!(att.get_f32("BANG"), Some(45.0));
        assert_eq!(att.get_str("NOUN"), Some("TEST"));
    }

    #[test]
    fn test_create_complete_test_attmap() {
        let att = create_complete_test_attmap();
        assert_eq!(att.get_str("NOUN"), Some("TEST"));
        assert_eq!(att.get_f32("BANG"), Some(45.0));
        assert_eq!(att.get_f32("ZDIS"), Some(100.0));
        assert_eq!(att.get_dvec3("NPOS"), Some(glam::DVec3::new(1.0, 2.0, 3.0)));
    }
}
