use crate::tool::dir_tool::parse_ori_str_to_quat;
use crate::tool::direction_parse::parse_expr_to_dir;
use crate::{room::room::load_aabb_tree, rs_surreal, tool::math_tool};
use glam::{DMat3, DQuat, DVec3, Mat3, Quat, Vec3};
use std::sync::Arc;
use surrealdb::sql::Thing;

fn test_print_ori(ori: &str) {
    let rotation = parse_ori_str_to_quat(ori).unwrap_or(glam::Quat::IDENTITY);
    dbg!(Mat3::from_quat(rotation));
    dbg!(math_tool::quat_to_pdms_ori_xyz_str(&rotation, false));
}

// fn test_cal_ori(v: DVec3) {
//     let ref_dir = if v.dot(DVec3::NEG_Z).abs() > 0.999 {
//         DVec3::NEG_Y
//     }else{
//         DVec3::NEG_Z
//     };
//     let y_dir = v.cross(ref_dir).normalize();
//     let x_dir = y_dir.cross(v).normalize();
//     let rotation = DQuat::from_mat3(&DMat3::from_cols(x_dir.into(), y_dir.into(), v.into()));
//     dbg!((x_dir, y_dir, v));
//     dbg!(math_tool::quat_to_pdms_ori_xyz_str(&rotation.as_quat()));
// }

#[cfg(test)]
mod test_transform {
    use crate::{init_test_surreal};
    use crate::tool::dir_tool::{parse_ori_str_to_mat, parse_ori_str_to_quat};
    use crate::tool::math_tool;
    use crate::tool::math_tool::{
        cal_quat_by_zdir_with_xref, dquat_to_pdms_ori_xyz_str, to_pdms_dvec_str, vec3_to_xyz_str,
    };
    use crate::{cal_ori_by_extru_axis, cal_ori_by_z_axis_ref_x, rs_surreal, RefU64};
    use bevy_reflect::Array;
    use glam::{DMat3, DVec3, Mat3};
    use crate::tool::direction_parse::parse_expr_to_dir;

    //sctn 等等
    #[test]
    fn test_cal_ori_by_line_axis() {
        //Y方向始终在X Z 方向
        let tests = [
            "Y is X and Z is Z",  // X is -Y
            "Y is X and Z is -Z", // X is Y
            "Y is Z and Z is Y",
            "Y is Z and Z is -Y",
            "Y is Z and Z is X",
            "Y is Z and Z is -X",
            "Y is X 45 Z and Z is -Y 45 Z",
            "Y is X 45 Z and Z is Y 45 Z",
            "Y is X 45 Z and Z is X 45 -Z",
            "Y is -X 45 Z and Z is -X 45 -Z",
            "Y is Z and Z is -Y 31 -X",
        ];
        let oris = tests
            .into_iter()
            .map(|x| parse_ori_str_to_mat(x).unwrap())
            .collect::<Vec<_>>();
        // dbg!(&oris);

        for ori in oris {
            let extru_dir = ori.z_axis.as_dvec3();
            // dbg!(to_pdms_dvec_str(&extru_dir));
            let quat = cal_ori_by_extru_axis(extru_dir, false);
            // dbg!(dquat_to_pdms_ori_xyz_str(&quat));
        }
    }

    async fn test_ori(refno: RefU64, assert_ori: &str) {
        let transform = rs_surreal::get_world_mat4(refno, false)
            .await
            .unwrap()
            .unwrap();
        let (scale, rot, translation) = transform.to_scale_rotation_translation();

        dbg!(translation);
        //如果包含其中的任意一个，则不需要转成XYZ
        let convert_xyz = ["E", "N", "U", "W", "S", "D"]
            .into_iter()
            .any(|x| assert_ori.contains(x));
        let ori_str = dquat_to_pdms_ori_xyz_str(&rot, !convert_xyz);
        dbg!(&ori_str);
        assert_eq!(ori_str, assert_ori);
    }

    async fn test_transform(refno: RefU64, assert_ori: &str, pos_str: &str) {
        let transform = rs_surreal::get_world_mat4(refno, false)
            .await
            .unwrap()
            .unwrap();
        let (scale, rot, translation) = transform.to_scale_rotation_translation();

        // dbg!(translation);
        //如果包含其中的任意一个，则不需要转成XYZ
        let convert_xyz = ["E", "N", "U", "W", "S", "D"]
            .into_iter()
            .any(|x| assert_ori.contains(x));
        let ori_str = dquat_to_pdms_ori_xyz_str(&rot, !convert_xyz);
        // dbg!(&ori_str);
        // dbg!(math_tool::dvec3_to_xyz_str(translation));
        if !assert_ori.is_empty() {
            assert_eq!(ori_str, assert_ori);
        }
        if !pos_str.is_empty() {
            assert_eq!(math_tool::dvec3_to_xyz_str(translation), pos_str);
        }
    }

