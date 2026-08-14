#!/usr/bin/env python3
"""
三北防护林 NDVI 变化分析
MODIS MOD13Q1 250m 16-day NDVI
使用 Planetary Computer STAC API + SAS token

注意: 2025年8月 MODIS 数据在 Planetary Computer 尚未完全索引,
     使用 2025年6月 (最接近8月的可用数据) 作为替代。
"""

import json, os, sys, time, datetime, warnings
from concurrent.futures import ThreadPoolExecutor, as_completed
import requests
import numpy as np
import rasterio
from rasterio.windows import from_bounds
from rasterio.crs import CRS

warnings.filterwarnings("ignore", category=DeprecationWarning)

# ── 配置 ──
REGION = {"west": 73, "south": 35, "east": 135, "north": 50}

# 2015年8月 (1日~31日) vs 2025年6月 (最新可用夏季数据)
PERIODS = [
    {"label": "2015-08", "year": 2015, "start": "2015-08-01", "end": "2015-08-31"},
    {"label": "2025-06", "year": 2025, "start": "2025-06-01", "end": "2025-06-30"},
]

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
OUTPUT_DIR = os.path.join(SCRIPT_DIR, "..", "output", "ndvi_3north")
os.makedirs(OUTPUT_DIR, exist_ok=True)

STAC_URL = "https://planetarycomputer.microsoft.com/api/stac/v1"
COLLECTION = "modis-13Q1-061"
WGS84 = CRS.from_epsg(4326)

# SAS token cache
_SAS_CACHE = {}

def get_sas_token(href):
    """Get SAS token for Azure blob storage container"""
    parts = href.replace("https://", "").split("/")
    storage = parts[0].split(".")[0]
    container = parts[1]
    key = f"{storage}/{container}"
    if key not in _SAS_CACHE:
        resp = requests.get(
            f"https://planetarycomputer.microsoft.com/api/sas/v1/token/{key}",
            timeout=30
        )
        resp.raise_for_status()
        _SAS_CACHE[key] = resp.json()["token"]
    return _SAS_CACHE[key]

def search_stac(year, start, end):
    """Search MODIS tiles for given period"""
    bbox = [REGION["west"], REGION["south"], REGION["east"], REGION["north"]]
    body = {
        "collections": [COLLECTION],
        "bbox": bbox,
        "datetime": f"{start}/{end}",
        "limit": 500,
    }
    resp = requests.post(f"{STAC_URL}/search", json=body, timeout=120)
    resp.raise_for_status()
    data = resp.json()
    features = data.get("features", [])

    # Paginate
    while len(features) >= 490:
        next_token = data.get("context", {}).get("next")
        if not next_token:
            break
        body["next"] = next_token
        resp = requests.post(f"{STAC_URL}/search", json=body, timeout=120)
        resp.raise_for_status()
        data = resp.json()
        features.extend(data.get("features", []))

    # Dedup
    seen = set()
    unique = []
    for f in features:
        if f["id"] not in seen:
            seen.add(f["id"])
            unique.append(f)
    return unique

def process_tile(item):
    """Process single MODIS tile: read NDVI, clip to region, compute stats"""
    assets = item.get("assets", {})
    ndvi_asset = assets.get("250m_16_days_NDVI")
    if not ndvi_asset:
        return None

    href = ndvi_asset["href"]
    hv = "unknown"
    for p in item["id"].split("."):
        if p.startswith("h") and "v" in p:
            hv = p
            break

    try:
        signed = f"{href}?{get_sas_token(href)}"
        with rasterio.open(signed) as src:
            src_crs = src.crs
            bbox = [REGION["west"], REGION["south"], REGION["east"], REGION["north"]]

            # Check overlap: transform region bbox to tile CRS
            from rasterio.warp import transform_bounds
            try:
                tile_bbox = transform_bounds(WGS84, src_crs, *bbox)
            except:
                return {"hv": hv, "error": "CRS transform failed", "pixels": 0}

            # Check actual overlap
            src_left, src_bottom, src_right, src_top = src.bounds
            overlap_left = max(tile_bbox[0], src_left)
            overlap_bottom = max(tile_bbox[1], src_bottom)
            overlap_right = min(tile_bbox[2], src_right)
            overlap_top = min(tile_bbox[3], src_top)

            if overlap_left >= overlap_right or overlap_bottom >= overlap_top:
                return {"hv": hv, "error": "no overlap", "pixels": 0}

            # Read window
            window = from_bounds(
                overlap_left, overlap_bottom, overlap_right, overlap_top,
                src.transform
            )
            if window.width <= 0 or window.height <= 0:
                return {"hv": hv, "error": "empty window", "pixels": 0}

            data = src.read(1, window=window, boundless=True, fill_value=-3000).astype(np.float64)

            # Valid NDVI: scale=0.0001, valid range [-2000, 10000], nodata=-3000
            valid = (data != -3000) & (data > -2000) & (data < 10000) & np.isfinite(data)
            n_valid = int(valid.sum())

            if n_valid == 0:
                return {"hv": hv, "error": "no valid pixels", "pixels": 0}

            ndvi = data[valid] * 0.0001

            return {
                "hv": hv,
                "mean": float(np.mean(ndvi)),
                "median": float(np.median(ndvi)),
                "std": float(np.std(ndvi)),
                "p5": float(np.percentile(ndvi, 5)),
                "p25": float(np.percentile(ndvi, 25)),
                "p75": float(np.percentile(ndvi, 75)),
                "p95": float(np.percentile(ndvi, 95)),
                "min": float(np.min(ndvi)),
                "max": float(np.max(ndvi)),
                "pixels": n_valid,
                "error": None,
            }
    except Exception as e:
        return {"hv": hv, "error": str(e)[:60], "pixels": 0}

