-- 调试 BEND 24381/46958 的 SQL 查询脚本

-- 查询 1: 检查 BEND 的基本信息
SELECT 
    'BEND 基本信息' as query_name,
    id,
    refno,
    noun,
    name
FROM pe:⟨24381_46958⟩;

-- 查询 2: 检查 BEND 的所有子元素
SELECT 
    'BEND 子元素' as query_name,
    id,
    refno,
    noun,
    name
FROM (
    SELECT VALUE in FROM pe:⟨24381_46958⟩<-pe_owner
)
ORDER BY noun, refno;

-- 查询 3: 检查 SSLC 子元素及其关键属性
SELECT 
    'SSLC 子元素属性' as query_name,
    id,
    refno,
    noun,
    refno.PXTS as top_x_shear,
    refno.PYTS as top_y_shear,
    refno.PXBS as btm_x_shear,
    refno.PYBS as btm_y_shear,
    refno.PDIA as diameter,
    refno.PHEI as height,
    refno.PAXI as axis,
    refno.PDIS as dist_to_btm
FROM (
    SELECT VALUE in FROM pe:⟨24381_46958⟩<-pe_owner WHERE in.noun = 'SSLC'
)
ORDER BY refno;

-- 查询 4: 检查第一个 SSLC 的完整属性
SELECT 
    'SSLC 完整属性' as query_name,
    *
FROM (
    SELECT VALUE in FROM pe:⟨24381_46958⟩<-pe_owner WHERE in.noun = 'SSLC' LIMIT 1
)[0].refno;

-- 查询 5: 检查是否有 SSL (不是 SSLC) 类型的子元素
SELECT 
    'SSL 子元素' as query_name,
    id,
    refno,
    noun,
    name
FROM (
    SELECT VALUE in FROM pe:⟨24381_46958⟩<-pe_owner WHERE in.noun = 'SSL'
)
ORDER BY refno;

-- 查询 6: 检查所有几何类型的子元素
SELECT 
    '所有几何子元素' as query_name,
    id,
    refno,
    noun,
    name
FROM (
    SELECT VALUE in FROM pe:⟨24381_46958⟩<-pe_owner 
    WHERE in.noun IN ['SBOX', 'SCYL', 'SSPH', 'LCYL', 'SCON', 'LSNO', 'LPYR', 'SDSH', 'SCTO', 'SEXT', 'SREV', 'SRTO', 'SSLC']
)
ORDER BY noun, refno;

-- 查询 7: 检查两层深度的所有子元素
SELECT 
    'Level 1 子元素' as level,
    id,
    refno,
    noun,
    name
FROM (
    SELECT VALUE in FROM pe:⟨24381_46958⟩<-pe_owner
)
UNION ALL
SELECT 
    'Level 2 孙子元素' as level,
    id,
    refno,
    noun,
    name
FROM (
    SELECT VALUE in FROM array::flatten(
        (SELECT VALUE in FROM pe:⟨24381_46958⟩<-pe_owner)<-pe_owner
    )
)
ORDER BY level, noun, refno;

