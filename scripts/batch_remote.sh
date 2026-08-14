#!/bin/bash
# 三北防护林 NDVI 批处理 - 远程读取 COG 元数据（不下载文件）
# 使用 GDAL gdalinfo 直接读取远程 COG statistics
cd "$(dirname "$0")/.."
OUTDIR="output/ndvi_remote"
mkdir -p "$OUTDIR"
export PATH="$PATH:/d/Program Files/QGIS 3.40.15/bin"

STAC="https://planetarycomputer.microsoft.com/api/stac/v1"
SAS_TOKEN_URL="https://planetarycomputer.microsoft.com/api/sas/v1/token/modiseuwest/modis-061-cogs"

echo "==========================================="
echo " 三北防护林 NDVI 批处理（远程读取）"
echo "==========================================="

# 获取 SAS token
SAS=$(curl -s --max-time 10 "$SAS_TOKEN_URL" | python -c "import sys,json; print(json.load(sys.stdin)['token'])")
echo "SAS token OK"

# 分区域搜索
search() {
  local year=$1 start=$2 end=$3 label=$4
  local out="$OUTDIR/urls_${year}.txt"
  > "$out"
  
  for bbox in "73,35,95,45" "95,35,115,45" "115,35,135,50"; do
    curl -s -X POST "$STAC/search" \
      -H "Content-Type: application/json" \
      -d "{\"collections\":[\"modis-13Q1-061\"],\"bbox\":[$bbox],\"datetime\":\"${start}/${end}\",\"limit\":500}" | \
      python -c "
import sys, json
d = json.load(sys.stdin)
for f in d.get('features', []):
    hv = [p for p in f['id'].split('.') if 'h' in p and 'v' in p]
    hv = hv[0] if hv else '?'
    url = f['assets'].get('250m_16_days_NDVI', {}).get('href', '')
    if url: print(f'{hv}|{url}')
" >> "$out"
  done
  
  # 去重
  sort -u -t'|' -k1,1 "$out" > "${out}.tmp" && mv "${out}.tmp" "$out"
  echo "  $label $(wc -l < "$out") unique tiles"
}

echo ""
echo "搜索 2015年8月..."
search 2015 "2015-08-01" "2015-08-31" "2015-08"

echo ""
echo "搜索 2025年6月..."
search 2025 "2025-06-01" "2025-06-30" "2025-06"

# 远程读取 NDVI 统计
process_year() {
  local year=$1 label=$2
  local urlfile="$OUTDIR/urls_${year}.txt"
  local out="$OUTDIR/stats_${year}.txt"
  > "$out"
  
  echo ""
  echo "读取 $label NDVI..."
  
  local total=0 ok=0
  local sum=0
  
  while IFS='|' read -r hv url; do
    [ -z "$url" ] && continue
    total=$((total + 1))
    
    # 远程读取 COG 统计 (只传 metadata，很快)
    mean=$(gdalinfo -stats -norat "${url}?${SAS}" 2>/dev/null | grep -oP 'STATISTICS_MEAN=\K[0-9.]+' | head -1)
    
    if [ -n "$mean" ]; then
      ndvi=$(python -c "print(f'{float($mean)*0.0001:.4f}')")
      echo "$hv|$ndvi|$mean" >> "$out"
      echo "  $hv: $ndvi"
      ok=$((ok + 1))
      sum=$(python -c "print($sum + $mean)")
    else
      echo "  $hv: FAIL"
    fi
  done < "$urlfile"
  
  if [ $ok -gt 0 ]; then
    avg=$(python -c "print(f'{$sum/$ok*0.0001:.4f}')")
    echo "  [$label] $ok/$total tiles, mean NDVI=$avg"
    echo "$avg" > "$OUTDIR/avg_${year}.txt"
  fi
}

process_year 2015 "2015-08"
process_year 2025 "2025-06"

# 变化
echo ""
echo "==========================================="
echo " NDVI 变化结果"
echo "==========================================="
if [ -f "$OUTDIR/avg_2015.txt" ] && [ -f "$OUTDIR/avg_2025.txt" ]; then
  N15=$(cat "$OUTDIR/avg_2015.txt")
  N25=$(cat "$OUTDIR/avg_2025.txt")
  DIFF=$(python -c "print(f'{float($N25)-float($N15):+.4f}')")
  PCT=$(python -c "print(f'{(float($N25)/float($N15)-1)*100:+.2f}')")
  echo "  2015年8月 NDVI: $N15"
  echo "  2025年6月 NDVI: $N25"
  echo "  变化: $DIFF ($PCT%)"
else
  echo "  数据不足"
fi
echo "输出: $OUTDIR"
