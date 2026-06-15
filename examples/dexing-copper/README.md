# 德兴铜矿生态修复效果评估

基于 Sentinel-2 MSI 影像（10m 分辨率）的 NDVI 差值分析与碳汇变化评估。

## 流程

1. **下载 Sentinel-2 影像** — 2020 和 2025 年 6-8 月，云量 < 10%
2. **NDVI 计算** — (NIR - RED) / (NIR + RED)
3. **NDVI 差值分析** — 2025 - 2020，评估植被恢复
4. **碳汇变化评估** — 基于 NDVI → 植被覆盖度 → 碳储量转换
5. **报告生成** — 带评分、图表、DXF 导出

## 运行

```bash
pip install numpy matplotlib geopandas ezdxf scikit-image rasterio
python dexing_copper_assessment.py
```

## 输出

- `output/dexing_copper_report.md` — 评估报告
- `output/dexing_ndvi_2020.png` — 2020 NDVI 图
- `output/dexing_ndvi_2025.png` — 2025 NDVI 图  
- `output/dexing_ndvi_change.png` — NDVI 差值图
- `output/dexing_restoration_zones.dxf` — 修复区 DXF
- `output/dexing_assessment.json` — 结构化评估数据
