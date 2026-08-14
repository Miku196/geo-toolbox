#!/bin/bash
# 三北NDVI: 下载全部 tile + gdalinfo 统计
cd "$(dirname "$0")/.."
OUTDIR="output/ndvi_full"
mkdir -p "$OUTDIR"
export PATH="$PATH:/d/Program Files/QGIS 3.40.15/bin"

STAC="https://planetarycomputer.microsoft.com/api/stac/v1"
SASAPI="https://planetarycomputer.microsoft.com/api/sas/v1/token/modiseuwest/modis-061-cogs"

get_sas() { curl -s --max-time 10 "$SASAPI" | python -c "import sys,json; print(json.load(sys.stdin)['token'])"; }

echo "=== 搜索 2015年8月 & 2025年6月 ==="
for YR in 2015 2025; do
  DT="2015-08-01/2015-08-31"
  [ "$YR" = "2025" ] && DT="2025-06-01/2025-06-30"
  
  for BBOX in "73,35,95,45" "95,35,115,45" "115,35,135,50"; do
    curl -s -X POST "$STAC/search" -H "Content-Type: application/json" \
      -d "{\"collections\":[\"modis-13Q1-061\"],\"bbox\":[$BBOX],\"datetime\":\"$DT\",\"limit\":500}" | \
      python -c "
import sys, json
d = json.load(sys.stdin)
for f in d.get('features', []):
    hv = [p for p in f['id'].split('.') if 'h' in p and 'v' in p]
    hv = hv[0] if hv else '?'
    url = f['assets'].get('250m_16_days_NDVI', {}).get('href', '')
    if url: print(f'{hv}|{url}')
" 2>/dev/null
  done | sort -u -t'|' -k1,1 > "$OUTDIR/urls_${YR}.txt"
  
  echo "  $YR: $(wc -l < "$OUTDIR/urls_${YR}.txt") tiles"
done

echo ""
echo "=== 下载 + 统计 ==="
SAS=$(get_sas)
echo "SAS: ${SAS:0:20}..."

for YR in 2015 2025; do
  echo "--- $YR ---"
  DIR="$OUTDIR/$YR"; mkdir -p "$DIR"
  > "$DIR/stats.txt"
  TOTAL=0 OK=0
  
  while IFS='|' read -r hv url; do
    TOTAL=$((TOTAL+1))
    TIF="$DIR/${hv}.tif"
    
    # 下载(5% 分辨率 ~120KB, 快)
    if [ ! -f "$TIF" ] || [ "$(wc -c < "$TIF")" -lt 500 ]; then
      gdal_translate -outsize 5% 5% -of GTiff -q "${url}?${SAS}" "$TIF" 2>/dev/null
    fi
    
    if [ -f "$TIF" ] && [ "$(wc -c < "$TIF")" -gt 500 ]; then
      MEAN=$(gdalinfo -stats "$TIF" 2>/dev/null | grep -oP 'STATISTICS_MEAN=\K[0-9.]+')
      if [ -n "$MEAN" ]; then
        NDVI=$(python -c "print(f'{float($MEAN)*0.0001:.4f}')")
        echo "$hv|$NDVI" >> "$DIR/stats.txt"
        OK=$((OK+1))
        echo "  $hv: $NDVI"
      fi
    fi
  done < "$OUTDIR/urls_${YR}.txt"
  
  # 均值
  python -c "
with open('$DIR/stats.txt') as f:
    vals = [float(l.split('|')[1]) for l in f if '|' in l]
if vals:
    print(f'  [$YR] mean={sum(vals)/len(vals):.4f} ({len(vals)}/{TOTAL} tiles)')
" 2>/dev/null
done

echo ""
echo "=== NDVI 变化 ==="
python -c "
with open('$OUTDIR/2015/stats.txt') as f:
    v15 = [float(l.split('|')[1]) for l in f if '|' in l]
with open('$OUTDIR/2025/stats.txt') as f:
    v25 = [float(l.split('|')[1]) for l in f if '|' in l]
if v15 and v25:
    m15 = sum(v15)/len(v15)
    m25 = sum(v25)/len(v25)
    print(f'  2015-08: {m15:.4f} ({len(v15)} tiles)')
    print(f'  2025-06: {m25:.4f} ({len(v25)} tiles)')
    print(f'  变化: {m25-m15:+.4f} ({(m25/m15-1)*100:+.2f}%)')
else:
    print('  数据不足')
"
echo "输出: $OUTDIR"
