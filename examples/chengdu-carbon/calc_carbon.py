#!/usr/bin/env python3
"""
成都开发区碳收支变化评估
=========================
方法: IPCC 排放因子法 (Tier 1)
数据: 基于公开统计年鉴和土地变更调查的估算值
      排放因子来源 IPCC 2006/2019 + 四川省碳排放核算指南 2023

计算逻辑:
  碳储量损失  = Σ (转出面积 × 原覆被碳密度)
  持续排放    = Σ (工业面积 × 年排放因子)
  剩余碳汇    = Σ (保留面积 × 碳汇因子)
  净碳收支    = 碳储量损失 + 持续排放 - 剩余碳汇

注意: 土地利用数据为基于公开文献的估算值，非实测数据。
      仅用于方法学演示，不构成正式碳核算报告。
"""

import csv
import json
from collections import defaultdict
from pathlib import Path

DATA_DIR = Path(__file__).parent

# ── 加载排放因子 ──────────────────────────────────

def load_factors(path: Path, source: str = "IPCC_2019") -> dict:
    """加载排放因子表，返回 {(category,subcategory): factor_value}"""
    factors = {}
    with open(path, encoding="utf-8") as f:
        for row in csv.DictReader(f):
            if row["source"] == source:
                key = (row["category"], row["subcategory"])
                factors[key] = float(row["factor_value"])
    return factors

# ── 碳收支计算 ──────────────────────────────────

def calc_carbon_budget(transitions: list[dict], factors: dict[tuple[str, str], float]) -> dict[str, float | list[dict]]:
    """计算单个开发区的碳收支。

    Args:
        transitions: 土地利用转换记录列表。
        factors: 排放因子表 {(category, subcategory): value}。

    Returns:
        包含 stock_loss, ongoing_emission, remaining_sink, net_budget, detail 的字典。
    """
    stock_loss = 0.0      # 碳储量损失 (一次性, tCO₂)
    ongoing_emission = 0.0 # 持续排放 (每年, tCO₂/yr)
    remaining_sink = 0.0   # 剩余碳汇 (每年, tCO₂/yr)
    detail = []

    for t in transitions:
        area = float(t["area_ha"])
        cat_from = (t["landcover_baseline"], "")
        cat_to = (t["landcover_target"], "")

        # 匹配排放因子 - 查找源覆被的碳密度
        factor_from = 0.0
        for (cat, sub), val in factors.items():
            if cat == t["landcover_baseline"]:
                if sub == "" or sub in t.get("notes", ""):
                    factor_from = val
                    break
        if factor_from == 0.0:
            # fallback: match just the category
            for (cat, sub), val in factors.items():
                if cat == t["landcover_baseline"] and sub != "":
                    factor_from = val
                    break

        factor_to = 0.0
        for (cat, sub), val in factors.items():
            if cat == t["landcover_target"]:
                factor_to = val
                break

        if t["landcover_baseline"] != t["landcover_target"]:
            # 覆被变化: 计算碳储量损失(一次性)
            if factor_from < 0:  # 原覆被是碳汇
                loss = area * abs(factor_from)
                stock_loss += loss
            elif factor_to > 0:  # 目标覆被是排放源
                pass  # handled below

            if factor_to > 0:  # 目标覆被是排放源 (工业)
                ongoing_emission += area * factor_to

            detail.append({
                "transition": t["conversion"],
                "area_ha": area,
                "stock_loss_tco2": area * abs(factor_from) if factor_from < 0 else 0,
                "annual_emission_tco2_per_yr": area * factor_to if factor_to > 0 else 0,
            })
        else:
            # 覆被不变
            if factor_from < 0:  # 保留的碳汇
                remaining_sink += area * factor_from
            if factor_to > 0:
                ongoing_emission += area * factor_to

            detail.append({
                "transition": t["conversion"],
                "area_ha": area,
                "stock_loss_tco2": 0,
                "annual_sink_tco2_per_yr": area * factor_from if factor_from < 0 else 0,
                "annual_emission_tco2_per_yr": area * factor_to if factor_to > 0 else 0,
            })

    net = stock_loss + ongoing_emission + remaining_sink  # remaining_sink is negative

    return {
        "stock_loss_tco2": stock_loss,
        "ongoing_emission_tco2_per_yr": ongoing_emission,
        "remaining_sink_tco2_per_yr": remaining_sink,
        "net_budget_tco2": net,
        "net_per_ha_tco2_per_ha": net / sum(float(t["area_ha"]) for t in transitions),
        "detail": detail,
    }

# ── 读取土地覆被变化数据 ─────────────────────────

def load_transitions(path: Path) -> dict[str, list[dict]]:
    """加载开发区转换数据。"""
    """按开发区组织土地覆被变化数据"""
    zones = defaultdict(list)
    with open(path, encoding="utf-8") as f:
        for row in csv.DictReader(f):
            zones[row["zone_id"]].append(row)
    return zones

