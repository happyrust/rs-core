///查询工艺大宗材料数据
pub async fn get_gy_dzcl(refno: RefU64) -> anyhow::Result<Vec<NamedAttrMap>> {
    let mut response = SUL_DB
        .query(include_str!(
            "schemas/query_ancestor_attmaps_by_refno.surql"
        ))
        .bind(("refno", refno.to_string()))
        .await?;
    let o: SurlValue = response.take(1)?;
    let os: Vec<SurlValue> = o.try_into().unwrap();
    let named_attmaps: Vec<NamedAttrMap> = os.into_iter().map(|x| x.into()).collect();
    Ok(named_attmaps)
}