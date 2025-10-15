//! 测试助手模块
//! 
/// 使用内存 SurrealDB 进行 RefnoEnum 测试
/// 
use crate::SUL_DB;
use crate::function::define_common_functions;
use anyhow::{anyhow, Result};
use crate::pdms_types::RefnoEnum;
use surrealdb::opt::auth::Root;
use surrealdb::opt::Config;

/// 测试助手结构体
/// 
/// 专门用于测试 SurrealQL 查询返回结果直接反序列化为 RefnoEnum
pub struct Test {
    response: surrealdb::Response,
}

impl Test {
    /// 创建新的测试实例，使用内存数据库执行指定的 SQL 并存储响应结果
    /// 
    /// # Arguments
    /// * `sql` - 要执行的 SurrealQL 查询语句
    /// 
    /// # Returns
    /// 返回新的 Test 实例，可使用 take_refno_enum 等方法获取结果
    pub async fn new(sql: &str) -> Result<Self> {
        // 初始化内存测试数据库环境
        init_memory_test_surreal().await?;
        
        // 执行 SQL 查询
        let response = SUL_DB.query(sql).await?;
        
        Ok(Self { response })
    }

    /// 从查询结果的指定位置直接获取 RefnoEnum
    /// 
    /// # Arguments
    /// * `index` - 查询结果的索引位置
    /// 
    /// # Returns
    /// 直接返回反序列化后的 RefnoEnum
    pub fn take_refno_enum(&mut self, index: usize) -> Result<RefnoEnum> {
        let refno_enum: Option<RefnoEnum> = self.response.take(index)?;
        refno_enum.ok_or_else(|| anyhow::anyhow!("No RefnoEnum found at index {}", index))
    }

    /// 获取第一个 RefnoEnum 结果（相当于 take_refno_enum(0)）
    pub fn take_first_refno_enum(&mut self) -> Result<RefnoEnum> {
        self.take_refno_enum(0)
    }

    /// 获取 RefnoEnum 数组（适用于多行查询结果）
    pub fn take_refno_enum_vec(&mut self, index: usize) -> Result<Vec<RefnoEnum>> {
        let refno_enums: Vec<RefnoEnum> = self.response.take(index)?;
        Ok(refno_enums)
    }

    /// 检查查询结果的数量
    pub fn check_result_count(&self, expected: usize) -> Result<&Self> {
        assert_eq!(
            self.response.num_results(),
            expected,
            "Unexpected number of results: {} - Expected: {}",
            self.response.num_results(),
            expected
        );
        Ok(self)
    }

    /// 验证指定位置的 RefnoEnum 是否符合预期
    pub fn assert_refno_enum_at(&mut self, index: usize, expected_dbnum: u32, expected_sesno: u32) -> Result<RefnoEnum> {
        let refno_enum = self.take_refno_enum(index)?;
        assert_eq!(refno_enum.refno().get_0(), expected_dbnum, "Database number mismatch");
        assert_eq!(refno_enum.refno().get_1(), expected_sesno, "Session number mismatch");
        Ok(refno_enum)
    }

    /// 验证 RefnoEnum 是否为普通版本（非历史版本）
    pub fn assert_normal_refno_enum(&mut self, index: usize, expected_dbnum: u32, expected_sesno: u32) -> Result<RefnoEnum> {
        let refno_enum = self.assert_refno_enum_at(index, expected_dbnum, expected_sesno)?;
        assert!(!refno_enum.is_history(), "Expected normal RefnoEnum, got history version");
        Ok(refno_enum)
    }

    /// 验证 RefnoEnum 是否为历史版本
    pub fn assert_history_refno_enum(&mut self, index: usize, expected_dbnum: u32, expected_sesno: u32, expected_history_session: u32) -> Result<RefnoEnum> {
        let refno_enum = self.assert_refno_enum_at(index, expected_dbnum, expected_sesno)?;
        assert!(refno_enum.is_history(), "Expected history RefnoEnum, got normal version");
        
        if let crate::pdms_types::RefnoEnum::SesRef(ses_ref) = refno_enum {
            assert_eq!(ses_ref.sesno, expected_history_session, "History session number mismatch");
        } else {
            panic!("RefnoEnum is not a history version");
        }
        
        Ok(refno_enum)
    }

    /// 获取原始的 Response 对象（用于复杂测试场景）
    pub fn into_response(self) -> Response {
        self.response
    }
}

/// 创建测试数据并验证 RefnoEnum 转换的辅助函数
pub async fn create_and_test_pe_with_refno(dbnum: u32, sesno: u32) -> Result<(Test, RefnoEnum)> {
    let refno_str = format!("pe:{}_{}", dbnum, sesno);
    
    let sql = format!(
        r#"
        INSERT INTO pe {{
            id: {},
            noun: 'TEST',
            name: 'Test Equipment'
        }};
        SELECT VALUE id FROM {};
        "#,
        refno_str, refno_str
    );

    let mut test = Test::new(&sql).await?;
    
    // 跳过 INSERT 结果，关注 SELECT 返回的 RefnoEnum
    let refno_enum = test.assert_normal_refno_enum(0, dbnum, sesno)?;
    
    Ok((test, refno_enum))
}

