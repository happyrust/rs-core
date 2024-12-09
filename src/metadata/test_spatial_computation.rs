use std::str::FromStr;
use crate::{init_test_surreal, RefnoEnum, RefU64};
use crate::metadata::spatial_computation::get_supp_panel;

#[tokio::test]
async fn test_get_supp_panel() {
    init_test_surreal().await;
    let refno = RefU64::from_str("24383/89904").unwrap();
    let name = get_supp_panel(refno.into()).await.unwrap();
    assert_eq!(name,"1RS02TT0265P".to_string());
}