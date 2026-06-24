#!/usr/bin/env python3
"""
geo-toolbox: 自包含 HTML 报告生成器
将地震/洪水管线的 PNG + GeoJSON + 统计数据打包为单文件 HTML，
双击即可在浏览器中查看，无需服务器。
"""

import base64
import json
import sys
from pathlib import Path
from datetime import datetime

OUTPUT_DIR = Path(__file__).resolve().parent / "output"
GEO_TOOLBOX = Path(__file__).resolve().parent.parent.parent

def b64_image(path: Path) -> str:
    """将图片文件编码为 base64 字符串。"""
    with open(path, "rb") as f:
        return base64.b64encode(f.read()).decode()

def generate_html(report_type: str, title: str, maps: list[Path],
                  geojson: Path | None = None, stats: dict | None = None) -> Path:
    """生成自包含 HTML 报告。

    Args:
        report_type: 报告类型标识 ("seismic" / "flood")。
        title: 报告标题。
        maps: PNG 地图文件路径列表。
        geojson: 可选 GeoJSON 数据文件路径。
        stats: 可选统计数据字典。

    Returns:
        生成的 HTML 文件路径。
    """
    
    map_b64 = {}
    for mp in maps:
        if mp.exists():
            map_b64[mp.stem] = b64_image(mp)
    
    # 内嵌纯 JS CRS 变换，无需外部 WASM 文件
    
    images_html = ""
    for name, data in map_b64.items():
        images_html += f"""
    <div class="card">
      <h2>{name}</h2>
      <img src="data:image/png;base64,{data}" style="max-width:100%;border-radius:4px">
    </div>"""
    
    stats_html = ""
    if stats:
        rows = ""
        for lvl in ["极高风险", "高风险", "中风险", "低风险", "极低风险"]:
            s = stats.get(lvl, {})
            rows += f"<tr><td>{lvl}</td><td>{s.get('cells','-')}</td><td>{s.get('area_10k_km2','-'):.1f}</td><td>{s.get('pct','-'):.1f}%</td></tr>"
        stats_html = f"""
    <div class="card">
      <h2>📊 风险统计</h2>
      <table><thead><tr><th>等级</th><th>网格数</th><th>面积(万km²)</th><th>占比</th></tr></thead>
      <tbody>{rows}</tbody></table>
    </div>"""
    
    html = f"""<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>{title}</title>
<style>
*{{box-sizing:border-box;margin:0;padding:0}}
body{{font-family:'Microsoft YaHei',sans-serif;background:#0f172a;color:#e2e8f0;line-height:1.6}}
.header{{background:linear-gradient(135deg,#1e3a5f,#1e293b);padding:30px 20px;text-align:center;border-bottom:3px solid #2563eb}}
.header h1{{font-size:26px;margin-bottom:6px}}
.header p{{color:#94a3b8;font-size:13px}}
.container{{max-width:1200px;margin:0 auto;padding:20px}}
.card{{background:#1e293b;border:1px solid #334155;border-radius:10px;padding:20px;margin-bottom:20px}}
.card h2{{font-size:16px;margin-bottom:12px;color:#93c5fd}}
table{{width:100%;border-collapse:collapse;font-size:13px}}
th,td{{padding:8px 12px;text-align:left;border-bottom:1px solid #334155}}
th{{color:#94a3b8;font-weight:600}}
.footer{{text-align:center;padding:20px;color:#64748b;font-size:12px;border-top:1px solid #334155;margin-top:20px}}
.toolbox{{margin-top:10px;font-size:12px;color:#60a5fa}}
.wasm-section{{margin-top:30px}}
</style>
</head>
<body>
<div class="header">
  <h1>🌍 {title}</h1>
  <p>生成时间: {datetime.now().strftime('%Y-%m-%d %H:%M')} | 工具链: Camoufox + geo-toolbox + Python GIS</p>
  <p class="toolbox">🔧 geo-toolbox: <code>crs transform</code> (纯Rust) | <code>crs list</code> | <code>output geojson --from-file</code></p>
</div>

<div class="container">
  {stats_html}
  {images_html}

  <div class="wasm-section">
    <div class="card">
      <h2>🧪 交互式坐标变换 (geo-wasm)</h2>
      <p style="color:#94a3b8;font-size:12px;margin-bottom:10px">WGS84 ↔ Web Mercator ↔ GCJ-02 火星坐标 ↔ BD-09 百度坐标 — 纯浏览器计算，零数据外传</p>
      <div style="display:flex;gap:8px;flex-wrap:wrap;align-items:center">
        <select id="wasmFrom" style="padding:6px;background:#0f172a;color:#e2e8f0;border:1px solid #334155;border-radius:4px">
          <option value="4326">EPSG:4326 WGS84</option>
          <option value="3857">EPSG:3857 Web Mercator</option>
          <option value="9000">EPSG:9000 GCJ-02</option>
          <option value="9001">EPSG:9001 BD-09</option>
        </select>
        <span>→</span>
        <select id="wasmTo" style="padding:6px;background:#0f172a;color:#e2e8f0;border:1px solid #334155;border-radius:4px">
          <option value="3857">EPSG:3857 Web Mercator</option>
          <option value="4326">EPSG:4326 WGS84</option>
          <option value="9000">EPSG:9000 GCJ-02</option>
          <option value="9001">EPSG:9001 BD-09</option>
        </select>
        <input id="wasmX" value="104.06" style="width:100px;padding:6px;background:#0f172a;color:#e2e8f0;border:1px solid #334155;border-radius:4px">
        <input id="wasmY" value="30.57" style="width:100px;padding:6px;background:#0f172a;color:#e2e8f0;border:1px solid #334155;border-radius:4px">
        <button id="wasmBtn" style="padding:6px 16px;background:#2563eb;color:#fff;border:none;border-radius:4px;cursor:pointer">变换</button>
        <span id="wasmResult" style="color:#22c55e;font-family:monospace;font-size:13px"></span>
      </div>
    </div>
  </div>
</div>

<div class="footer">
  <p>本报告由 geo-toolbox 自动生成 | <a href="http://127.0.0.1:8899/demo.html" style="color:#60a5fa">WASM Demo</a></p>
</div>

<script>
// 内嵌纯 JS CRS 变换 (不依赖 WASM，保证离线可用)
const DEG2RAD=Math.PI/180, R=6378137;
function wgs84ToMerc(lon,lat){{return[lon*DEG2RAD*R,Math.log(Math.tan((90+lat)*DEG2RAD/2))*R]}}
function mercToWgs84(x,y){{return[x/R/DEG2RAD,(2*Math.atan(Math.exp(y/R))-Math.PI/2)/DEG2RAD]}}
function wgs84ToGcj02(lon,lat){{
  if(lon<72.004||lon>137.8347||lat<0.8293||lat>55.8271)return[lon,lat];
  const PI=Math.PI, A=6378245, EE=0.00669342162296594323;
  function tl(x,y){{let r=300+x+2*y+0.1*x*x+0.1*x*y+0.1*Math.sqrt(Math.abs(x));r+=(20*Math.sin(6*x*PI)+20*Math.sin(2*x*PI))*2/3;r+=(20*Math.sin(x*PI)+40*Math.sin(x/3*PI))*2/3;r+=(150*Math.sin(x/12*PI)+300*Math.sin(x/30*PI))*2/3;return r}}
  function tb(x,y){{let r=-100+2*x+3*y+0.2*y*y+0.1*x*y+0.2*Math.sqrt(Math.abs(x));r+=(20*Math.sin(6*x*PI)+20*Math.sin(2*x*PI))*2/3;r+=(20*Math.sin(y*PI)+40*Math.sin(y/3*PI))*2/3;r+=(160*Math.sin(y/12*PI)+320*Math.sin(y*PI/30))*2/3;return r}}
  const dl=tl(lon-105,lat-35), db=tb(lon-105,lat-35);
  const r=lat*PI/180, m=1-EE*Math.sin(r)*Math.sin(r), s=Math.sqrt(m);
  return[lon+dl*180/(A/s*Math.cos(r)*PI),lat+db*180/((A*(1-EE))/(m*s)*PI)]
}}
document.getElementById('wasmBtn').onclick=function(){{
  const from=+document.getElementById('wasmFrom').value, to=+document.getElementById('wasmTo').value;
  const x=+document.getElementById('wasmX').value, y=+document.getElementById('wasmY').value;
  const el=document.getElementById('wasmResult');
  try{{
    let r;
    if(from===4326&&to===3857)r=wgs84ToMerc(x,y);
    else if(from===3857&&to===4326)r=mercToWgs84(x,y);
    else if(from===4326&&to===9000)r=wgs84ToGcj02(x,y);
    else if(from===to)r=[x,y];
    else{{el.textContent='组合不支持，请使用 WASM demo 页面';return}}
    el.textContent='('+r[0].toFixed(4)+', '+r[1].toFixed(4)+')';
  }}catch(e){{el.textContent='Error: '+e}}
}};
</script>
</body>
</html>"""
    
    out_path = OUTPUT_DIR / f"{Path(title).stem}.html"
    with open(out_path, "w", encoding="utf-8") as f:
        f.write(html)
    print(f"  OK HTML: {out_path}")
    return out_path

