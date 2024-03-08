use crate::{options::DbOption, SUL_DB};



///创建几何相关索引索引
pub async fn create_geom_index() -> anyhow::Result<()> {
    //针对一些特殊的表，需要先创建表，定义索引
    {
        SUL_DB
            .query(" DEFINE INDEX unique_inst_relate ON TABLE inst_relate COLUMNS in, out UNIQUE; \
                DEFINE INDEX unique_geo_relate ON TABLE geo_relate COLUMNS in, geom_refno UNIQUE; \
                DEFINE INDEX unique_tubi_relate ON TABLE tubi_relate COLUMNS arrive, leave UNIQUE")
            .await
            .unwrap();
    }
    Ok(())
}

/// 创建 pe_owner 的唯一性索引，in, out的组合索引
pub async fn create_owner_index(db_option: &DbOption) -> anyhow::Result<()> {
    //针对一些特殊的表，需要先创建表，定义索引
    if !db_option.incr_sync {
        SUL_DB
            .query(r#"DEFINE INDEX unique_pe_owner ON TABLE pe_owner COLUMNS in, out UNIQUE"#)
            .await
            .unwrap();
    }
    Ok(())
}

pub async fn define_fullname_index(db_option: &DbOption) -> anyhow::Result<()> {
    //针对一些特殊的表，需要先创建表，定义索引
    if !db_option.incr_sync {
        SUL_DB
            .query(r#"DEFINE ANALYZER name_fulltext TOKENIZERS class FILTERS lowercase;
                    DEFINE INDEX fulltext_name ON TABLE pe FIELDS name SEARCH ANALYZER name_fulltext BM25 HIGHLIGHTS;
                "#)
            .await
            .unwrap();
    }
    Ok(())
}