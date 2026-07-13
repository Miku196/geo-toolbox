-- 001_factor_registry: 排放因子注册表
-- 每条记录有明确的有效年份范围，支持跨年混合因子

CREATE TABLE IF NOT EXISTS factor_registry (
    factor_set_id   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source          TEXT NOT NULL,          -- 'IPCC_2006' | 'IPCC_2019' | '中国省级指南_2023'
    category        TEXT NOT NULL,          -- 'forest' | 'cropland' | 'wetland' | 'grassland' | 'settlement'
    subcategory     TEXT,                   -- 'evergreen_broadleaf' | 'paddy_rice'
    factor_value    DOUBLE PRECISION NOT NULL,
    unit            TEXT NOT NULL DEFAULT 'tCO2e/ha/yr',
    valid_from_year INT NOT NULL,
    valid_to_year   INT,                    -- NULL = 至今有效
    region          TEXT,                   -- 'CN-44' (广东省) | 'global'
    citation        TEXT,                   -- 文献引用
    ingested_at     TIMESTAMPTZ DEFAULT now(),
    dvc_hash        TEXT,                   -- 对应 CSV 文件的 DVC MD5

    -- 同一 source + category + region + 时间窗口不能重叠
    CONSTRAINT uq_factor_validity EXCLUDE USING GIST (
        source WITH =,
        category WITH =,
        COALESCE(region, '__GLOBAL__') WITH =,
        int4range(valid_from_year, COALESCE(valid_to_year, 9999), '[]') WITH &&
    )
);

-- 按来源+年份快速查找适用因子
CREATE INDEX IF NOT EXISTS idx_factor_source_year
    ON factor_registry (source, valid_from_year, valid_to_year);

-- 按土地覆盖类别查找当前有效因子
CREATE INDEX IF NOT EXISTS idx_factor_category_current
    ON factor_registry (category) WHERE valid_to_year IS NULL;

COMMENT ON TABLE factor_registry IS
'排放因子注册表。每条记录有明确的有效年份范围。
碳核算结果通过 factor_set_id UUID 精确引用，支持跨年混合因子。';
