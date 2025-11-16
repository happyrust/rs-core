use crate::RefU64;
use crate::aios_db_mgr::PdmsDataInterface;

/// 获取材料编码，通过分割spre或hstu
///
/// 命名规则为 第二个 / 到 :
pub async fn get_spre_material_code(
    refno: RefU64,
    foreign_name: &str,
    aios_mgr: &dyn PdmsDataInterface,
) -> Option<String> {
    let Ok(Some(spre_attr)) = aios_mgr.get_foreign_attr(refno, foreign_name).await else {
        return None;
    };
    let Some(spre_name) = spre_attr.get_name() else {
        return None;
    };
    let material_code = split_spre_material_code(&spre_name).unwrap_or("".to_string());
    Some(material_code)
}

/// 通过spre name 返回材料编码 命名规则为 第二个 / 到 :
///
/// 例如 "/VMB1/CPP00102:P,50" -> "CPP00102"
pub fn split_spre_material_code(spre_name: &str) -> Option<String> {
    if spre_name.contains(" OF ") {
        return None;
    }
    let spre_name_split = spre_name.split("/").collect::<Vec<_>>();
    if spre_name_split.len() < 3 {
        return None;
    }
    let spre_name_last = spre_name_split[2];
    let split = spre_name_last.split(":").collect::<Vec<_>>();
    if split.len() < 2 {
        return None;
    }
    Some(split[0].to_string())
}
