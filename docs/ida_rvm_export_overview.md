# IDA 反编译：RVM 导出相关函数整理

> **背景**：以下函数均位于 `core.dll`，由 `MODIFY COPY` (`MODCPY`) 命令驱动，用于调用 `mcopy.dll` 的 `McpTransferMgr::Export` 生成 RVM 文件，并同步输出 DATAL 脚本。  
> **命名说明**：给出的“建议名称”在足够理解逻辑后可替换原 `sub_xxxxxxxx` 符号，便于交叉对照与进一步分析。

## 顶层流程

- **0x101DA35C – `MODCPY`**
  - **建议名称**：`dispatch_modify_copy_command`
  - **功能概述**：解析 `MODIFY COPY` 指令的各类子命令编号（`*a2`），维护导出模式标志位（`dword_10E9F020/024/028`），根据不同编号调用下游准备函数，最后在导出模式下触发实际的导出或打开流程。
  - **关键调用**：`sub_100239E0`、`sub_10024380`、`sub_10024580` 等预处理；`sub_10024310` → `McpTransferMgr::Export`。

- **0x10024310 – `sub_10024310`**
  - **建议名称**：`perform_review_export`
  - **功能概述**：从缓冲区读取导出路径，构造 `std::string`，调用 `McpTransferMgr::Instance()` 与 `McpTransferMgr::Export()`（在 `mcopy.dll`），真正启动 RVM 文件写入。
  - **关键点**：清理本地字符串缓存，处理长度大于 0x10 的 SSO 场景。

- **0x10024260 – `sub_10024260`**
  - **建议名称**：`open_transfer_session`
  - **功能概述**：与 `McpTransferMgr::Open()` 交互，用于导入或更新模式；构造传输集名称后返回是否成功打开。

## MODCPY 内部辅助

- **0x100239E0 – `sub_100239E0`**
  - **建议名称**：`trigger_import_mode`
  - **功能概述**：简单代理 `McpTransferMgr::Instance()` → `Import()`，用于命令编号 6（导入）场景。

- **0x10024380 – `sub_10024380`**
  - **建议名称**：`show_selection_and_prompt`
  - **功能概述**：调用 `McpImpSelectedItems::ShowSelection()` 展示现有选择集，并在检测到特定环境标志时输出 MR 消息提示用户确认。

- **0x10024580 – `sub_10024580`**
  - **建议名称**：`enqueue_selected_element`
  - **功能概述**：将当前元素加入待处理集合（`unk_10A393BC` 等），供后续导出时使用；常在命令编号 0x0B 分支被调用。

- **0x10023B40 – `sub_10023B40`**
  - **建议名称**：`finalize_export_view`
  - **功能概述**：在开启 Review 视图导出时，刷新或关闭当前导出视图的内部状态。

- **0x10024B30 – `sub_10024B30`**
  - **建议名称**：`apply_view_transfer`
  - **功能概述**：当用户选择“导出视图 + 保持层级”时，将 MR 消息与元素列表同步到导出集合。

- **0x10024E10 – `sub_10024E10`**
  - **建议名称**：`apply_secondary_transfer`
  - **功能概述**：处理第二套导出策略（如 Secondary 集合），在 MR 输出中写入相应命令。

- **0x10024690 – `sub_10024690`**
  - **建议名称**：`flush_attribute_buffer`
  - **功能概述**：把 `byte_10E9CC20` 中的属性字符串拷贝到 RVM/DATAL 输出需要的缓冲区 (`dword_10E9F02C`)，处理长度与末尾终止符。

## 过滤与替换逻辑

- **0x10023F40 – `sub_10023F40`**
  - **建议名称**：`prepare_owner_substitute`
  - **功能概述**：判断 `McpTransferMgr::IsExporting()`，若在导出状态则调用 `McpSetupMgr::GetTransferItemSetup()` 查找元素的 owner 替代关系，并将结果写入输出缓冲；保留需要 fallback 的元素 hash。

- **0x10025230 – `sub_10025230`**
  - **建议名称**：`prepare_element_export_record`
  - **功能概述**：针对元素名字、路径、层级等做一致性检查，必要时调用 `DB_Element::getAtt()`、`sub_10023B90()` 等过滤非法名称，决定是否允许该元素进入最终导出。

- **0x10025A90 – `sub_10025A90`**
  - **建议名称**：`filter_exportable_attributes`
  - **功能概述**：利用 `DB_Attribute_findAttribute_by_id()`、`DB_Attribute::hashName()` 获取属性信息，只允许 FONT 等受支持属性导出；对不支持的属性设置替换标志。

## DATAL 文档生成链路

- **0x10277854 – `sub_10277854`**
  - **建议名称**：`write_datal_input_header`
  - **功能概述**：输出 DATAL 脚本头部（`INPUT BEGIN$/`），初始化列表上下文（`PCRTEL`、`sub_1027AA90`），随后调用元素与层级遍历函数。