    #[tokio::test]
    async fn test_query_transform_DRNS_DRNE() -> anyhow::Result<()> {
        init_test_surreal().await;
        let refno = "17496/202374".into();
        let mat = crate::get_world_mat4(refno, true).await?.unwrap();
        let spine_att = crate::get_named_attmap(refno).await?;
        let drns = spine_att.get_dvec3("DRNS").unwrap();
        let drne = spine_att.get_dvec3("DRNE").unwrap();

        let mat_inv = mat.inverse();
        let local_drns = mat_inv.transform_vector3(drns);
        let local_drne = mat_inv.transform_vector3(drne);
        // dbg!(to_pdms_dvec_str(&local_drns));
        // dbg!(to_pdms_dvec_str(&local_drne));

        let angle_x = (local_drns.x / local_drns.z).atan();
        let angle_y = (local_drns.y / local_drns.z).atan();
        let scale_drns = DVec3::new(1.0 / angle_x.cos(), 1.0 / angle_y.cos(), 1.0);

        let angle_x = (local_drne.x / local_drne.z).atan();
        let angle_y = (local_drne.y / local_drne.z).atan();
        let scale_drne = DVec3::new(1.0 / angle_x.cos(), 1.0 / angle_y.cos(), 1.0);

        dbg!(scale_drns);
        dbg!(scale_drne);

        Ok(())
    }

    #[tokio::test]
    async fn test_query_transform_ENDATU() -> anyhow::Result<()> {
        init_test_surreal().await;

        //todo fix
        test_ori("17496/268348".into(), "Y is Y 0.374 -X 0.345 -Z and Z is -X 0.693 -Y 42.739 -Z").await;
        // test_ori("17496/273497".into(), "Y is X and Z is Z").await;
        test_ori("24384/25783".into(), "Y is X 89.969 Z and Z is -X 0.031 Z").await;

        Ok(())
    }

    #[tokio::test]
    async fn test_query_transform_SNODE() -> anyhow::Result<()> {
        init_test_surreal().await;

        test_transform(
            "24383/93573".into(),
            "",
            "X 10492.213mm Y 24025.362mm Z 12560mm",
        )
        .await;
        // let w1 = parse_ori_str_to_quat("Y is X 21 -Y and Z is Z").unwrap().as_dquat();
        // let w2 = parse_ori_str_to_quat("Y is -Y and Z is X").unwrap().as_dquat();
        // let w3 = parse_ori_str_to_quat(" Y is X and Z is Y").unwrap().as_dquat();
        // let w4 = parse_ori_str_to_quat(" Y is -X and Z is Z").unwrap().as_dquat();
        // dbg!(w4);
        // let w = w1 * w2;
        // let nw  = w3 * w2.inverse();
        // // dbg!(dquat_to_pdms_ori_xyz_str(&w, true));
        // dbg!(dquat_to_pdms_ori_xyz_str(&(w4 * w2), true));
        // dbg!(dquat_to_pdms_ori_xyz_str(&nw, true));
        Ok(())
    }

    #[tokio::test]
    async fn test_query_transform_SCOJ() -> anyhow::Result<()> {
        init_test_surreal().await;
        //如果是SCOJ, 需要获取两边的连接点，组合出来的方向位置

        // test_ori("24383/93574".into(), "Y is -Y 21 -X and Z is X 21 -Y").await;
        test_transform("24383/80522".into(), "Y is -X 41 Y and Z is -Y 41 -X", "").await;
        Ok(())
    }

    #[tokio::test]
    async fn test_query_transform_CMFI() -> anyhow::Result<()> {
        init_test_surreal().await;
        //如果是SCOJ, 需要获取两边的连接点，组合出来的方向位置

        test_transform(
            "17496/140425".into(),
            "Y is -X 22.452 -Y and Z is Y 22.452 -X",
            "X 3018.841mm Y -7725.884mm Z 900mm",
        )
        .await;
        Ok(())
    }

