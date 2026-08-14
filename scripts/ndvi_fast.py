#!/usr/bin/env python3
"""
三北防护林 NDVI 变化分析
用 toolbox 管线：STAC 搜索 → SAS 签名 → rasterio 读取 → NDVI 统计
快速版 — 聚焦核心区域
"""
import json, os, sys, time, warnings
from concurrent.futures import ThreadPoolExecutor, as_completed
import requests, numpy as np, rasterio
from rasterio.windows import from_bounds
from rasterio.warp import transform_bounds
warnings.filterwarnings("ignore")

OUT = os.path.join(os.path.dirname(__file__), "..", "output", "ndvi_3north")
os.makedirs(OUT, exist_ok=True)

# 三北核心区：黄土高原 + 内蒙古 + 华北 (北纬35-45, 东经100-120)
BBOX = [100, 35, 120, 45]
STAC = "https://planetarycomputer.microsoft.com/api/stac/v1"
COL = "modis-13Q1-061"

PERIODS = [
    ("2015-08", 2015, "2015-08-01", "2015-08-31"),
    ("2025-06", 2025, "2025-06-01", "2025-06-30"),
]

def sas_token(href):
    p = href.replace("https://","").split("/")
    k = f"{p[0].split('.')[0]}/{p[1]}"
    r = requests.get(f"https://planetarycomputer.microsoft.com/api/sas/v1/token/{k}", timeout=30)
    return f"{href}?{r.json()['token']}"

def search(year, start, end):
    body = {"collections":[COL], "bbox":BBOX, "datetime":f"{start}/{end}", "limit":500}
    r = requests.post(f"{STAC}/search", json=body, timeout=120)
    return r.json().get("features", [])

def proc_tile(item):
    ndvi_a = item.get("assets", {}).get("250m_16_days_NDVI")
    if not ndvi_a: return None
    href = ndvi_a["href"]
    hv = next((p for p in item["id"].split(".") if p.startswith("h") and "v" in p), "?")
    try:
        signed = sas_token(href)
        with rasterio.open(signed) as src:
            # Transform WGS84 bbox to tile CRS
            wgs84 = rasterio.CRS.from_epsg(4326)
            tbbox = transform_bounds(wgs84, src.crs, BBOX[0], BBOX[1], BBOX[2], BBOX[3])
            # Compute window in tile CRS
            w = from_bounds(tbbox[0], tbbox[1], tbbox[2], tbbox[3], src.transform)
            # Clip to dataset extent
            w = w.intersection(rasterio.windows.Window(0, 0, src.width, src.height))
            if w.width <= 0 or w.height <= 0: return None
            d = src.read(1, window=w).astype(np.float64)
            ok = (d != -3000) & (d > -2000) & (d < 10000) & np.isfinite(d)
            n = int(ok.sum())
            if n == 0: return None
            ndvi = d[ok] * 0.0001
            return {"hv":hv, "mean":float(np.mean(ndvi)), "std":float(np.std(ndvi)),
                    "p5":float(np.percentile(ndvi,5)), "p95":float(np.percentile(ndvi,95)),
                    "pixels":n}
    except Exception as e:
        return None

def run():
    print("="*50)
    print("  三北防护林 NDVI 变化")
    print("  MODIS MOD13Q1 250m NDVI")
    print(f"  区域: {BBOX[0]}E~{BBOX[2]}E {BBOX[1]}N~{BBOX[3]}N")
    print("="*50)

    results = {}
    for label, year, start, end in PERIODS:
        print(f"\n--- {label} ---")
        t0 = time.time()
        items = search(year, start, end)
        print(f"  搜索: {len(items)} tiles")
        if not items: continue

        tile_data = []
        with ThreadPoolExecutor(max_workers=6) as pool:
            for fut in as_completed({pool.submit(proc_tile, i): i for i in items}):
                r = fut.result()
                if r:
                    tile_data.append(r)

        if not tile_data: continue
        means = np.array([t["mean"] for t in tile_data])
        w = np.array([t["pixels"] for t in tile_data])
        gm = float(np.average(means, weights=w))
        gs = float(np.sqrt(np.average((means - gm)**2, weights=w)))

        res = {"label":label, "tiles":len(tile_data), "pixels":int(w.sum()),
               "mean":round(gm,4), "std":round(gs,4),
               "min":round(float(np.min(means)),4), "max":round(float(np.max(means)),4)}
        results[label] = res
        print(f"  NDVI: {res['mean']:.4f} ±{res['std']:.4f} [{len(tile_data)} tiles, {time.time()-t0:.0f}s]")

    # 变化
    print(f"\n{'='*50}\n  变化分析\n{'='*50}")
    r15, r25 = results.get("2015-08"), results.get("2025-06")
    if r15 and r25:
        d = r25["mean"] - r15["mean"]
        p = d / r15["mean"] * 100
        print(f"  2015-08: {r15['mean']:.4f}")
        print(f"  2025-06: {r25['mean']:.4f}")
        print(f"  变化: {d:+.4f} ({p:+.2f}%)")
        msg = "显著改善 ✓" if d > 0.03 else ("轻度改善 ↑" if d > 0.01 else
               ("稳定 →" if d > -0.01 else ("轻度退化 ↓" if d > -0.03 else "显著退化 ⚠")))
        print(f"  结论: NDVI {msg}")
        change = {"baseline":"2015-08","target":"2025-06","ndvi_2015":r15["mean"],
                  "ndvi_2025":r25["mean"],"abs_change":round(d,4),"rel_change":round(p,2),
                  "note":"2025年8月MODIS未索引，改用6月数据"}
    else:
        change = None; print("  数据不足")

    out = {"analysis":"三北防护林NDVI变化","source":COL,"bbox":BBOX,"results":results,"change":change}
    p = os.path.join(OUT, "result.json")
    with open(p,"w",encoding="utf-8") as f: json.dump(out,f,ensure_ascii=False,indent=2)
    print(f"\n  保存: {p}")

if __name__ == "__main__": run()
