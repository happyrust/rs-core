# 任务进展

- [x] 优化 `inst_relate` 查询逻辑，提高效率，减少内存占用并在 `rs-core` 中实现。
- [x] 在 `rs-core` 中移除 `old_pe` 和 `old_refno` 相关逻辑。
- [x] 修改 `GeomInstQuery` 和 `TubiInstQuery` 结构体，移除冗余字段。
- [x] 更新 `inst.rs` 中的 SurrealQL 查询。

## 下一步计划

- [ ] 验证变更在实际环境中的运行情况。
