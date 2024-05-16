use glam::Vec3;
use crate::room::query::*;
use crate::rs_surreal;
use crate::test::test_surreal::init_test_surreal;

#[tokio::test]
async fn test_query_point_room() -> anyhow::Result<()> {
    init_test_surreal().await;
    // let refno = "13292_92".into();
    // let pe = rs_surreal::get_pe(refno).await.unwrap();
    // dbg!(pe);
    let point = Vec3::new(19381.85,4055.68,16679.);
    let mut time = std::time::Instant::now();
    let result = query_room_number_by_point(point).await.unwrap();
    dbg!(&result);
    println!("query_room_number_by_point花费时间: {} ms", time.elapsed().as_millis());


    Ok(())
}