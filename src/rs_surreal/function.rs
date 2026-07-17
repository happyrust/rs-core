use crate::{NamedAttrMap, RefU64, SUL_DB, SurlValue, SurrealQueryExt};
use anyhow::Context as _;
use cached::proc_macro::cached;
use std::io::Read;
use std::path::PathBuf;

fn collect_surql_files(dir_path: &str) -> anyhow::Result<Vec<PathBuf>> {
    let mut surql_files: Vec<PathBuf> = std::fs::read_dir(dir_path)?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|p| p.is_file())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("surql"))
        .collect();
    surql_files.sort();
    Ok(surql_files)
}

async fn execute_surql_files_on_db(
    db: &surrealdb::Surreal<surrealdb::engine::any::Any>,
    ns: &str,
    db_name: &str,
    surql_files: Vec<PathBuf>,
    log_prefix: &str,
) -> anyhow::Result<()> {
    for file in surql_files {
        crate::use_ns_db_compat(db, ns, db_name).await?;
        let file_name = file
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or("<unknown>")
            .to_string();
        println!("载入surreal{} {}", log_prefix, file_name);

        let mut f = std::fs::File::open(&file)
            .with_context(|| format!("打开 Surreal 脚本失败: {}", file.display()))?;
        let mut content = String::new();
        f.read_to_string(&mut content)
            .with_context(|| format!("读取 Surreal 脚本失败: {}", file.display()))?;

        db.query_response(&content)
            .await
            .with_context(|| format!("执行 Surreal 脚本失败: {}", file_name))?;
    }

    Ok(())
}

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
    // 读取配置文件（即使外部显式传入 script_dir，也需要从配置中取得 NS/DB）
    use config::{Config, File};
    let config_file_name =
        std::env::var("DB_OPTION_FILE").unwrap_or_else(|_| "db_options/DbOption".to_string());
    let s = Config::builder()
        .add_source(File::with_name(&config_file_name))
        .build()?;
    let db_option: crate::options::DbOption = s.try_deserialize()?;

    // 如果传入 None，从 DbOption 配置中读取脚本目录
    let dir_path = script_dir
        .map(|dir| dir.to_string())
        .unwrap_or_else(|| db_option.get_surreal_script_dir().to_string());

    let ns = db_option.surreal_ns.clone();
    let db = db_option.project_name.clone();

    let surql_files = collect_surql_files(&dir_path)?;
    execute_surql_files_on_db(&SUL_DB, &ns, &db, surql_files, "").await
}

/// 在指定的数据库连接上执行 SurrealDB 脚本目录中的所有脚本。
///
/// 与 `define_common_functions` 相同逻辑，但可指定目标 DB。
pub async fn define_common_functions_on_db(
    db: &surrealdb::Surreal<surrealdb::engine::any::Any>,
    script_dir: Option<&str>,
) -> anyhow::Result<()> {
    use config::{Config, File};
    let config_file_name =
        std::env::var("DB_OPTION_FILE").unwrap_or_else(|_| "db_options/DbOption".to_string());
    let s = Config::builder()
        .add_source(File::with_name(&config_file_name))
        .build()?;
    let db_option: crate::options::DbOption = s.try_deserialize()?;

    let dir_path = script_dir
        .map(|dir| dir.to_string())
        .unwrap_or_else(|| db_option.get_surreal_script_dir().to_string());

    let ns = db_option.surreal_ns.clone();
    let db_name = db_option.project_name.clone();

    let surql_files = collect_surql_files(&dir_path)?;
    execute_surql_files_on_db(db, &ns, &db_name, surql_files, "(KV)").await
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
