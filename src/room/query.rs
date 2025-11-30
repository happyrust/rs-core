use crate::shape::pdms_shape::PlantMesh;
use crate::{RefU64, RefnoEnum, SUL_DB, query_insts};
use glam::Vec3;
use nalgebra::Point3;
use parry3d::bounding_volume::Aabb;
use parry3d::math::Isometry;
use parry3d::query::PointQuery;
use parry3d::shape::TriMeshFlags;

#[cfg(all(not(target_arch = "wasm32"), feature = "sqlite"))]
use crate::spatial::sqlite;
#[cfg(all(not(target_arch = "wasm32"), feature = "sqlite"))]
use anyhow::Context;

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
pub async fn query_room_panel_by_point(point: Vec3) -> anyhow::Result<Option<RefnoEnum>> {
    let candidates =
        tokio::task::spawn_blocking(move || sqlite::query_containing_point(point, 256))
            .await
            .context("查询房间空间索引失败")??;
    if candidates.is_empty() {
        return Ok(None);
    }
    let refnos: Vec<RefnoEnum> = candidates
        .iter()
        .map(|(refno, _)| RefnoEnum::Refno(*refno))
        .collect();
    let insts = query_insts(&refnos, true).await?;
    let pt: Point3<f32> = point.into();
    let parry_pt = parry3d::math::Point::new(pt.x, pt.y, pt.z);

    for (refno, aabb) in candidates {
        if aabb.mins.x > 1_000_000.0 {
            continue;
        }
        let Some(geom_inst) = insts.iter().find(|x| x.refno.refno() == refno) else {
            continue;
        };
        for inst in &geom_inst.insts {
            // 使用配置路径和 L0 最低精度 LOD
            use crate::utils::lod_path_detector::build_mesh_path;
            let mesh_path = crate::get_db_option()
                .get_meshes_path()
                .join(build_mesh_path(&inst.geo_hash, "L0"));

            let Ok(mesh) = PlantMesh::des_mesh_file(&mesh_path) else {
                continue;
            };
            let Some(mut tri_mesh) = mesh.get_tri_mesh_with_flag(
                (geom_inst.world_trans * &inst.transform).to_matrix(),
                TriMeshFlags::ORIENTED,
            ) else {
                continue;
            };
            if tri_mesh.contains_point(&Isometry::identity(), &parry_pt) {
                return Ok(Some(RefnoEnum::Refno(refno)));
            }
        }
    }

    Ok(None)
}
