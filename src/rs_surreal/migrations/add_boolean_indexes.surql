-- 布尔运算查询优化索引
-- 创建日期: 2025-12-01
-- 目的: 优化 neg_relate 和 ngmr_relate 相关的查询性能

-- 1. inst_relate 表索引优化
-- 用于快速查找正实体基础信息
DEFINE INDEX idx_inst_relate_in ON TABLE inst_relate COLUMNS in;
DEFINE INDEX idx_inst_relate_bad_bool ON TABLE inst_relate COLUMNS bad_bool;
DEFINE INDEX idx_inst_relate_aabb ON TABLE inst_relate COLUMNS aabb;
DEFINE INDEX idx_inst_relate_world_trans ON TABLE inst_relate COLUMNS world_trans;

-- 2. neg_relate 表索引优化
-- 用于快速查找负实体到正实体的关系
DEFINE INDEX idx_neg_relate_out ON TABLE neg_relate COLUMNS out;
DEFINE INDEX idx_neg_relate_in ON TABLE neg_relate COLUMNS in;
DEFINE INDEX idx_neg_relate_composite ON TABLE neg_relate COLUMNS out, in;

-- 3. ngmr_relate 表索引优化
-- 用于快速查找 NGMR 关系
DEFINE INDEX idx_ngmr_relate_out ON TABLE ngmr_relate COLUMNS out;
DEFINE INDEX idx_ngmr_relate_in ON TABLE ngmr_relate COLUMNS in;
DEFINE INDEX idx_ngmr_relate_composite ON TABLE ngmr_relate COLUMNS out, in;

-- 4. geo_relate 表索引优化
-- 用于快速查找几何类型和变换
DEFINE INDEX idx_geo_relate_geo_type ON TABLE geo_relate COLUMNS geo_type;
DEFINE INDEX idx_geo_relate_trans ON TABLE geo_relate COLUMNS trans;
DEFINE INDEX idx_geo_relate_geom_refno ON TABLE geo_relate COLUMNS geom_refno;
DEFINE INDEX idx_geo_relate_out ON TABLE geo_relate COLUMNS out;
DEFINE INDEX idx_geo_relate_composite ON TABLE geo_relate COLUMNS geo_type, trans;

-- 5. pe 表索引优化
-- 用于快速查找存在负实体关系的正实体
DEFINE INDEX idx_pe_neg_relate_in ON TABLE pe COLUMNS in;
DEFINE INDEX idx_pe_ngmr_relate_in ON TABLE pe COLUMNS in;

-- 6. inst_info 表索引优化
-- 用于快速关联实例信息
DEFINE INDEX idx_inst_info_geo_relate_out ON TABLE inst_info COLUMNS out;