use crate::{NamedAttrMap, RefU64, SUL_DB, SurlValue, SurrealQueryExt};
use cached::proc_macro::cached;
use std::io::Read;
use std::path::PathBuf;

/// 执行 SurrealDB 脚本目录中的所有脚本
///
/// # 参数
/// * `script_dir` - 脚本目录路径，如果为 None，则从 DbOption 配置中读取
///
/// # 示例
/// ```no_run
/// // 使用默认配置路径
/// define_common_functions(None).await?;
///
/// // 使用指定路径
/// define_common_functions(Some("resource/surreal")).await?;
/// ```
pub async fn define_common_functions(script_dir: Option<&str>) -> anyhow::Result<()> {
    // 如果传入 None，从 DbOption 配置中读取路径
    let dir_path = if let Some(dir) = script_dir {
        dir.to_string()
    } else {
        // 读取配置文件获取脚本目录
        use config::{Config, File};
        // 读取配置文件
        let config_file_name = std::env::var("DB_OPTION_FILE").unwrap_or_else(|_| "DbOption".to_string());
        let s = Config::builder()
            .add_source(File::with_name(&config_file_name))
            .build()?;
        let db_option: crate::options::DbOption = s.try_deserialize()?;
        db_option.get_surreal_script_dir().to_string()
    };

    let target_dir = std::fs::read_dir(&dir_path)?
        .into_iter()
        .map(|entry| {
            let entry = entry.unwrap();
            entry.path()
        })
        .collect::<Vec<PathBuf>>();
    for file in target_dir {
        println!(
            "载入surreal {}",
            file.file_name().unwrap().to_str().unwrap().to_string()
        );
        let mut file = std::fs::File::open(file)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        SUL_DB.query_response(&content).await?;
    }
    Ok(())
}

/// 定义数据库编号事件
///
/// 当创建新的 pe 记录时,会触发此事件来更新 dbnum_info_table 表中的信息
///
/// # 错误
///
/// 如果数据库操作失败,将返回错误
pub async fn define_dbnum_event() -> anyhow::Result<()> {
    SUL_DB.query_response(r#"
        DEFINE EVENT OVERWRITE update_dbnum_event ON pe WHEN $event = "CREATE" OR $event = "UPDATE" OR $event = "DELETE" THEN {
            -- 获取当前记录的 dbnum
            LET $dbnum = $value.dbnum;
            LET $id = record::id($value.id);
            let $ref_0 = array::at($id, 0);
            let $ref_1 = array::at($id, 1);
            let $is_delete = $value.deleted and $event = "UPDATE";
            let $max_sesno = if $after.sesno > $before.sesno?:0 { $after.sesno } else { $before.sesno };
            -- 根据事件类型处理  type::record("dbnum_info_table", $ref_0)
            IF $event = "CREATE"   {
                UPSERT type::record('dbnum_info_table', $ref_0) SET
                    dbnum = $dbnum,
                    count = count?:0 + 1,
                    sesno = $max_sesno,
                    max_ref1 = $ref_1;
            } ELSE IF $event = "DELETE" OR $is_delete  {
                UPSERT type::record('dbnum_info_table', $ref_0) SET
                    count = count - 1,
                    sesno = $max_sesno,
                    max_ref1 = $ref_1
                WHERE count > 0;
            };
        };
        "#)
        .await?;
    Ok(())
}
