use crate::{room::room::load_aabb_tree, rs_surreal, tool::math_tool};

use std::sync::Arc;
use glam::{DMat3, DQuat, DVec3, Mat3, Quat, Vec3};
use surrealdb::sql::Thing;
use crate::tool::dir_tool::parse_ori_str_to_quat;
use crate::tool::direction_parse::parse_expr_to_dir;

fn test_print_ori(ori: &str) {
    let rotation = parse_ori_str_to_quat(ori).unwrap_or(glam::Quat::IDENTITY);
    dbg!(Mat3::from_quat(rotation));
    dbg!(math_tool::quat_to_pdms_ori_xyz_str(&rotation));
}

fn test_cal_ori(v: DVec3) {
    let ref_dir = if v.dot(DVec3::NEG_Z).abs() > 0.999 {
        DVec3::NEG_Y
    }else{
        DVec3::NEG_Z
    };
    let y_dir = v.cross(ref_dir).normalize();
    let x_dir = y_dir.cross(v).normalize();
    let rotation = DQuat::from_mat3(&DMat3::from_cols(x_dir.into(), y_dir.into(), v.into()));
    dbg!((x_dir, y_dir, v));
    dbg!(math_tool::quat_to_pdms_ori_xyz_str(&rotation.as_quat()));
}

#[cfg(test)]
mod test_transform {
    use glam::{DVec3, Mat3};
    use crate::{cal_ori_by_extru_axis, cal_ori_by_z_axis_ref_x, RefU64, rs_surreal};
    use crate::test::test_surreal::init_test_surreal;
    use crate::tool::dir_tool::parse_ori_str_to_mat;
    use crate::tool::math_tool;
    use crate::tool::math_tool::{dquat_to_pdms_ori_xyz_str, to_pdms_dvec_str};

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
            "Y is Z and Z is -Y 31 -X"
        ];
        let oris = tests.into_iter().map(|x| parse_ori_str_to_mat(x).unwrap()).collect::<Vec<_>>();
        dbg!(&oris);

        for ori in oris {
            let extru_dir = ori.z_axis.as_dvec3();
            dbg!(to_pdms_dvec_str(&extru_dir));
            let quat = cal_ori_by_extru_axis(extru_dir, false);
            dbg!(dquat_to_pdms_ori_xyz_str(&quat));
        }
    }

    async fn test_transform(refno: RefU64, assert_ori: &str){
        let transform = rs_surreal::get_world_transform(refno)
            .await
            .unwrap().unwrap();
        dbg!(transform);
        let rot_mat = Mat3::from_quat(transform.rotation);
        dbg!(rot_mat);
        let ori_str = math_tool::to_pdms_ori_xyz_str(&rot_mat);
        dbg!(&ori_str);
        assert_eq!(ori_str, assert_ori);
    }

    #[tokio::test]
    async fn test_query_transform_JLDATU() -> anyhow::Result<()> {
        init_test_surreal().await;
        // test_transform("24384/28751".into(), "Y is Y 31.0031 X 89.9693 Z and Z is -Y 31 -X 0.0307 Z").await;
        test_transform("17496/137181".into(), "Y is Z and Z is -Y 34.6032 -X").await;
        Ok(())
    }

    #[tokio::test]
    async fn test_query_transform_BOX() -> anyhow::Result<()> {
        init_test_surreal().await;
        test_transform("17496/171666".into(), "Y is -X 5 -Y 40 -Z and Z is -X 5 -Y 50 Z").await;
        Ok(())
    }

    #[tokio::test]
    async fn test_query_transform_GENSEC() -> anyhow::Result<()> {
        init_test_surreal().await;
        test_transform("24384/28745".into(), "Y is -X 31 Y and Z is Z").await;
        Ok(())
    }

    #[tokio::test]
    async fn test_query_transform_PLDATU() -> anyhow::Result<()> {
        init_test_surreal().await;

        test_transform("24384/28752".into(), "Y is -Y 31 -X 0.0307 Z and Z is X 31 -Y").await;

        Ok(())
    }

    #[tokio::test]
    async fn test_query_transform_FIXING() -> anyhow::Result<()> {
        init_test_surreal().await;
        test_transform("24384/28753".into(), "-Y 31 -X 0.0307 Z and Z is Y 31 X 89.9693 Z").await;
        Ok(())
    }

}

#[tokio::test]
async fn test_query_transform() -> anyhow::Result<()> {
    super::init_test_surreal().await;

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
        .unwrap().unwrap();
    dbg!(transform);
    let rot_mat = Mat3::from_quat(transform.rotation);
    dbg!(rot_mat);
    let ori_str = math_tool::to_pdms_ori_xyz_str(&rot_mat);
    dbg!(&ori_str);


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
    super::init_test_surreal().await;
    let transform = rs_surreal::get_world_transform("25688_43205".into())
        .await
        .unwrap().unwrap();
    dbg!(transform);
    let rot_mat = Mat3::from_quat(transform.rotation);
    let ori_str = math_tool::to_pdms_ori_xyz_str(&rot_mat);
    dbg!(&ori_str);
    Ok(())
}


#[tokio::test]
async fn test_query_nearest_along() -> anyhow::Result<()> {
    super::init_test_surreal().await;
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