use crate::SurrealQueryExt;
/// Boolean operation query functions
///
/// This module provides query functions for boolean operations on geometry,
/// including catalog negative boolean groups, manifold operations, and OCC operations.
use crate::rs_surreal::query_structs::{
    CataNegGroup, GmGeoData, ManiGeoTransQuery, OccGeoTransQuery,
};
use crate::types::RefnoEnum;
use crate::{SUL_DB, get_inst_relate_keys};

/// Query catalog negative boolean groups
///
/// # Parameters
///
/// * `refnos` - Array of reference numbers
/// * `replace_exist` - Whether to replace existing boolean operation results
///
/// # Returns
///
/// Returns `Vec<CataNegGroup>` containing catalog negative boolean groups
///
/// # SQL Query
///
/// Selects instances with catalog negative geometry:
/// - Filters by `has_cata_neg` flag
/// - Optionally filters by `!bad_bool and !booled` if not replacing
/// - Returns refno, inst_info_id, and boolean_group (flattened geom_refno and cata_neg)
pub async fn query_cata_neg_boolean_groups(
    refnos: &[RefnoEnum],
    replace_exist: bool,
) -> anyhow::Result<Vec<CataNegGroup>> {
    let inst_keys = get_inst_relate_keys(refnos);

    let mut sql = format!(
        r#" select in as refno, (->inst_info)[0] as inst_info_id, (select value array::flatten([geom_refno, cata_neg])
            from ->inst_info->geo_relate where visible and !out.bad and cata_neg!=none) as boolean_group
            from {inst_keys} where in.id != none and (->inst_info)[0]!=none and has_cata_neg "#
    );

    if !replace_exist {
        sql.push_str("and !bad_bool and !booled");
    }

    SUL_DB.query_take(&sql, 0).await
}

/// Query geometry mesh data
///
/// # Parameters
///
/// * `refno` - Reference number
/// * `geom_refnos` - Array of geometry reference numbers to query
///
/// # Returns
///
/// Returns `Vec<GmGeoData>` containing geometry mesh data (ID, transform, param, aabb)
///
/// # SQL Query
///
/// Selects geometry data from inst_relate->inst_info->geo_relate:
/// - Filters by geom_refno in the provided list
/// - Requires aabb and param to be non-null
/// - Returns record ID, geom_refno, transform, param, and aabb_id
pub async fn query_geom_mesh_data(
    refno: RefnoEnum,
    geom_refnos: &[RefnoEnum],
) -> anyhow::Result<Vec<GmGeoData>> {
    let pes = geom_refnos
        .iter()
        .map(|x| x.to_pe_key())
        .collect::<Vec<_>>()
        .join(",");

    let sql = format!(
        r#"
        select out as id, geom_refno, trans.d as trans, out.param as param, out.aabb as aabb_id
        from {}->inst_relate->inst_info->geo_relate
        where !out.bad and geom_refno in [{}]  and out.aabb!=none and out.param!=none"#,
        refno.to_pe_key(),
        pes
    );

    SUL_DB.query_take(&sql, 0).await
}