    #[test]
    fn get_cal_ori() {
        use crate::tool::direction_parse::parse_expr_to_dir;
        let zdir = parse_expr_to_dir("Y 7.2448 X").unwrap();
        let rot = cal_quat_by_zdir_with_xref(zdir);
        // dbg!(dquat_to_pdms_ori_xyz_str(&rot, true));
        assert_eq!(
            dquat_to_pdms_ori_xyz_str(&rot, true),
            "Y is -X 7.245 Y and Z is Y 7.245 X"
        );

        // return;
        let zdir = parse_expr_to_dir("-Z").unwrap();
        let rot = cal_quat_by_zdir_with_xref(zdir);
        // dbg!(dquat_to_pdms_ori_xyz_str(&rot, true));
        assert_eq!(dquat_to_pdms_ori_xyz_str(&rot, true), "Y is Y and Z is -Z");

        let zdir = parse_expr_to_dir("-Y 84.452 -Z").unwrap();
        let rot = cal_quat_by_zdir_with_xref(zdir);
        // dbg!(dquat_to_pdms_ori_xyz_str(&rot, true));
        assert_eq!(
            dquat_to_pdms_ori_xyz_str(&rot, true),
            "Y is Y 5.548 -Z and Z is -Y 84.452 -Z"
        );

        let zdir = parse_expr_to_dir("-Y 84.452 Z").unwrap();
        let rot = cal_quat_by_zdir_with_xref(zdir);
        // dbg!(dquat_to_pdms_ori_xyz_str(&rot, true));
        assert_eq!(
            dquat_to_pdms_ori_xyz_str(&rot, true),
            "Y is Y 5.548 Z and Z is -Y 84.452 Z"
        );

        // let zdir = parse_expr_to_dir("-Y 84.452 -Z").unwrap();
        // let rot = crate::cal_ori_by_z_axis_ref_y(zdir, );
        // dbg!(dquat_to_pdms_ori_xyz_str(&rot, true));
        // let assert_str = "Y is Y 5.548 -Z and Z is -Y 84.452 -Z";
        //
        // let dir_y = parse_expr_to_dir("-X 7.2448 Y").unwrap();
        // let dir_z = parse_expr_to_dir("Y 7.2448 X").unwrap();
        // let dir_x = dir_y.cross(dir_z).normalize();
        // dbg!(to_pdms_dvec_str(&dir_x, true));
        // let mat3 = DMat3::from_quat(parse_ori_str_to_quat("Y is Y 5.548 Z and Z is -Y 84.452 Z").unwrap().as_dquat());
        // dbg!(mat3);
        // //Y is -Y 5.548 Z and Z is -Y 84.452 -Z
        let mat3 = DMat3::from_quat(
            parse_ori_str_to_quat("Y is X 89.969 Z and Z is -X 0.0307 Z")
                .unwrap()
                .as_dquat(),
        );
        dbg!(mat3);
    }

    #[tokio::test]
    async fn test_query_transform_SBFI() -> anyhow::Result<()> {
        init_test_surreal().await;
        //如果是SCOJ, 需要获取两边的连接点，组合出来的方向位置
        //
        test_transform(
            "17496/140426".into(),
            "Y is X 28.02 Y and Z is -Y 28.02 X",
            "X 3018.841mm Y -7725.884mm Z 900mm",
        )
        .await;
        //17496/140428
        test_transform(
            "17496/140428".into(),
            "Y is -X 25 Y and Z is Y 25 X",
            "X 8964.131mm Y -6883.685mm Z -20mm",
        )
        .await;
        let m1 = parse_ori_str_to_quat("Y is -Y 5.548 Z and Z is -Y 84.452 -Z")
            .unwrap()
            .as_dquat();
        // let m2 = parse_ori_str_to_quat("Y is -X 22.452 -Y and Z is Y 22.452 -X").unwrap().as_dquat();
        // let m3 = m2 * m1;
        // println!("matrix: {}", dquat_to_pdms_ori_xyz_str(&m3, true));
        // let m1 = parse_ori_str_to_quat("Y is -Y 5.548 Z and Z is -Y 84.452 -Z").unwrap().as_dquat();
        Ok(())
    }

