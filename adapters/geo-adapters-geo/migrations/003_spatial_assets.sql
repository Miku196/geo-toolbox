-- 003_spatial_assets: 统一空间资产表
-- 矢量数据存 geometry 列，栅格数据存 raster_index (MinIO 路径 + 元数据)

CREATE TABLE IF NOT EXISTS spatial_assets (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    aoi_id          UUID,
    source          TEXT NOT NULL,          -- 'CamoFox' | 'GEE_RF' | 'manual_survey'
    asset_type      TEXT NOT NULL DEFAULT 'vector'
        CHECK (asset_type IN ('vector', 'raster_index')),
    geom            GEOMETRY(Geometry, 4326),
    properties      JSONB DEFAULT '{}',
    file_path       TEXT,                   -- MinIO/S3 路径
    crs             TEXT NOT NULL DEFAULT 'EPSG:4326',
    dvc_hash        TEXT,
    ingested_at     TIMESTAMPTZ DEFAULT now(),
    ingested_by     TEXT DEFAULT 'geo-toolbox'
);

-- 空间索引 (核心: 所有空间查询的入口)
CREATE INDEX IF NOT EXISTS idx_spatial_geom
    ON spatial_assets USING GIST (geom);

-- AOI 分组 + 时间排序
CREATE INDEX IF NOT EXISTS idx_spatial_aoi
    ON spatial_assets (aoi_id, ingested_at DESC);

-- 数据源追溯
CREATE INDEX IF NOT EXISTS idx_spatial_source
    ON spatial_assets (source, asset_type);

-- JSONB 属性索引 (按 landcover class 筛选)
CREATE INDEX IF NOT EXISTS idx_spatial_class
    ON spatial_assets USING BTREE ((properties->>'class'))
    WHERE properties->>'class' IS NOT NULL;

COMMENT ON TABLE spatial_assets IS
'统一空间资产表。所有写入经过 geo-store BatchWriter 攒批 COPY 写入。';