/// Query manifold boolean operations
///
/// # Parameters
///
/// * `refno` - Reference number
///
/// # Returns
///
/// Returns `Vec<ManiGeoTransQuery>` containing manifold boolean operation data
///
/// # SQL Query
///
/// Selects instances with negative geometry relationships:
/// - Filters instances with neg_relate or ngmr_relate
/// - Returns refno, sesno, noun, world_trans, aabb
/// - Returns positive geometries (Compound/Pos) with transforms
/// - Returns negative geometries (Neg/CataCrossNeg) with transforms and aabb
pub async fn query_manifold_boolean_operations(
    refno: RefnoEnum,
) -> anyhow::Result<Vec<ManiGeoTransQuery>> {
    let sql = format!(
        r#"
        select
                in as refno,
                in.sesno as sesno,
                in.noun as noun,
                world_trans.d as wt,
                aabb.d as aabb,
                (select value [out, trans.d] from out->geo_relate where geo_type in ["Compound", "Pos"] and trans.d != NONE ) as ts,
                (select value [in, world_trans.d,
                    (select out as id, geo_type, trans.d as trans, out.aabb.d as aabb
                    from array::flatten(out->geo_relate) where trans.d != NONE and ( geo_type=="Neg" or (geo_type=="CataCrossNeg"
                        and geom_refno in (select value ngmr from pe:{refno}<-ngmr_relate) ) ))]
                        from array::flatten([array::flatten(in<-neg_relate.in->inst_relate), array::flatten(in<-ngmr_relate.in->inst_relate)]) where world_trans.d!=none
                ) as neg_ts
             from inst_relate:{refno} where in.id != none and !bad_bool and ((in<-neg_relate)[0] != none or in<-ngmr_relate[0] != none) and aabb.d != NONE
        "#
    );

    SUL_DB.query_take(&sql, 0).await
}

/// Query OCC boolean operations
///
/// # Parameters
///
/// * `refnos` - Array of reference numbers
/// * `replace_exist` - Whether to replace existing boolean operation results
///
/// # Returns
///
/// Returns `Vec<OccGeoTransQuery>` containing OCC boolean operation data
///
/// # Note
///
/// This function needs to be implemented based on the OCC boolean operation requirements.
/// The SQL query should be similar to manifold operations but return OccGeoTransQuery instead.
pub async fn query_occ_boolean_operations(
    refnos: &[RefnoEnum],
    replace_exist: bool,
) -> anyhow::Result<Vec<OccGeoTransQuery>> {
    // TODO: Implement OCC boolean operations query
    // This should query instances with negative geometry relationships
    // and return data suitable for OCC boolean operations

    let inst_keys = get_inst_relate_keys(refnos);

    let mut sql = format!(
        r#"
        select
                in as refno,
                in.noun as noun,
                world_trans.d as wt,
                aabb.d as aabb,
                (select value [out.param, trans.d] from out->geo_relate where geo_type in ["Compound", "Pos"] and trans.d != NONE and out.param != NONE) as ts,
                (select value [in, world_trans.d,
                    (select out.param as param, geo_type, para_type, trans.d as trans, out.aabb.d as aabb
                    from array::flatten(out->geo_relate) where trans.d != NONE and out.param != NONE and ( geo_type=="Neg" or (geo_type=="CataCrossNeg"
                        and geom_refno in (select value ngmr from in<-ngmr_relate) ) ))]
                        from array::flatten([array::flatten(in<-neg_relate.in->inst_relate), array::flatten(in<-ngmr_relate.in->inst_relate)]) where world_trans.d!=none
                ) as neg_ts
             from {inst_keys} where in.id != none and ((in<-neg_relate)[0] != none or in<-ngmr_relate[0] != none) and aabb.d != NONE
        "#
    );

    if !replace_exist {
        sql.push_str(" and !bad_bool and !booled");
    }

    SUL_DB.query_take(&sql, 0).await
}

/// Query simple catalog negative boolean operations
///
/// # Returns
///
/// Returns `Vec<CataNegGroup>` containing simple catalog negative boolean groups
///
/// # SQL Query
///
/// Similar to query_cata_neg_boolean_groups but without refno filtering.
/// Used for catalog-level boolean operations.
pub async fn query_simple_cata_negative_bool() -> anyhow::Result<Vec<CataNegGroup>> {
    let sql = r#"
        select in as refno, (->inst_info)[0] as inst_info_id,
        (select value array::flatten([geom_refno, cata_neg])
            from ->inst_info->geo_relate where visible and !out.bad and cata_neg!=none) as boolean_group
        from inst_relate where in.id != none and (->inst_info)[0]!=none and has_cata_neg and !bad_bool and !booled
    "#;

    SUL_DB.query_take(sql, 0).await
}