    #[tokio::test]
    async fn test_query_transform_PJOI() -> anyhow::Result<()> {
        init_test_surreal().await;
        // SJOI 里的CUTP，如果有CUTP，x轴必须为Z轴，如果CUTP轴为(-)Z轴了，z轴改为X，

        // test_ori("24381/178547".into(), "Y is Y 27.581 X and Z is -X 27.581 Y").await;
        test_transform("24381/178550".into(), "Y is -X 27.581 Y and Z is -Y 27.581 -X", "X 12850.226mm Y 24922.073mm Z -4194.68mm").await;
        // let m1 = parse_ori_str_to_quat("Y is X and Z is Y").unwrap();
        // let m2 = parse_ori_str_to_quat("Y is -Y 27.581 -X and Z is Z").unwrap();
        // let m3 = m2 * m1;
        // dbg!(dquat_to_pdms_ori_xyz_str(&m3.as_dquat(), true));
        Ok(())
    }

    #[tokio::test]
    async fn test_query_transform_SJOI() -> anyhow::Result<()> {
        init_test_surreal().await;
        // SJOI 里的CUTP，如果有CUTP，x轴必须为Z轴，如果CUTP轴为(-)Z轴了，z轴改为X，

        test_ori("24381/562".into(), "Y is X and Z is Y").await;
        return Ok(());
        //with cutplane
        let sjoi_strs = vec![
            "Y is -X 22.452 -Y and Z is -Y 22.452 X",
            "Y is -X 20 -Y and Z is -Y 20 X",
            "Y is -X 20 Y and Z is -Y 20 -X",
            "Y is -Y and Z is X",
            "Y is -X and Z is -Y",
            "Y is X and Z is Y",
            "Y is Y 30.484 X and Z is -X 30.484 Y",
        ];
        for s in sjoi_strs {
            let m = DMat3::from_quat(parse_ori_str_to_quat(s).unwrap().as_dquat());
            dbg!(m.x_axis);
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_query_transform_STWALL() -> anyhow::Result<()> {
        init_test_surreal().await;
        //如果是SCOJ, 需要获取两边的连接点，组合出来的方向位置

        test_ori("17496/105740".into(), "Y is Z and Z is -X 22.452 -Y").await;
        Ok(())
    }

    #[tokio::test]
    async fn test_query_transform_SCTN() -> anyhow::Result<()> {
        init_test_surreal().await;

        //todo fix
        // test_transform("17496/274032".into(), "Y is -Z and Z is X").await;
        test_transform(
            "24381/178543".into(),
            "Y is -Y 27.581 -X and Z is Z",
            "",
        )
            .await;

        // test_transform(
        //     "24381/77310".into(),
        //     "Y is Z and Z is Y 43 -X",
        //     "X 9574.353mm Y 8934.174mm Z 4345.8mm",
        // )
        // .await;
        Ok(())
    }

    #[tokio::test]
    async fn test_query_transform_TMPL() -> anyhow::Result<()> {
        init_test_surreal().await;
        test_ori("8196/14755".into(), "Y is -X and Z is Z").await;
        Ok(())
    }

    #[tokio::test]
    async fn test_query_transform_JLDATU() -> anyhow::Result<()> {
        init_test_surreal().await;
        test_ori("17496/152079".into(), "Y is Z and Z is -Y 29.298 X").await;
        // test_transform("17496/268326".into(), "Y is -Y and Z is -Z").await;
        // test_transform("25688/48820".into(), "Y is Z and Z is X 33.955 Y").await;
        // test_transform("24384/28751".into(), "Y is Y 31.0031 X 89.9693 Z and Z is -Y 31 -X 0.0307 Z").await;
        // test_transform("17496/137181".into(), "Y is Z and Z is -Y 34.6032 -X").await;
        Ok(())
    }

    #[tokio::test]
    async fn test_query_transform_SPINE() -> anyhow::Result<()> {
        init_test_surreal().await;
        test_ori(
            "17496/268345".into(),
            "Y is X 89.891 Z and Z is -X 0.1089 Z",
        )
        .await;
        Ok(())
    }

    #[tokio::test]
    async fn test_query_transform_BOX() -> anyhow::Result<()> {
        init_test_surreal().await;
        test_ori(
            "17496/171666".into(),
            "Y is -X 5 -Y 40 -Z and Z is -X 5 -Y 50 Z",
        )
        .await;
        Ok(())
    }

    #[tokio::test]
    async fn test_query_transform_GENSEC() -> anyhow::Result<()> {
        init_test_surreal().await;
        test_ori("24384/28745".into(), "Y is -X 31 Y and Z is Z").await;
        Ok(())
    }

    #[tokio::test]
    async fn test_query_transform_PLDATU() -> anyhow::Result<()> {
        init_test_surreal().await;

        test_ori("24384/25786".into(), "Y is -X and Z is -Y").await;
        // test_ori("25688/48689".into(), "Y is Y 43.307 X and Z is X 43.307 -Y").await;
        // test_ori("25688/48821".into(), "Y is X 33.955 Y and Z is Y 33.955 -X").await;

        Ok(())
    }

    #[tokio::test]
    async fn test_query_transform_FIXING() -> anyhow::Result<()> {
        init_test_surreal().await;
        test_ori(
            "24384/28753".into(),
            "Y is -Y 31 -X 0.031 Z and Z is Y 31 X 89.969 Z",
        )
        .await;
        // test_ori("17496/152081".into(), "Y is Z and Z is X 30.659 Y").await;
        test_ori("24384/25787".into(), "Y is -X 0.031 Z and Z is X 89.969 Z").await;
        Ok(())
    }

    #[tokio::test]
    async fn test_query_transform_FIT() -> anyhow::Result<()> {
        init_test_surreal().await;
        test_transform("24381/55590".into(), "Y is Z and Z is -X", "X 15455.2mm Y -39949.8mm Z 34500mm").await;
        // test_ori("24381/77311".into(), "Y is -Y 43 X and Z is Z").await;
        // test_ori("17496/202352".into(), "Y is X and Z is -Y").await;
        // test_ori("24381/38388".into(), "Y is -X 13 Y and Z is Y 13 X").await;
        // test_ori("17496/106463".into(), "Y is X 25 -Y and Z is -Y 25 -X").await;
        Ok(())
    }
}

#[tokio::test]
async fn test_query_transform() -> anyhow::Result<()> {
    crate::init_test_surreal().await;

    // //X
    test_print_ori("Y is -X 14 -Y and Z is Y 14 -X");
    // //Y
    test_print_ori("Y is -Y 14 X and Z is -X 14 -Y");
    // //Z
    test_print_ori("Y is Y 14 -X and Z is Z");

    // test_cal_ori(DVec3::X);
    // test_cal_ori(DVec3::NEG_X);
    // test_cal_ori(DVec3::Y);
    // test_cal_ori(DVec3::NEG_Y);
    // test_cal_ori(DVec3::Z);
    // test_cal_ori(DVec3::NEG_Z);
    // //
    // let dir = parse_expr_to_dir("X 45 Y").unwrap();
    // test_cal_ori(dir);

    // let ori = Quat::from_rotation_arc(Vec3::Z, Vec3::X);
    // dbg!(math_tool::quat_to_pdms_ori_xyz_str(&ori));
    //
    // dbg!(math_tool::quat_to_pdms_ori_xyz_str(&Quat::from_rotation_arc(Vec3::Z, Vec3::Y)));

    let transform = rs_surreal::get_world_transform("17496/202374".into())
        .await
        .unwrap()
        .unwrap();
    dbg!(transform);
    let rot_mat = Mat3::from_quat(transform.rotation);
    dbg!(rot_mat);
    // let ori_str = math_tool::to_pdms_ori_xyz_str(&rot_mat);
    // dbg!(&ori_str);

    // let transform = rs_surreal::get_world_transform("24383/89691".into())
    //     .await
    //     .unwrap().unwrap();
    // dbg!(transform);
    // let rot_mat = Mat3::from_quat(transform.rotation);
    // let ori_str = math_tool::to_pdms_ori_xyz_str(&rot_mat);
    // dbg!(&ori_str);

    Ok(())
}

#[tokio::test]
async fn test_query_fixing() -> anyhow::Result<()> {
    crate::init_test_surreal().await;
    let transform = rs_surreal::get_world_transform("25688_43205".into())
        .await
        .unwrap()
        .unwrap();
    dbg!(transform);
    let rot_mat = Mat3::from_quat(transform.rotation);
    // let ori_str = math_tool::to_pdms_ori_xyz_str(&rot_mat);
    // dbg!(&ori_str);
    Ok(())
}

#[tokio::test]
async fn test_query_nearest_along() -> anyhow::Result<()> {
    crate::init_test_surreal().await;
    load_aabb_tree().await.unwrap();
    let nearest = rs_surreal::query_neareast_along_axis("24383/66745".into(), Vec3::NEG_Z, "FLOOR")
        .await
        .unwrap();
    dbg!(nearest);
    // assert_eq!(nearest.to_string().as_str(), "25688_71674");

    let nearest = rs_surreal::query_neareast_along_axis("24383/66771".into(), Vec3::NEG_Z, "FLOOR")
        .await
        .unwrap();
    dbg!(nearest);
    // assert_eq!(nearest.to_string(), "25688_45314");
    Ok(())
}