/// 批量创建测试数据并验证 RefnoEnum 数组转换
pub async fn create_multiple_pe_with_refnos(refno_pairs: &[(u32, u32)]) -> Result<(Test, Vec<RefnoEnum>)> {
    if refno_pairs.is_empty() {
        return Err(anyhow::anyhow!("No refno pairs provided"));
    }

    let mut insert_statements = Vec::new();
    let mut refno_strs = Vec::new();

    for (dbnum, sesno) in refno_pairs {
        let refno_str = format!("pe:{}_{}", dbnum, sesno);
        insert_statements.push(format!(
            "INSERT INTO pe {{ id: {}, noun: 'TEST', name: 'Test Equipment' }};",
            refno_str
        ));
        refno_strs.push(format!("{}", refno_str));
    }

    let sql = format!(
        r#"
        {}
        SELECT VALUE id FROM pe WHERE noun = 'TEST' ORDER BY id;
        "#,
        insert_statements.join("\n")
    );

    let mut test = Test::new(&sql).await?;
    
    let refno_enums = test.take_refno_enum_vec(0)?;
    
    // 验证所有 RefnoEnum 的正确性
    assert_eq!(refno_enums.len(), refno_pairs.len());
    
    for (i, (expected_dbnum, expected_sesno)) in refno_pairs.iter().enumerate() {
        if let crate::pdms_types::RefnoEnum::Refno(refno) = &refno_enums[i] {
            assert_eq!(refno.get_0(), *expected_dbnum, "RefnoEnum {} dbnum mismatch", i);
            assert_eq!(refno.get_1(), *expected_sesno, "RefnoEnum {} sesno mismatch", i);
        } else {
            panic!("RefnoEnum {} is not a normal Refno variant", i);
        }
    }
    
    Ok((test, refno_enums))
}

/// 测试 SurrealQL 查询是否能正确返回 RefnoEnum
pub async fn test_basic_select_refno(dbnum: u32, sesno: u32) -> Result<RefnoEnum> {
    let refno_str = format!("pe:{}_{}", dbnum, sesno);
    
    // 首先插入测试数据
    crate::init_test_surreal().await.query(&format!(
        "INSERT INTO pe {{ id: {}, noun: 'TEST', name: 'Test Equipment' }}",
        refno_str
    )).await?;

    let sql = format!("SELECT VALUE id FROM {}", refno_str);
    let mut test = Test::new(&sql).await?;
    
    let refno_enum = test.take_first_refno_enum()?;
    Ok(refno_enum)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_select_refno_conversion() -> Result<()> {
        let _db = init_memory_test_surreal().await?;
        
        // 直接测试简单查询
        let sql = "SELECT VALUE 'pe:17496_123456' as id";
        let mut test = Test::new(sql).await?;
        
        let refno_enum: String = test.response.take(0)?;
        assert_eq!(refno_enum, "pe:17496_123456");
        
        // 清理
        cleanup_memory_test_surreal().await?;
        
        Ok(())
    }

    #[tokio::test]
    async fn test_memory_database_setup() -> Result<()> {
        // 测试内存数据库初始化
        let _db = init_memory_test_surreal().await?;
        
        // 验证数据库连接
        let sql = "RETURN 'test'";
        let mut test = Test::new(sql).await?;
        
        let result: String = test.response.take(0)?;
        assert_eq!(result, "test");
        
        // 清理
        cleanup_memory_test_surreal().await?;
        
        Ok(())
    }

    #[tokio::test]
    async fn test_refno_enum_direct_deserialization() -> Result<()> {
        let _db = init_memory_test_surreal().await?;
        
        // 测试直接反序列化 pe: 格式
        let sql = "SELECT 'pe:17496_123456' as refno";
        let mut test = Test::new(sql).await?;
        
        let refno_str: String = test.response.take(0)?;
        assert_eq!(refno_str, "pe:17496_123456");
        
        // 验证这可以在业务代码中直接反序列化为 RefnoEnum
        let refno_enum: crate::pdms_types::RefnoEnum = serde_json::from_str(&format!("\"{}\"", refno_str))?;
        assert_eq!(refno_enum.refno().get_0(), 17496);
        assert_eq!(refno_enum.refno().get_1(), 123456);
        assert!(!refno_enum.is_history());
        
        // 清理
        cleanup_memory_test_surreal().await?;
        
        Ok(())
    }
}


/// 初始化内存测试数据库
/// 
/// 创建一个独立的内存数据库实例，不依赖外部配置文件
/// 确保测试的独立性和可重复性
pub async fn init_memory_test_surreal() -> Result<()> {
    // 创建配置，启用 AST 格式
    let config = Config::default().ast_payload();

    // 连接到内存数据库
    SUL_DB
        .connect(("memory", config))
        .with_capacity(10) // 测试使用较小的连接池
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to memory database: {}", e))?;

    // 设置测试命名空间和数据库
    SUL_DB
        .use_ns("test")
        .use_db("test")
        .await
        .map_err(|e| anyhow::anyhow!("Failed to set namespace and database: {}", e))?;

    // 以 Root 用户身份登录（内存数据库默认没有密码）
    SUL_DB
        .signin(Root {
            username: "root",
            password: "root",
        })
        .await
        .map_err(|e| anyhow::anyhow!("Failed to sign in to memory database: {}", e))?;

    // 定义常用函数（可选，根据实际需要）
    let _result = define_common_functions().await;

    Ok(())
}

/// 清理内存测试数据库
/// 
/// 重置内存数据库状态，确保测试之间的独立性
pub async fn cleanup_memory_test_surreal() -> Result<()> {
    // 删除所有测试表
    let tables = ["pe", "wosl", "site", "zone", "equi", "pipe"];
    
    for table in &tables {
        let _ = SUL_DB.query(&format!("REMOVE TABLE {}", table)).await;
    }
    
    Ok(())
}

