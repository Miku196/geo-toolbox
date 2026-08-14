#!/bin/bash
# 三北防护林 NDVI 批量处理
# 工具: curl, gdal_translate, gdalinfo, awk
cd "$(dirname "$0")/.."
OUTDIR="output/ndvi_3north_full"
mkdir -p "$OUTDIR"
export PATH="$PATH:/d/Program Files/QGIS 3.40.15/bin"

STAC="https://planetarycomputer.microsoft.com/api/stac/v1"
BBOX="73,35,135,50"
SAS_URL="https://planetarycomputer.microsoft.com/api/sas/v1/token/modiseuwest/modis-061-cogs"

echo "============================================"
echo " 三北防护林 NDVI 批量处理"
echo " 区域: $BBOX"
echo "============================================"

# 获取 SAS token
SAS=$(curl -s --max-time 10 "$SAS_URL" | awk -F'"token":"' '{print $2}' | awk -F'"' '{print $1}')
echo "SAS token: ${SAS:0:20}..."

search_year() {
  local year=$1 start=$2 end=$3
  local f="$OUTDIR/search_${year}.json"
  
  curl -s -X POST "$STAC/search" \
    -H "Content-Type: application/json" \
    -d "{\"collections\":[\"modis-13Q1-061\"],\"bbox\":[$BBOX],\"datetime\":\"${start}/${end}\",\"limit\":500}" > "$f"
  
  # 提取唯一 tile (去重)
  awk 'BEGIN{RS="features";FS="\\{\"id\""} NR>1{for(i=2;i<=NF;i++){split($i,a,"\"");print a[2]}}' "$f" | \
    grep -oP 'MOD13Q1\.A\d{7}\.[^"]+' | sort -u > "$OUTDIR/ids_${year}.txt"
  
  echo "$(wc -l < "$OUTDIR/ids_${year}.txt") tiles"
}

get_ndvi_url() {
  local id=$1 year=$2
  local hv=$(echo "$id" | grep -oP 'h\d+v\d+')
  grep -oP '"250m_16_days_NDVI":{"href":"[^"]*'"$hv"'[^"]*"' "$OUTDIR/search_${year}.json" | \
    awk -F'"250m_16_days_NDVI":{"href":"' '{print $2}' | awk -F'"' '{print $1}' | head -1
}

echo ""
echo "--- 搜索 2015年8月 ---"
search_year 2015 "2015-08-01" "2015-08-31"

echo ""
echo "--- 搜索 2025年6月 ---"
search_year 2025 "2025-06-01" "2025-06-30"

process_year() {
  local year=$1 label=$2
  local dir="$OUTDIR/$year"
  mkdir -p "$dir"
  
  local total=0 ok=0 fail=0
  local sum=0 cnt=0
  
  echo ""
  echo "--- 下载+处理 $label ---"
  
  while IFS= read -r id; do
    [ -z "$id" ] && continue
    hv=$(echo "$id" | grep -oP 'h\d+v\d+')
    url=$(get_ndvi_url "$id" "$year")
    [ -z "$url" ] && continue
    
    total=$((total + 1))
    tif="$dir/${hv}.tif"
    
    # 下载（如果不存在或太小）
    if [ ! -f "$tif" ] || [ "$(wc -c < "$tif" 2>/dev/null || echo 0)" -lt 1000 ]; then
      "D:/Program Files/QGIS 3.40.15/bin/gdal_translate.exe" \
        -outsize 10% 10% -of GTiff -q \
        "${url}?${SAS}" "$tif" 2>/dev/null
    fi
    
    if [ -f "$tif" ] && [ "$(wc -c < "$tif")" -gt 1000 ]; then
      mean=$("D:/Program Files/QGIS 3.40.15/bin/gdalinfo.exe" -stats "$tif" 2>/dev/null | \
        grep -oP 'STATISTICS_MEAN=\K[0-9.]+' | head -1)
      if [ -n "$mean" ]; then
        ndvi=$(awk "BEGIN{printf \"%.4f\", $mean * 0.0001}")
        echo "  $hv: $ndvi"
        ok=$((ok + 1))
        sum=$(awk "BEGIN{print $sum + $mean}")
        cnt=$((cnt + 1))
      else
        fail=$((fail + 1))
      fi
    fi
  done < "$OUTDIR/ids_${year}.txt"
  
  if [ $cnt -gt 0 ]; then
    avg=$(awk "BEGIN{printf \"%.4f\", $sum / $cnt * 0.0001}")
    echo "  [$label] ok=$ok/$total fail=$fail mean=$avg"
    echo "$avg" > "$OUTDIR/mean_${year}.txt"
  else
    echo "  [$label] ok=$ok/$total fail=$fail"
  fi
}

process_year 2015 "2015-08"
process_year 2025 "2025-06"

echo ""
echo "=== NDVI 变化 ==="
if [ -f "$OUTDIR/mean_2015.txt" ] && [ -f "$OUTDIR/mean_2025.txt" ]; then
  N15=$(cat "$OUTDIR/mean_2015.txt")
  N25=$(cat "$OUTDIR/mean_2025.txt")
  DIFF=$(awk "BEGIN{printf \"%.4f\", $N25 - $N15}")
  PCT=$(awk "BEGIN{printf \"%.2f\", ($N25 / $N15 - 1) * 100}")
  echo "  2015年8月 NDVI: $N15"
  echo "  2025年6月 NDVI: $N25"
  echo "  变化: $DIFF (${PCT}%)"
fi
echo "输出: $OUTDIR"
