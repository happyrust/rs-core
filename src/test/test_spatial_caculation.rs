use crate::{spatial::pipe::create_valve_floor_relations, test::test_surreal::init_test_surreal};


#[tokio::test]
async fn test_cal_valve_meta_data() {
    init_test_surreal().await;
    let mut valve_meta_data = create_valve_floor_relations().await.unwrap();
}