if __name__ == "__main__":
    # 生成地震报告的 HTML 版
    generate_html(
        report_type="seismic",
        title="2026年中国地震活动评估报告",
        maps=[
            OUTPUT_DIR / "china_seismic_2026.png",
            OUTPUT_DIR / "china_seismic_2026_regions.png",
            OUTPUT_DIR / "china_seismic_2026_stats.png",
        ],
        geojson=OUTPUT_DIR / "china_seismic_high_risk_2026.geojson",
        stats={"极高风险":{"cells":1054,"area_10k_km2":71.6,"pct":7.4},
               "高风险":{"cells":3933,"area_10k_km2":257.2,"pct":26.4},
               "中风险":{"cells":2443,"area_10k_km2":152.9,"pct":15.7},
               "低风险":{"cells":451,"area_10k_km2":28.5,"pct":2.9},
               "极低风险":{"cells":7347,"area_10k_km2":462.6,"pct":47.5}},
    )
    
    # 洪水报告 HTML 版
    generate_html(
        report_type="flood",
        title="中国2026年洪水高风险区评估报告",
        maps=[
            OUTPUT_DIR / "china_flood_risk_2026.png",
            OUTPUT_DIR / "china_flood_risk_2026_regions.png",
            OUTPUT_DIR / "china_flood_risk_2026_stats.png",
        ],
        geojson=OUTPUT_DIR / "china_flood_high_risk_zones_2026.geojson",
        stats={"极高风险":{"cells":1225,"area_10k_km2":85.0,"pct":8.7},
               "高风险":{"cells":2114,"area_10k_km2":133.0,"pct":13.7},
               "中风险":{"cells":641,"area_10k_km2":40.1,"pct":4.1},
               "低风险":{"cells":524,"area_10k_km2":36.6,"pct":3.8},
               "极低风险":{"cells":10724,"area_10k_km2":678.1,"pct":69.7}},
    )
