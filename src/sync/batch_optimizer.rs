//! 批量操作优化
//!
//! 提供高效的批量数据操作优化

use crate::db_adapter::{DatabaseAdapter, QueryContext};
use crate::types::*;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 批量操作优化器
pub struct BatchOptimizer {
    /// 批量大小配置
    config: BatchConfig,
    /// 操作缓冲区
    buffer: Arc<RwLock<OperationBuffer>>,
}

/// 批量操作配置
#[derive(Debug, Clone)]
pub struct BatchConfig {
    /// PE批量大小
    pub pe_batch_size: usize,
    /// 属性批量大小
    pub attr_batch_size: usize,
    /// 关系批量大小
    pub relation_batch_size: usize,
    /// 缓冲区大小
    pub buffer_size: usize,
    /// 自动刷新阈值
    pub auto_flush_threshold: f64,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            pe_batch_size: 100,
            attr_batch_size: 500,
            relation_batch_size: 1000,
            buffer_size: 10000,
            auto_flush_threshold: 0.8,
        }
    }
}

/// 操作缓冲区
#[derive(Debug, Default)]
struct OperationBuffer {
    /// PE写入缓冲
    pe_writes: Vec<SPdmsElement>,
    /// 属性写入缓冲
    attr_writes: HashMap<RefnoEnum, NamedAttrMap>,
    /// 关系写入缓冲
    relation_writes: Vec<(RefnoEnum, RefnoEnum, String)>,
}

impl BatchOptimizer {
    /// 创建批量优化器
    pub fn new(config: BatchConfig) -> Self {
        Self {
            config,
            buffer: Arc::new(RwLock::new(OperationBuffer::default())),
        }
    }

    /// 添加PE到缓冲区
    pub async fn buffer_pe(&self, pe: SPdmsElement) -> Result<()> {
        let mut buffer = self.buffer.write().await;
        buffer.pe_writes.push(pe);

        // 检查是否需要自动刷新
        if buffer.pe_writes.len() >= self.config.pe_batch_size {
            drop(buffer);
            self.flush_pe_buffer().await?;
        }

        Ok(())
    }

    /// 添加属性到缓冲区
    pub async fn buffer_attributes(&self, refno: RefnoEnum, attmap: NamedAttrMap) -> Result<()> {
        let mut buffer = self.buffer.write().await;
        buffer.attr_writes.insert(refno, attmap);

        // 检查是否需要自动刷新
        if buffer.attr_writes.len() >= self.config.attr_batch_size {
            drop(buffer);
            self.flush_attr_buffer().await?;
        }

        Ok(())
    }

    /// 添加关系到缓冲区
    pub async fn buffer_relation(
        &self,
        from: RefnoEnum,
        to: RefnoEnum,
        rel_type: String,
    ) -> Result<()> {
        let mut buffer = self.buffer.write().await;
        buffer.relation_writes.push((from, to, rel_type));

        // 检查是否需要自动刷新
        if buffer.relation_writes.len() >= self.config.relation_batch_size {
            drop(buffer);
            self.flush_relation_buffer().await?;
        }

        Ok(())
    }

    /// 刷新PE缓冲区
    async fn flush_pe_buffer(&self) -> Result<()> {
        let mut buffer = self.buffer.write().await;
        if buffer.pe_writes.is_empty() {
            return Ok(());
        }

        let pes = std::mem::take(&mut buffer.pe_writes);
        drop(buffer);

        // TODO: 实际批量写入操作
        log::debug!("批量写入 {} 个PE", pes.len());

        Ok(())
    }

    /// 刷新属性缓冲区
    async fn flush_attr_buffer(&self) -> Result<()> {
        let mut buffer = self.buffer.write().await;
        if buffer.attr_writes.is_empty() {
            return Ok(());
        }

        let attrs = std::mem::take(&mut buffer.attr_writes);
        drop(buffer);

        // TODO: 实际批量写入操作
        log::debug!("批量写入 {} 个属性集", attrs.len());

        Ok(())
    }

    /// 刷新关系缓冲区
    async fn flush_relation_buffer(&self) -> Result<()> {
        let mut buffer = self.buffer.write().await;
        if buffer.relation_writes.is_empty() {
            return Ok(());
        }

        let relations = std::mem::take(&mut buffer.relation_writes);
        drop(buffer);

        // TODO: 实际批量写入操作
        log::debug!("批量写入 {} 个关系", relations.len());

        Ok(())
    }

    /// 刷新所有缓冲区
    pub async fn flush_all(&self) -> Result<()> {
        self.flush_pe_buffer().await?;
        self.flush_attr_buffer().await?;
        self.flush_relation_buffer().await?;
        Ok(())
    }

    /// 获取缓冲区状态
    pub async fn get_buffer_status(&self) -> BufferStatus {
        let buffer = self.buffer.read().await;
        BufferStatus {
            pe_count: buffer.pe_writes.len(),
            attr_count: buffer.attr_writes.len(),
            relation_count: buffer.relation_writes.len(),
        }
    }
}

/// 缓冲区状态
#[derive(Debug, Clone)]
pub struct BufferStatus {
    pub pe_count: usize,
    pub attr_count: usize,
    pub relation_count: usize,
}

impl BufferStatus {
    /// 获取总缓冲项数
    pub fn total_count(&self) -> usize {
        self.pe_count + self.attr_count + self.relation_count
    }
}

/// 批量读取优化器
pub struct BatchReader {
    /// 读取配置
    config: BatchReadConfig,
    /// 预读缓存
    prefetch_cache: Arc<RwLock<HashMap<RefnoEnum, SPdmsElement>>>,
}