# ── 加载 AOI 元数据 ─────────────────────────────

def load_zone_meta(geojson_path: Path) -> dict[str, str]:
    """从 GeoJSON 加载开发区元数据。"""
    """从 GeoJSON 读取开发区元数据"""
    with open(geojson_path, encoding="utf-8") as f:
        data = json.load(f)
    meta = {}
    for feat in data["features"]:
        zone_id = feat["id"]
        props = feat["properties"]
        meta[zone_id] = {
            "name": props["name"],
            "name_cn": props["name_cn"],
            "area_km2": props["area_km2"],
            "baseline_year": props["baseline_year"],
            "target_year": props["target_year"],
        }
    return meta

# ── 主计算 ──────────────────────────────────────

def main():
    factors = load_factors(DATA_DIR / "emission-factors.csv", source="IPCC_2019")
    zones_trans = load_transitions(DATA_DIR / "landcover-transition.csv")
    zones_meta = load_zone_meta(DATA_DIR / "chengdu-zones.geojson")

    print("=" * 72)
    print("  成都主要开发区 碳收支变化评估")
    print("  方法: IPCC 排放因子法 (Tier 1)")
    print("  因子来源: IPCC 2019 Refinement")
    print("=" * 72)

    total_stock_loss = 0
    total_ongoing = 0
    total_sink = 0

    for zone_id in ["cd-gaoxin", "cd-tianfu", "cd-jingkai", "cd-dongbu"]:
        meta = zones_meta[zone_id]
        transitions = zones_trans[zone_id]
        budget = calc_carbon_budget(transitions, factors)

        total_area = sum(float(t["area_ha"]) for t in transitions)
        converted_area = sum(float(t["area_ha"]) for t in transitions
                            if t["landcover_baseline"] != t["landcover_target"])

        print(f"\n{'─' * 60}")
        print(f"  {meta['name_cn']} ({meta['name']})")
        print(f"  面积: {meta['area_km2']} km² | 基线: {meta['baseline_year']} → 目标: {meta['target_year']}")
        print(f"{'─' * 60}")
        print(f"  {'覆被变化':<24} {'面积(ha)':>10} {'碳储量损失':>14} {'年排放':>12} {'年碳汇':>12}")
        print(f"  {'─' * 60}")

        for d in budget["detail"]:
            loss = d.get("stock_loss_tco2", 0)
            emis = d.get("annual_emission_tco2_per_yr", 0)
            sink = d.get("annual_sink_tco2_per_yr", 0)
            t_name = d["transition"][:22]
            print(f"  {t_name:<24} {d['area_ha']:>10.0f} {loss:>14,.0f} {emis:>12,.0f} {sink:>12,.0f}")

        print(f"  {'─' * 60}")

        # 总览
        co2_kt = budget["stock_loss_tco2"] / 1000
        ong_kt = budget["ongoing_emission_tco2_per_yr"] / 1000
        sink_kt = budget["remaining_sink_tco2_per_yr"] / 1000
        net_kt = budget["net_budget_tco2"] / 1000

        print(f"  • 碳储量一次性损失:  {co2_kt:>10,.1f} ktCO₂")
        print(f"  • 持续年排放(工业):   {ong_kt:>10,.1f} ktCO₂/yr")
        print(f"  • 剩余碳汇年吸收:     {sink_kt:>10,.1f} ktCO₂/yr")
        print(f"  • 净碳收支(一次性):   {net_kt:>10,.1f} ktCO₂")
        print(f"  • 开发面积占比:       {converted_area/total_area*100:>10.1f}%")

        total_stock_loss += budget["stock_loss_tco2"]
        total_ongoing += budget["ongoing_emission_tco2_per_yr"]
        total_sink += budget["remaining_sink_tco2_per_yr"]

    # ── 全市汇总 ──
    print(f"\n{'═' * 72}")
    print(f"  成都市开发区碳收支汇总")
    print(f"{'═' * 72}")
    print(f"  碳储量一次性损失:  {total_stock_loss/1e6:>10.2f} MtCO₂")
    print(f"  持续年排放:        {total_ongoing/1e6:>10.2f} MtCO₂/yr")
    print(f"  剩余碳汇年吸收:    {total_sink/1e6:>10.2f} MtCO₂/yr")
    net_yearly = total_ongoing - abs(total_sink)
    print(f"  年净排放(持续性):  {net_yearly/1e6:>10.2f} MtCO₂/yr")
    print(f"  (一次性碳储量损失 {total_stock_loss/1e6:.2f} MtCO₂ 相当于")
    print(f"   成都市 {total_stock_loss/1e6/18:.1f} 年的年净排放)")
    print(f"\n  ⚠ 数据基于公开统计估算，仅作方法学演示")
    print(f"  ⚠ 正式核算需实测土地覆被矢量 + GEE 遥感验证")

if __name__ == "__main__":
    main()
