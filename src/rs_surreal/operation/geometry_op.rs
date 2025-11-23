use crate::prim_geo::category::CateCsgShape;
use crate::rs_surreal::inst_structs::GeoRelate;
use crate::rs_surreal::query_ext::SurrealQueryExt;
use crate::{RefnoEnum, SUL_DB};
use anyhow::Result;

/// Persist geo_relate edges for a batch of shapes, ensuring `trans` is written from `CateCsgShape.transform`.
///
/// This function does NOT create inst_geo or inst_relate; it only creates geo_relate edges
/// between an existing inst node (input) and a geo node (output) and writes `trans`.
///
/// Parameters:
/// - `inst_in`: Thing id of inst node (e.g., "inst:⟨...⟩" or a valid Surreal record id string)
/// - `geo_out`: Thing id of geo node (e.g., "inst_geo:⟨...⟩")
/// - `owner`: The owner Refno, used to synthesize a stable relate id if needed
/// - `shapes`: The generated CSG shapes; transform will be persisted into geo_relate.trans
/// - `geo_type`: A string tag describing the geometry type (e.g., "PrimLoft", "Extrusion")
///
/// Returns the created geo_relate ids.
pub async fn persist_geo_relates_for_shapes(
    inst_in: &str,
    geo_out: &str,
    owner: RefnoEnum,
    shapes: &[CateCsgShape],
    geo_type: &str,
) -> Result<Vec<String>> {
    let mut ids = Vec::with_capacity(shapes.len());
    for (idx, s) in shapes.iter().enumerate() {
        // Synthesize a deterministic relate id using owner and index
        let relate_id = format!("{}_{}", owner.refno(), idx);
        let mut gr = GeoRelate::new(
            format!("relate_{}", relate_id),
            inst_in.to_string(),
            geo_out.to_string(),
            true,
            true,
            geo_type.to_string(),
        )
        .with_trans(s.transform)
        .with_geom_refno(format!("{}", s.refno));

        let sql = gr.to_surql();
        // Fire-and-forget per relate. If any fails, return error.
        SUL_DB.query_response(sql).await?;
        ids.push(relate_id);
    }
    Ok(ids)
}