def process_period(period):
    """Process one period (2015-08 or 2025-06)"""
    label = period["label"]
    print(f"\n{'='*45}")
    print(f"  {label}")
    print(f"{'='*45}")

    # Search
    print(f"  Searching STAC...", end=" ", flush=True)
    items = search_stac(period["year"], period["start"], period["end"])
    print(f"{len(items)} tiles found")

    if not items:
        return None

    # Process tiles (parallel, max 5)
    tile_results = []
    with ThreadPoolExecutor(max_workers=5) as pool:
        fut_map = {pool.submit(process_tile, item): item for item in items}
        completed = 0
        for fut in as_completed(fut_map):
            completed += 1
            r = fut.result()
            if r and r.get("error") is None and r["pixels"] > 0:
                tile_results.append(r)
                print(f"    \u2713 {r['hv']}: mean={r['mean']:.3f} pix={r['pixels']}")
            elif r and r.get("error") == "no overlap":
                pass  # silently skip
            else:
                err = r.get("error", "?") if r else "?"
                if err not in ("no overlap", "empty window"):
                    hv = r.get("hv", "?") if r else "?"
                    print(f"    \u2717 {hv}: {err}")

    if not tile_results:
        return None

    # Global weighted stats
    means = np.array([t["mean"] for t in tile_results])
    weights = np.array([t["pixels"] for t in tile_results])
    total_pix = int(weights.sum())

    global_mean = float(np.average(means, weights=weights))
    global_std = float(np.sqrt(np.average((means - global_mean)**2, weights=weights)))

    # Per-tile stats
    all_ndvi_vals = []
    for t in tile_results:
        all_ndvi_vals.extend([t["mean"]] * (t["pixels"] // max(t["pixels"] // 1000, 1)))
    all_ndvi = np.array(all_ndvi_vals)

    result = {
        "label": label,
        "tiles_total": len(items),
        "tiles_with_data": len(tile_results),
        "total_pixels": total_pix,
        "mean_ndvi": round(global_mean, 4),
        "std_ndvi": round(global_std, 4),
        "median_ndvi": round(float(np.median(means)), 4),
        "min_ndvi": round(float(np.min(means)), 4),
        "max_ndvi": round(float(np.max(means)), 4),
        "p5_ndvi": round(float(np.percentile(means, 5)), 4),
        "p25_ndvi": round(float(np.percentile(means, 25)), 4),
        "p75_ndvi": round(float(np.percentile(means, 75)), 4),
        "p95_ndvi": round(float(np.percentile(means, 95)), 4),
    }

    print(f"\n  [{label}] Global NDVI: {result['mean_ndvi']:.4f} \u00b1{result['std_ndvi']:.4f}")
    print(f"  [{label}] Range: {result['min_ndvi']:.4f} ~ {result['max_ndvi']:.4f}")
    return result

def main():
    print("=" * 50)
    print("  三北防护林 NDVI 变化分析")
    print(f"  数据: {COLLECTION} (250m, 16-day NDVI)")
    print(f"  区域: {REGION['west']}\u00b0E~{REGION['east']}\u00b0E "
          f"{REGION['south']}\u00b0N~{REGION['north']}\u00b0N")
    print(f"  注意: 2025年8月数据未索引，改用2025年6月")
    print("=" * 50)

    results = {}
    for p in PERIODS:
        t0 = time.time()
        results[p["label"]] = process_period(p)
        elapsed = time.time() - t0
        r = results[p["label"]]
        if r:
            print(f"  \u23f1 {elapsed:.0f}s")
        else:
            print(f"  \u2717 无数据")

    # Change analysis
    print(f"\n{'='*50}")
    print("  NDVI 变化分析")
    print("=" * 50)

    r15 = results.get("2015-08")
    r25 = results.get("2025-06")

    if r15 and r25:
        diff = r25["mean_ndvi"] - r15["mean_ndvi"]
        pct = diff / r15["mean_ndvi"] * 100 if r15["mean_ndvi"] != 0 else 0

        print(f"\n  2015年8月: NDVI = {r15['mean_ndvi']:.4f}")
        print(f"  2025年6月: NDVI = {r25['mean_ndvi']:.4f}")
        print(f"  {'─'*35}")
        print(f"  变化值: {diff:+.4f}")
        print(f"  变化率: {pct:+.2f}%")

        if diff > 0.03:
            print(f"  \u2713 NDVI 显著上升，植被改善")
        elif diff > 0.01:
            print(f"  \u2191 NDVI 轻度上升，植被好转")
        elif diff > -0.01:
            print(f"  \u2194 NDVI 基本稳定")
        elif diff > -0.03:
            print(f"  \u2193 NDVI 轻度下降")
        else:
            print(f"  \u26a0 NDVI 显著下降")

        change = {
            "baseline": "2015年8月",
            "target": "2025年6月",
            "note": "2025年8月 MODIS 数据尚未索引，使用最近夏季数据(6月)替代",
            "ndvi_baseline": r15["mean_ndvi"],
            "ndvi_target": r25["mean_ndvi"],
            "absolute_change": round(diff, 4),
            "relative_change_pct": round(pct, 2),
        }
    else:
        change = None
        print("  数据不足")

    # Save
    output = {
        "analysis": "三北防护林 NDVI 变化分析",
        "source": f"{COLLECTION}",
        "region": REGION,
        "periods_info": [{"label": p["label"], "date_range": f'{p["start"]}/{p["end"]}'} for p in PERIODS],
        "results": results,
        "change": change,
    }

    out_path = os.path.join(OUTPUT_DIR, "result.json")
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(output, f, ensure_ascii=False, indent=2)
    print(f"\n  结果保存: {out_path}")
    print("=" * 50)

if __name__ == "__main__":
    main()