/// 批量读取配置
#[derive(Debug, Clone)]
pub struct BatchReadConfig {
    /// 预读大小
    pub prefetch_size: usize,
    /// 并行读取数
    pub parallel_reads: usize,
}

impl Default for BatchReadConfig {
    fn default() -> Self {
        Self {
            prefetch_size: 100,
            parallel_reads: 4,
        }
    }
}

impl BatchReader {
    /// 创建批量读取器
    pub fn new(config: BatchReadConfig) -> Self {
        Self {
            config,
            prefetch_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 批量预读PE
    pub async fn prefetch_pes(
        &self,
        refnos: &[RefnoEnum],
        adapter: &Arc<dyn DatabaseAdapter>,
    ) -> Result<()> {
        let ctx = QueryContext::default();
        let mut cache = self.prefetch_cache.write().await;

        // 分批并行读取
        for chunk in refnos.chunks(self.config.prefetch_size) {
            let mut handles = Vec::new();

            for &refno in chunk {
                let adapter = adapter.clone();
                let ctx = ctx.clone();

                let handle = tokio::spawn(async move { adapter.get_pe(refno, Some(ctx)).await });
                handles.push((refno, handle));
            }

            // 收集结果
            for (refno, handle) in handles {
                if let Ok(Ok(Some(pe))) = handle.await {
                    cache.insert(refno, pe);
                }
            }
        }

        Ok(())
    }

    /// 从缓存获取PE
    pub async fn get_from_cache(&self, refno: RefnoEnum) -> Option<SPdmsElement> {
        let cache = self.prefetch_cache.read().await;
        cache.get(&refno).cloned()
    }

    /// 清除预读缓存
    pub async fn clear_cache(&self) {
        let mut cache = self.prefetch_cache.write().await;
        cache.clear();
    }
}

/// 批量操作事务
pub struct BatchTransaction {
    /// 操作列表
    operations: Vec<BatchOperation>,
    /// 是否已提交
    committed: bool,
}

/// 批量操作类型
#[derive(Debug, Clone)]
enum BatchOperation {
    WritePE(SPdmsElement),
    WriteAttributes(RefnoEnum, NamedAttrMap),
    WriteRelation(RefnoEnum, RefnoEnum, String),
    DeletePE(RefnoEnum),
}

impl BatchTransaction {
    /// 创建新事务
    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
            committed: false,
        }
    }

    /// 添加写入PE操作
    pub fn add_write_pe(&mut self, pe: SPdmsElement) {
        self.operations.push(BatchOperation::WritePE(pe));
    }

    /// 添加写入属性操作
    pub fn add_write_attributes(&mut self, refno: RefnoEnum, attmap: NamedAttrMap) {
        self.operations
            .push(BatchOperation::WriteAttributes(refno, attmap));
    }

    /// 添加写入关系操作
    pub fn add_write_relation(&mut self, from: RefnoEnum, to: RefnoEnum, rel_type: String) {
        self.operations
            .push(BatchOperation::WriteRelation(from, to, rel_type));
    }

    /// 添加删除PE操作
    pub fn add_delete_pe(&mut self, refno: RefnoEnum) {
        self.operations.push(BatchOperation::DeletePE(refno));
    }

    /// 提交事务
    pub async fn commit(&mut self, adapter: &Arc<dyn DatabaseAdapter>) -> Result<()> {
        if self.committed {
            return Err(anyhow::anyhow!("事务已提交"));
        }

        // 按操作类型分组
        let mut pe_writes = Vec::new();
        let mut attr_writes = Vec::new();
        let mut relation_writes = Vec::new();
        let mut pe_deletes = Vec::new();

        for op in &self.operations {
            match op {
                BatchOperation::WritePE(pe) => pe_writes.push(pe.clone()),
                BatchOperation::WriteAttributes(refno, attmap) => {
                    attr_writes.push((*refno, attmap.clone()));
                }
                BatchOperation::WriteRelation(from, to, rel_type) => {
                    relation_writes.push((*from, *to, rel_type.clone()));
                }
                BatchOperation::DeletePE(refno) => pe_deletes.push(*refno),
            }
        }

        // 批量执行操作
        for pe in pe_writes {
            adapter.save_pe(&pe).await?;
        }

        for (refno, attmap) in attr_writes {
            adapter.save_attmap(refno, &attmap).await?;
        }

        for (from, to, rel_type) in relation_writes {
            adapter.create_relation(from, to, &rel_type).await?;
        }

        for refno in pe_deletes {
            adapter.delete_pe(refno).await?;
        }

        self.committed = true;
        Ok(())
    }

    /// 回滚事务
    pub fn rollback(&mut self) {
        self.operations.clear();
        self.committed = false;
    }

    /// 获取操作数量
    pub fn operation_count(&self) -> usize {
        self.operations.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_batch_optimizer() {
        let optimizer = BatchOptimizer::new(BatchConfig::default());

        let pe = SPdmsElement::default();
        optimizer.buffer_pe(pe).await.unwrap();

        let status = optimizer.get_buffer_status().await;
        assert_eq!(status.pe_count, 1);
    }

    #[tokio::test]
    async fn test_batch_reader() {
        let reader = BatchReader::new(BatchReadConfig::default());

        let refno = RefnoEnum::from(RefU64(123));
        let cached = reader.get_from_cache(refno).await;
        assert!(cached.is_none());
    }

    #[test]
    fn test_batch_transaction() {
        let mut tx = BatchTransaction::new();

        let pe = SPdmsElement::default();
        tx.add_write_pe(pe);

        assert_eq!(tx.operation_count(), 1);
    }
}
