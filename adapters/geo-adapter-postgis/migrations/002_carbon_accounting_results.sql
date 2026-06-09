-- 002_carbon_accounting_results: 碳核算结果表
-- 每条记录绑定精确的排放因子行 (factor_set_id UUID)

CREATE TABLE IF NOT EXISTS carbon_accounting_results (
    calc_id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_run_id  UUID NOT NULL,
    calculation_at   TIMESTAMPTZ DEFAULT now(),

    -- 空间范围
    aoi_id           UUID NOT NULL,
    geometry         GEOMETRY(MultiPolygon, 4326),
    area_ha          DOUBLE PRECISION,

    -- 土地覆盖来源
    landcover_src    TEXT NOT NULL,
    landcover_class  TEXT NOT NULL,
    lc_dvc_hash      TEXT,

    -- 排放因子引用 (行级 UUID)
    factor_set_id    UUID NOT NULL REFERENCES factor_registry(factor_set_id),

    -- 计算结果
    emission_tco2e   DOUBLE PRECISION NOT NULL,
    confidence_low   DOUBLE PRECISION,
    confidence_high  DOUBLE PRECISION,

    -- 审计字段: geo-toolbox 写入, Pi Agent 更新
    audit_status     TEXT NOT NULL DEFAULT 'pending'
        CHECK (audit_status IN ('pending', 'in_review', 'approved', 'rejected')),
    auditor_id       TEXT,
    approved_at      TIMESTAMPTZ,
    rejection_reason TEXT,

    created_at       TIMESTAMPTZ DEFAULT now()
);

-- 空间索引
CREATE INDEX IF NOT EXISTS idx_carbon_geom
    ON carbon_accounting_results USING GIST (geometry);

-- 工作流追溯
CREATE INDEX IF NOT EXISTS idx_carbon_workflow
    ON carbon_accounting_results (workflow_run_id);

-- 按 AOI + 时间查询
CREATE INDEX IF NOT EXISTS idx_carbon_aoi_time
    ON carbon_accounting_results (aoi_id, calculation_at DESC);

-- 因子引用索引
CREATE INDEX IF NOT EXISTS idx_carbon_factor
    ON carbon_accounting_results (factor_set_id);

COMMENT ON TABLE carbon_accounting_results IS
'碳核算结果表。审计链: lc_dvc_hash → 遥感分类版本,
 factor_set_id → 排放因子行, auditor_id → 审核人。';
