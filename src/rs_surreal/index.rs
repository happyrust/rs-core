use crate::{SUL_DB, options::DbOption};
use surrealdb::Surreal;
use surrealdb::engine::any::Any;

///创建几何相关索引索引
pub async fn create_geom_index() -> anyhow::Result<()> {
    create_geom_index_with(&SUL_DB).await
}

/// 在给定连接上创建几何相关索引
pub async fn create_geom_index_with(conn: &Surreal<Any>) -> anyhow::Result<()> {
    //针对一些特殊的表，需要先创建表，定义索引
    //DEFINE INDEX unique_geo_relate ON TABLE geo_relate COLUMNS in, geom_refno UNIQUE;
    // DEFINE INDEX unique_tubi_relate ON TABLE tubi_relate COLUMNS arrive, leave UNIQUE
    //DEFINE INDEX unique_inst_relate ON TABLE inst_relate COLUMNS in, out UNIQUE;
    conn.query(
        r#"
                DEFINE INDEX unique_neg_relate ON TABLE neg_relate COLUMNS in, out UNIQUE;
                DEFINE INDEX unique_nearest_relate ON TABLE nearest_relate COLUMNS in, out UNIQUE;
             "#,
    )
    .await
    .unwrap();
    Ok(())
}

pub async fn define_room_index() -> anyhow::Result<()> {
    define_room_index_with(&SUL_DB).await
}

/// 在给定连接上创建房间相关索引
pub async fn define_room_index_with(conn: &Surreal<Any>) -> anyhow::Result<()> {
    //针对一些特殊的表，需要先创建表，定义索引
    conn.query(
        r#"
        DEFINE INDEX unique_room_relate ON TABLE room_relate COLUMNS in, out UNIQUE;
        DEFINE INDEX unique_room_panel_relate ON TABLE room_panel_relate COLUMNS in, out UNIQUE;
               "#,
    )
    .await
    .unwrap();
    Ok(())
}

/// 创建 pe_owner 的唯一性索引，in, out的组合索引
pub async fn define_owner_index() -> anyhow::Result<()> {
    define_owner_index_with(&SUL_DB).await
}

