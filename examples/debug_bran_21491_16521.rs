use aios_core::{RefnoEnum, SUL_DB, SurrealQueryExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    aios_core::init_test_surreal().await?;

    let bran_refno = RefnoEnum::from("21491/16521");
    println!("🔍 调试 BRAN: {}", bran_refno);
    println!("{}", "=".repeat(80));

    // 1. 检查 BRAN 本身的基本信息
    println!("\n【1】检查 BRAN 基本信息:");
    let sql = format!("SELECT id, noun, dbnum, sesno, deleted FROM {};", bran_refno.to_pe_key());
    println!("SQL: {}", sql);
    let result: Result<Vec<serde_json::Value>, _> = SUL_DB.query_take(&sql, 0).await;
    match result {
        Ok(bran_info) => println!("结果: {}", serde_json::to_string_pretty(&bran_info)?),
        Err(e) => println!("错误: {:?}", e),
    }

    // 2. 检查 BRAN 的属性（ARRI/LEAV）
    println!("\n【2】检查 BRAN 的 ARRI/LEAV 属性:");
    let attrs = aios_core::get_named_attmap(bran_refno.clone()).await?;
    println!("ARRI: {:?}", attrs.get("ARRI"));
    println!("LEAV: {:?}", attrs.get("LEAV"));
    println!("NAME: {:?}", attrs.get("NAME"));
    println!("noun: {:?}", attrs.get_type_str());

    // 3. 检查 tubi_relate 关系
    println!("\n【3】检查 tubi_relate 关系:");
    let sql = format!("SELECT id, arrive, leave FROM {}->tubi_relate;", bran_refno.to_pe_key());
    println!("SQL: {}", sql);
    let result: Result<Vec<serde_json::Value>, _> = SUL_DB.query_take(&sql, 0).await;
    match result {
        Ok(tubi_relates) => {
            println!("tubi_relate 数量: {}", tubi_relates.len());
            println!("结果: {}", serde_json::to_string_pretty(&tubi_relates)?);
        },
        Err(e) => println!("错误: {:?}", e),
    }

    // 4. 检查 inst_relate 关系
    println!("\n【4】检查 inst_relate 关系:");
    let sql = format!("SELECT id, in, out FROM {}->inst_relate LIMIT 1;", bran_refno.to_pe_key());
    println!("SQL: {}", sql);
    let result: Result<Vec<serde_json::Value>, _> = SUL_DB.query_take(&sql, 0).await;
    match result {
        Ok(inst_relates) => {
            println!("inst_relate 数量: {}", inst_relates.len());
            if !inst_relates.is_empty() {
                println!("第一个 inst_relate: {}", serde_json::to_string_pretty(&inst_relates[0])?);
            }
        },
        Err(e) => println!("错误: {:?}", e),
    }

    // 5. 检查 inst_relate 中的 ptset
    println!("\n【5】检查 inst_relate 中的 ptset:");
    let sql = format!("SELECT out.ptset FROM {}->inst_relate LIMIT 1;", bran_refno.to_pe_key());
    println!("SQL: {}", sql);
    let result: Result<Vec<serde_json::Value>, _> = SUL_DB.query_take(&sql, 0).await;
    match result {
        Ok(ptsets) => println!("结果: {}", serde_json::to_string_pretty(&ptsets)?),
        Err(e) => println!("错误: {:?}", e),
    }

    // 6. 检查 BRAN 的子元素
    println!("\n【6】检查 BRAN 的子元素:");
    let sql = format!("SELECT value in FROM {}<-pe_owner;", bran_refno.to_pe_key());
    println!("SQL: {}", sql);
    let result: Result<Vec<RefnoEnum>, _> = SUL_DB.query_take(&sql, 0).await;
    let children = match result {
        Ok(c) => {
            println!("子元素数量: {}", c.len());
            for (i, child) in c.iter().enumerate().take(10) {
                println!("  [{}] {}", i + 1, child);
            }
            c
        },
        Err(e) => {
            println!("错误: {:?}", e);
            vec![]
        }
    };

    // 7. 检查子元素的详细信息
    if !children.is_empty() {
        println!("\n【7】检查子元素的详细信息:");
        for (i, child) in children.iter().take(3).enumerate() {
            println!("\n  --- 子元素 [{}]: {} ---", i + 1, child);
            let child_attrs = aios_core::get_named_attmap(child.clone()).await?;
            println!("  noun: {:?}", child_attrs.get_type_str());
            println!("  ARRI: {:?}", child_attrs.get("ARRI"));
            println!("  LEAV: {:?}", child_attrs.get("LEAV"));
            println!("  NAME: {:?}", child_attrs.get("NAME"));

            // 检查子元素的 inst_relate
            let sql = format!("SELECT id FROM {}->inst_relate LIMIT 1;", child.to_pe_key());
            let result: Result<Vec<serde_json::Value>, _> = SUL_DB.query_take(&sql, 0).await;
            match result {
                Ok(child_inst) if !child_inst.is_empty() => {
                    println!("  inst_relate 存在: {}", serde_json::to_string_pretty(&child_inst[0])?);
                },
                _ => println!("  inst_relate: 无"),
            }
        }
    }

    // 8. 使用原始查询检查 tubi_relate 的详细信息
    println!("\n【8】使用原始查询检查 tubi_relate:");
    let sql = format!(
        r#"
        SELECT
            in.id as refno,
            in.owner.noun as generic,
            arrive,
            leave
        FROM array::flatten([{}]->tubi_relate)
        "#,
        bran_refno.to_pe_key()
    );
    println!("SQL: {}", sql);
    let result: Result<Vec<serde_json::Value>, _> = SUL_DB.query_take(&sql, 0).await;
    match result {
        Ok(tubi_details) => {
            println!("tubi_relate 详细信息数量: {}", tubi_details.len());
            for (i, detail) in tubi_details.iter().take(5).enumerate() {
                println!("\n  [{}] {}", i + 1, serde_json::to_string_pretty(detail)?);
            }
        },
        Err(e) => println!("错误: {:?}", e),
    }

    // 9. 检查 arrive/leave 点的查询
    println!("\n【9】检查 arrive/leave 点查询:");
    let sql = format!(
        r#"
        SELECT value [
            in,
            world_trans.d,
            (SELECT * FROM object::values(out.ptset) WHERE number=$parent.in.refno.ARRI)[0],
            (SELECT * FROM object::values(out.ptset) WHERE number=$parent.in.refno.LEAV)[0]
        ]
        FROM array::flatten([{}][? owner.noun in ['BRAN', 'HANG']]->inst_relate) WHERE world_trans.d!=none
        "#,
        bran_refno.to_pe_key()
    );
    println!("SQL: {}", sql);
    let result: Result<Vec<serde_json::Value>, _> = SUL_DB.query_take(&sql, 0).await;
    match result {
        Ok(arrive_leave) => {
            println!("arrive/leave 查询结果数量: {}", arrive_leave.len());
            println!("结果: {}", serde_json::to_string_pretty(&arrive_leave)?);
        },
        Err(e) => println!("错误: {:?}", e),
    }

    println!("\n{}", "=".repeat(80));
    println!("调试完成！");

    Ok(())
}