- **0x1027B654 – `sub_1027B654`**
  - **建议名称**：`emit_element_blocks`
  - **功能概述**：遍历导出元素（含层级、父子关系），生成 `!!EL.n` 系列行、NAME/TYPE/OWNER 等字段；对无效元素进行清理并记录在 `deferredDeleteRef`。

- **0x1027C530 – `sub_1027C530`**
  - **建议名称**：`emit_attribute_blocks`
  - **功能概述**：在已有元素列表基础上写入属性、连接、额外引用；配合 `filter_exportable_attributes` 确保只输出合法字段。

- **0x1027CDE0 – `sub_1027CDE0`**
  - **建议名称**：`emit_deferred_deletes`
  - **功能概述**：遍历延迟删除集合，输出诸如 `!!CE = !deferredDeleteRef[i]`、`DELETE $!!CE.type` 的尾部清理语句。

- **0x1027D190 – `sub_1027D190`**
  - **建议名称**：`emit_connection_stub`
  - **功能概述**：当导出选项要求包含连接时，输出 `$( <index>.$)` 等连接标记，并把连接信息写回暂存结构供后续细化。

- **0x1027D2DC – `sub_1027D2DC`**
  - **建议名称**：`emit_full_datal_payload`
  - **功能概述**：分配大体积缓冲（约 13KB），使用 `IOC/IOS` 族函数拼接 DATAL 主体，大部分脚本文本在此输出。

- **0x10277C80 – `sub_10277C80`**
  - **建议名称**：`run_listing_iterator`
  - **功能概述**：统筹 DATAL 文档生成的主循环：处理输入参数、决定是否输出连接块、调用 `sub_1027B654`/`sub_1027C530`/`sub_1027CDE0`/`sub_1027D190` 等，最终输出完整的列表脚本。

- **0x1027FD78 – `sub_1027FD78`**
  - **建议名称**：`emit_textual_listings`
  - **功能概述**：处理元素名称或文本属性相关的 DATAL 输出（`datal/DTLELN`），配合 `ATNTXT`/`UDSOFT` 把 PDMS 字段转为脚本内容。

- **0x102802F8 – `sub_102802F8`**
  - **建议名称**：`emit_relationship_listings`
  - **功能概述**：输出层级关系或父子引用的相关脚本片段，通常在 `run_listing_iterator` 中按特定选项调用。

- **0x1028242C – `sub_1028242C`**
  - **建议名称**：`emit_preview_listings`
  - **功能概述**：当需要导出 Preview/Secondary 集合时输出对应 DATAL 段落，结构与 `emit_element_blocks` 类似但针对不同数据源。

## 备注

- 所有 `emit_*` 函数都会检查 `McpTransferMgr::IsExporting()` 或命令旗标，以确保在导入模式下不会输出 RVM 专属内容。
- `IOC/IOS/IOIM/IOIW` 等调用族来自 AVEVA 自身的输出框架，主要负责将字符串写入 Shell 或 DATAL 输出文件；分析脚本时可借此定位实际文案。
- 若后续需要修改或复刻导出行为，可以建议名称作为 Hook/重写入口，并参考此文档确定依赖顺序。

## 导出流程概览

```mermaid
flowchart TD
    A[用户执行<br/>MODIFY COPY 指令] --> B[dispatch_modify_copy_command<br/>(解析子命令, 设置模式标志 dword_10E9F020/024/028)]
    B -->|case 2 (EXPORT)| C[show_selection_and_prompt<br/>/ enqueue_selected_element<br/>/ apply_view_transfer...]
    C --> D[run_listing_iterator<br/>(输出 DATAL 主循环)]
    D --> D1[emit_element_blocks]
    D --> D2[emit_attribute_blocks]
    D --> D3[emit_connection_stub]
    D --> D4[emit_preview_listings / emit_relationship_listings]
    D --> D5[emit_deferred_deletes]
    D5 --> E[emit_full_datal_payload<br/>(整合脚本)]
    E --> F[过滤/替换逻辑<br/>prepare_owner_substitute<br/>prepare_element_export_record<br/>filter_exportable_attributes]
    F -->|case 0x0F, dword_10E9F020==1| G[perform_review_export<br/>(构造路径, 调用 McpTransferMgr::Export)]
    F -->|case 0x0F, dword_10E9F020==2| H[open_transfer_session<br/>(McpTransferMgr::Open)]
    G --> I[RVM 文件输出<br/>(mcopy.dll 内部实现)]
    H --> I
```

> **说明**：图中省略了部分辅助分支（如导入模式 `trigger_import_mode`），重点呈现进入导出模式后的主要函数调用、DATAL 文档生成与最终触发 `McpTransferMgr::Export` 的关系。