/// 在给定连接上创建 pe_owner 的唯一性索引
pub async fn define_owner_index_with(conn: &Surreal<Any>) -> anyhow::Result<()> {
    //针对一些特殊的表，需要先创建表，定义索引
    conn.query(r#"DEFINE INDEX unique_pe_owner ON TABLE pe_owner COLUMNS in, out UNIQUE"#)
        .await
        .unwrap();
    Ok(())
}

pub async fn define_fullname_index() -> anyhow::Result<()> {
    define_fullname_index_with(&SUL_DB).await
}

/// 在给定连接上创建全文索引
pub async fn define_fullname_index_with(conn: &Surreal<Any>) -> anyhow::Result<()> {
    //针对一些特殊的表，需要先创建表，定义索引
    conn
        .query(r#"DEFINE ANALYZER name_fulltext TOKENIZERS class FILTERS lowercase;
                    DEFINE INDEX fulltext_name ON TABLE pe FIELDS name SEARCH ANALYZER name_fulltext BM25 HIGHLIGHTS;
                "#)
        .await
        .unwrap();
    Ok(())
}

pub async fn define_pe_index() -> anyhow::Result<()> {
    define_pe_index_with(&SUL_DB).await
}

/// 在给定连接上创建 pe 表相关索引
pub async fn define_pe_index_with(conn: &Surreal<Any>) -> anyhow::Result<()> {
    //针对一些特殊的表，需要先创建表，定义索引
    conn.query(
        r#"
        DEFINE index pe_name_index ON TABLE pe COLUMNS name;
        DEFINE index pe_noun_index ON TABLE pe COLUMNS noun;
        DEFINE index pe_refno_index ON TABLE pe COLUMNS refno;
        DEFINE index pe_cata_hash_index ON TABLE pe COLUMNS cata_hash;
        DEFINE index pe_dbnum_index ON TABLE pe COLUMNS dbnum;
        DEFINE INDEX sesno_index ON TABLE pe COLUMNS sesno;
        DEFINE INDEX idx_pe_neg_relate_reverse ON TABLE neg_relate COLUMNS out;
        DEFINE INDEX idx_pe_ngmr_relate_reverse ON TABLE ngmr_relate COLUMNS out;
                "#,
    )
    .await
    .unwrap();
    Ok(())
}
pub async fn define_ses_index() -> anyhow::Result<()> {
    define_ses_index_with(&SUL_DB).await
}

/// 在给定连接上创建 ses 表相关索引
pub async fn define_ses_index_with(conn: &Surreal<Any>) -> anyhow::Result<()> {
    //针对一些特殊的表，需要先创建表，定义索引
    conn.query(
        r#"
        DEFINE INDEX date_index ON ses COLUMNS date;
        DEFINE INDEX dbnum_index ON ses COLUMNS dbnum;
        DEFINE INDEX sesno_index ON ses COLUMNS sesno;
                "#,
    )
    .await
    .unwrap();
    Ok(())
}

pub async fn define_measurement_index() -> anyhow::Result<()> {
    define_measurement_index_with(&crate::SUL_DB).await
}

/// 在给定连接上创建 measurement 表相关索引
pub async fn define_measurement_index_with(conn: &Surreal<Any>) -> anyhow::Result<()> {
    // 创建测量表的索引以优化常用查询
    conn.query(
        r#"
        DEFINE INDEX idx_measurement_project_id ON TABLE measurement COLUMNS project_id;
        DEFINE INDEX idx_measurement_scene_id ON TABLE measurement COLUMNS scene_id;
        DEFINE INDEX idx_measurement_type ON TABLE measurement COLUMNS measurement_type;
        DEFINE INDEX idx_measurement_created_at ON TABLE measurement COLUMNS created_at;
        DEFINE INDEX idx_measurement_status ON TABLE measurement COLUMNS status;
        DEFINE INDEX idx_measurement_created_by ON TABLE measurement COLUMNS created_by;
        DEFINE INDEX idx_measurement_project_scene ON TABLE measurement COLUMNS project_id, scene_id;
                "#,
    )
    .await?;
    Ok(())
}

pub async fn define_annotation_index() -> anyhow::Result<()> {
    define_annotation_index_with(&crate::SUL_DB).await
}

/// 在给定连接上创建 annotation 表相关索引
pub async fn define_annotation_index_with(conn: &Surreal<Any>) -> anyhow::Result<()> {
    // 创建批注表的索引以优化常用查询
    conn.query(
        r#"
        DEFINE INDEX idx_annotation_project_id ON TABLE annotation COLUMNS project_id;
        DEFINE INDEX idx_annotation_scene_id ON TABLE annotation COLUMNS scene_id;
        DEFINE INDEX idx_annotation_type ON TABLE annotation COLUMNS annotation_type;
        DEFINE INDEX idx_annotation_status ON TABLE annotation COLUMNS status;
        DEFINE INDEX idx_annotation_created_by ON TABLE annotation COLUMNS created_by;
        DEFINE INDEX idx_annotation_assigned_to ON TABLE annotation COLUMNS assigned_to;
        DEFINE INDEX idx_annotation_created_at ON TABLE annotation COLUMNS created_at;
        DEFINE INDEX idx_annotation_priority ON TABLE annotation COLUMNS priority;
        DEFINE INDEX idx_annotation_project_scene ON TABLE annotation COLUMNS project_id, scene_id;
                "#,
    )
    .await?;
    Ok(())
}

pub async fn define_tag_name_mapping_index() -> anyhow::Result<()> {
    define_tag_name_mapping_index_with(&crate::SUL_DB).await
}

/// 在给定连接上创建 tag_name_mapping 表相关索引
pub async fn define_tag_name_mapping_index_with(conn: &Surreal<Any>) -> anyhow::Result<()> {
    // 创建位号映射表的索引以优化常用查询
    conn.query(
        r#"
        DEFINE INDEX idx_tag_name_mapping_in ON TABLE tag_name_mapping COLUMNS in;
        DEFINE INDEX idx_tag_name_mapping_tag_name ON TABLE tag_name_mapping COLUMNS tag_name;
        DEFINE INDEX idx_tag_name_mapping_full_name ON TABLE tag_name_mapping COLUMNS full_name;
                "#,
    )
    .await?;
    Ok(())
}
