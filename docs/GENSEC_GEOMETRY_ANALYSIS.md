# GENSEC 几何体生成与变换分析报告

## 1. 概述

基于 IDA Pro 对 core.dll 的逆向工程分析，本文档详细阐述了 GENSEC (General Section) 实体的几何体生成机制和坐标变换（Transform）逻辑。

## 2. 坐标变换 (Transform) 生成机制

GENSEC 的世界变换矩阵生成主要依赖于以下核心函数：

### 2.1 核心变换函数

*   **GSTRAM (Get Structural TRAnsformation Matrix)**
    *   **地址**: `0x100b0c22`
    *   **功能**: 计算源元素相对于目标元素的绝对变换矩阵。
    *   **逻辑**:
        1.  调用 `GATRAR` 获取源元素的绝对变换矩阵。
        2.  调用 `GATRAR` 获取目标元素的绝对变换矩阵。
        3.  调用 `INTRAM` 计算逆矩阵。
        4.  调用 `CONCAT` 进行矩阵乘法，计算相对变换。
    *   **关键子函数**:
        *   `GATRAR`: 获取绝对变换数组 (Get Absolute TRAnsformation Array)。
        *   `INTRAM`: 计算逆矩阵 (INverse TRAnsformation Matrix)。
        *   `CONCAT`: 矩阵乘法。
        *   底层 Fortran 接口: `DSAVE`, `DGOTO`, `DGETI`, `DGETF`, `DRESTO` 用于数据库游标操作。

*   **GTORI1 (Get ORIentation)**
    *   **地址**: `0x106852ec`
    *   **功能**: 将 PDMS 方向字符串（如 "N 45 E", "Y is U"）转换为 3x3 旋转矩阵。
    *   **逻辑**: 解析方向描述符，计算基向量，并构建正交旋转矩阵。

*   **TRAVCI (TRAnsform Vector Coordinates)**
    *   **地址**: `0x10687028`
    *   **功能**: 将变换矩阵应用到 3D 向量/点。
    *   **逻辑**: 执行标准的 `v' = M * v + T` 变换。

### 2.2 变换流程

对于 GENSEC 上的任意点（如 POINSP），其世界坐标计算流程如下：

1.  **局部定义**: 读取 POINSP 的局部属性（如 `POS`, `ORI`）。
2.  **父级变换**: 调用 `GSTRAM` 获取 GENSEC 相对于 World 的变换矩阵。
3.  **应用变换**: 调用 `TRAVCI` 将 POINSP 的局部坐标变换为世界坐标。

## 3. 几何体生成 (Geometry Generation)

GENSEC 的几何体是通过将 2D 截面（Section）沿 3D 路径（Spine）挤出（Extrude）或放样（Loft）生成的。

### 3.1 关键组件

*   **SPINE (路径)**
    *   **标识符**: 全局变量 `?NOUN_SPINE@@3QBVDB_Noun@@B` (由 `sub_1091CC20` 初始化)。
    *   **定义**: 由一系列 `POINSP` (Point in Spine) 和可能的 `CURVE` 元素定义。
    *   **作用**: 定义挤出路径的中心线和切向/法向变化。

*   **P-Line (截面)**
    *   **处理类**: `DBE_Pline` (相关函数如 `transformPos` @ `0x1052febe`)。
    *   **定义**: 定义了 GENSEC 的横截面形状。
    *   **逻辑**: `DBE_Pline::transformPos` 展示了如何将截面上的点变换到空间中。它首先定位到 Owner (GENSEC/SPINE)，获取变换矩阵，然后应用变换。

### 3.2 生成逻辑推断

基于分析，GENSEC 的几何体生成步骤如下：

1.  **获取路径**: 解析 `SPINE` 元素，提取所有 `POINSP` 的位置和方向。
    *   利用 `GTORI1` 解析每个点的方向（尤其是 `ORI` 属性）。
    *   利用插值算法（如 Hermite 或 Bezier，取决于 `CURVE` 类型）计算路径上的中间点。

2.  **获取截面**: 解析关联的 P-Line 定义（可能是引用 catalogue 中的定义）。

3.  **构建骨架**:
    *   沿 `SPINE` 路径，计算一系列变换矩阵（Frenet Frame 或由 POINSP 的 `ORI` 明确指定的 Frame）。
    *   `GSTRAM` 在此过程中可能被多次调用以处理层级变换。

4.  **生成网格**:
    *   将 2D 截面变换到路径上的每个采样点。
    *   连接相邻截面的对应点，生成侧面网格（Extrusion/Loft）。
    *   `TRAVCI` 被大量用于顶点变换。

## 4. 结论

GENSEC 的几何核心在于 **`GSTRAM`** (变换矩阵生成) 和 **`TRAVCI`** (坐标变换)。几何形状由 **SPINE** (路径) 和 **P-Line** (截面) 共同定义。

在重构或复现 GENSEC 逻辑时，必须准确实现：
1.  `GSTRAM` 的层级矩阵累乘逻辑。
2.  `GTORI1` 的方向解析逻辑（特别是 PDMS 特有的方向语法）。
3.  沿 SPINE 路径的坐标系插值算法。

---
**分析工具**: IDA Pro + mcp-server
**分析对象**: core.dll
**日期**: 2025-11-23
