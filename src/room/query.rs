use crate::{RefnoEnum, SUL_DB};
use glam::Vec3;

#[cfg(all(not(target_arch = "wasm32"), feature = "sqlite"))]
pub async fn query_room_number_by_point(point: Vec3) -> anyhow::Result<Option<String>> {
    let Some(refno) = query_room_panel_by_point(point).await? else {
        return Ok(None);
    };
    let mut response = SUL_DB
        .query(format!(
            r#"
            select value room_num from only {}<-room_panel_relate limit 1;
        "#,
            refno.to_pe_key()
        ))
        .await?;
    // dbg!(&response);
    let room_number: Option<String> = response.take(0)?;
    Ok(room_number)
}

//传进来的是世界坐标系下的点
#[cfg(all(not(target_arch = "wasm32"), feature = "sqlite"))]
pub async fn query_room_panel_by_point(_point: Vec3) -> anyhow::Result<Option<RefnoEnum>> {
    // 磁盘 .mesh 几何读取路径已下线（无写入方，rkyv unchecked 反序列化存在脏读/UB 风险）。
    // 该点-面片包含判定依赖磁盘 mesh；固化为“无磁盘几何可用 → 不命中”，与下线前新环境
    // （.mesh 文件恒不存在 → 逐候选读取失败跳过 → 返回 None）行为完全一致。
    Ok(None)
}
