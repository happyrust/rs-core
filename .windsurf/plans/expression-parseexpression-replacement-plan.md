# 用 Rust 复刻 core.dll 的“DB 二进制表达式 payload 解释/格式化”算法并替换表达式解析
 
 本计划通过 **IDA 反编译还原 DLL 中“二进制表达式 payload → 表达式树/后缀序列 → 格式化字符串(pretty)”** 的完整算法，在 Rust 中实现与 DLL **1:1 对齐的 payload 解释、错误行为与输出风格**，从而替换当前 `parse_pdms_db` 的手写表达式 payload 解码/格式化逻辑，且**不在运行时调用 `core.dll`**。
 
 ## 背景与现状
 - 现有实现：`parse_pdms_db/src/parse.rs::parse_raw_explicit_attrs` 对部分属性（`check_is_expr(hash)` 或 `force_expr`）会走 `parse_explict_tools::parse_expression_attr`（二进制 token 流 0x65/0x6A/0x76…）解码为字符串。
 - 目标行为：对齐 `core.dll` 中 `DLL_PMLCommand::ParseExpression` 的实现。
 - 说明：你选择的输入形态为 **B（DB 二进制表达式 payload）**，因此真正需要复刻的是 DLL 对 payload 的解释与格式化链路（更接近 `DBE_Builder::getExpressionTree` 一类逻辑），而不是“文本→token”的 `ParseExpression`。
 
 ## 已确认需求（必须 Rust 复刻，禁止运行时调用 DLL）
 1. **实现方式**：不在 Rust 运行时调用 `core.dll`，而是 **基于反编译在 Rust 中复刻** DLL 的 payload 解释/格式化算法。
 2. **一致性要求**：对同一 payload，输出字符串（pretty）与 DLL 的格式风格 **完全一致**（包括括号/空格/函数名风格等）。
 3. **失败策略**：严格模式（遇到未覆盖/非法 payload 即报错/中断）。
 
 ## 前置条件（必须满足，否则无法落地）
 - 需要一批 **对照样本**：表达式 payload（二进制 bytes/以 4 字节 word 对齐的十六进制串）+ 期望 pretty 输出（以及可选的中间结构，如 opcode 序列/AST）。
 - 说明：对照样本可来自现有 `test_cases/test_expression.rs`、运行日志、或你的分析记录；即使需要用 DLL/Hook 采样 pretty，也仅用于生成测试夹具，不会作为运行时依赖。
 
 ## 方案总览
 ### 方案核心
 - **把 DLL 的 payload 解释链路规格化**：围绕 `DBE_Builder::getExpressionTree`（以及 opcode 分发）抽取：
   - payload 的段结构/终止符/嵌套规则
   - opcode 分类（参考现有 `src/parser/attribute/opcode.rs` 的 `opcode/100` 分发）
   - 运算符/函数/属性引用/字符串/数值的还原规则
   - 错误与边界行为
 - **Rust 复刻实现**：
   - decoder：把 payload 解析为中间表示（AST 或 postfix + operand stack）
   - formatter：把中间表示格式化为 DLL 风格 pretty 字符串
 
 ### 关键决策
 - **对齐基准**：以同一 payload 在 DLL 中产生的 pretty 输出作为“真值”，Rust 输出必须逐字符对齐。
 - **覆盖策略**：先覆盖当前程序中已出现/已生成的表达式形态（`ATTRIB...`、`IFTRUE`、`MAX/MIN`、比较/算术、字符串字面量等），再扩展到更少见 opcode。
 
 ## Implementation Plan（里程碑）
 ### 1) 固化对照基准（payload & pretty 以 DLL 输出为准）
 - 准备一组表达式样本（优先使用仓库已有的 payload 样本，如 `src/test_cases/test_expression.rs`）。
 - 为每个样本收集：
   - payload bytes（或 hex 串）
   - 期望 pretty（来自你已有输出/日志，或从 DLL 侧采样后固化为夹具）
 - 保存为测试夹具（后续 Rust 回归逐字符比对）。
 
 ### 2) 反编译调研与规格化
 - 在 IDA 中围绕 payload 解释/格式化链路梳理：
   - `DBE_Builder::getExpressionTree` 与其 opcode 分发
   - value/attribute/string/func/operator 等节点的编码与消费方式
   - 终止符（例如 `0x06A5`）与嵌套片段的结构
 - 输出一份“可实现规格”：
   - payload 段结构与解析状态机
   - opcode→节点语义映射表
   - pretty 输出规则（括号与空格策略）
   - 错误处理与边界行为
 
 ### 3) Rust 实现：payload 反编译与格式化入口
 - 新增模块（建议直接落在现有表达式解析区域）：
   - `parser/attribute/expression_payload.rs`（或同级模块）
 - 提供 API：
   - `decode_expression_payload(payload: &[u8], refno: u64) -> Result<String>`（返回 DLL 风格 pretty）
 - 失败策略：严格模式（遇到未知 opcode/非法结构直接报错）。
 
 ### 4) 接入现有解析链路（替换点）
 - 替换点：`parse_pdms_db/src/parse.rs::parse_raw_explicit_attrs` 的表达式分支。
 - 直接替换为：payload → pretty（严格对齐 DLL 风格）。
 
 ### 5) 回归验证与性能评估
 - 新增单测：对照样本 payload，断言 pretty 与 DLL 一致。
 - 集成回归：用现有 examples/用例验证表达式相关字段输出稳定。
 
 ## 风险与应对
 - **风险：缺少真值样本**：若没有足够的 DLL pretty 输出样本，难以保证逐字符对齐。
 - **风险：payload 覆盖面大**：需要按出现频率分阶段补齐 opcode/段结构。
 
 ## 首批覆盖范围（以当前程序已出现/已生成文本形态为准）
 - **属性引用**：`ATTRIB <NAME>`、`ATTRIB <NAME>[n ]`、`ATTRIB <NAME>[<expr> ]`、以及带 `RPRO` 的变体
 - **常量与字面量**：数值常量、`PI`、字符串字面量 `'...'`
 - **算术/比较/布尔**：`+ - * /`、`EQ/NEQ/GT/LT/GE/LE`、`NOT/AND/OR`
 - **函数**：`SQRT/SIN/COS/TAN/ASIN/ACOS/ATAN/ATANT`、`MAX/MIN`、`INT/NINT/ABS/LOG/ALOG/POW`
 - **字符串/转换**：`LEN/TRIM/MAT/OCCUR/REAL/STR`、`UPCASE/LOWCASE/SUBSTRING/REPLACE/...`
 - **控制/通用函数**：`IFTRUE(...)`、`DISTCONVERT(...)`、`SET/UNSET/ARRAY/EMPTY/SPLIT`
 - **特殊结构**：`( <expr> OF <FUNC> )`、轴向/坐标类（如 `X/Y/Z`、`NEG`）
