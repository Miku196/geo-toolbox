#!/bin/bash
# 三北NDVI: 下载全部 tile + 比对报告
cd "$(dirname "$0")/.."
OUT="output/ndvi_final"
mkdir -p "$OUT/2015" "$OUT/2025"
export PATH="$PATH:/d/Program Files/QGIS 3.40.15/bin"
SASAPI="https://planetarycomputer.microsoft.com/api/sas/v1/token/modiseuwest/modis-061-cogs"
LOG="$OUT/run.log"

get_sas() { curl -s --max-time 10 "$SASAPI" | python -c "import sys,json; print(json.load(sys.stdin)['token'])"; }

echo "$(date) START" | tee -a "$LOG"

# 下载单个 tile
dl() {
  local url=$1 out=$2 sas=$3
  if [ -f "$out" ] && [ "$(stat -c%s "$out" 2>/dev/null || wc -c < "$out" 2>/dev/null)" -gt 500 ]; then
    return 0
  fi
  gdal_translate -outsize 5% 5% -of GTiff -q "${url}?${sas}" "$out" 2>/dev/null
  [ -f "$out" ] && [ "$(wc -c < "$out")" -gt 500 ]
}

SAS=$(get_sas); ST=$SECONDS

# 下载2015
echo "=== 2015年8月 ===" | tee -a "$LOG"
while IFS='|' read -r hv url; do
  [ "$hv" = "h27v03" ] && continue  # 不在三北范围
  TIF="$OUT/2015/${hv}.tif"
  echo -n "  $hv... " | tee -a "$LOG"
  if dl "$url" "$TIF" "$SAS"; then
    MEAN=$(gdalinfo -stats "$TIF" 2>/dev/null | grep -oP 'STATISTICS_MEAN=\K[0-9.]+')
    [ -n "$MEAN" ] && echo "$(python -c "print(f'{float($MEAN)*0.0001:.4f}')")" | tee -a "$LOG" || echo "STATFAIL" | tee -a "$LOG"
  else
    echo "FAIL" | tee -a "$LOG"
  fi
  # 刷新SAS每20min
  [ $((SECONDS - ST)) -gt 1200 ] && { SAS=$(get_sas); ST=$SECONDS; echo "  [SAS refreshed]" | tee -a "$LOG"; }
done < output/ndvi_full/urls_2015.txt

# 下载2025  
echo "=== 2025年6月 ===" | tee -a "$LOG"
while IFS='|' read -r hv url; do
  [ "$hv" = "h29v05" ] && continue  # 不在三北
  TIF="$OUT/2025/${hv}.tif"
  echo -n "  $hv... " | tee -a "$LOG"
  if dl "$url" "$TIF" "$SAS"; then
    MEAN=$(gdalinfo -stats "$TIF" 2>/dev/null | grep -oP 'STATISTICS_MEAN=\K[0-9.]+')
    [ -n "$MEAN" ] && echo "$(python -c "print(f'{float($MEAN)*0.0001:.4f}')")" | tee -a "$LOG" || echo "STATFAIL" | tee -a "$LOG"
  else
    echo "FAIL" | tee -a "$LOG"
  fi
  [ $((SECONDS - ST)) -gt 1200 ] && { SAS=$(get_sas); ST=$SECONDS; echo "  [SAS refreshed]" | tee -a "$LOG"; }
done < output/ndvi_full/urls_2025.txt

echo "$(date) DONE" | tee -a "$LOG"

# 生成报告
echo "" | tee -a "$LOG"
echo "======================" | tee -a "$LOG"
echo " NDVI变化分析报告" | tee -a "$LOG"
echo "======================" | tee -a "$LOG"

python -c "
import os, glob, csv

res = {}
for yr in ['2015','2025']:
    stats = []
    for f in sorted(glob.glob('$OUT/'+yr+'/*.tif')):
        hv = os.path.basename(f).replace('.tif','')
        # 用 python 调 gdalinfo
        import subprocess
        r = subprocess.run(['D:/Program Files/QGIS 3.40.15/bin/gdalinfo','-stats',f], capture_output=True, text=True)
        for line in r.stdout.split('\n'):
            if 'STATISTICS_MEAN=' in line:
                mean = float(line.split('=')[1])
                ndvi = round(mean * 0.0001, 4)
                stats.append((hv, ndvi))
                break
    
    res[yr] = stats
    print(f'\n{yr}年 ({len(stats)} tiles):')
    print(f'  {\"Tile\":>8}  NDVI')
    print(f'  {\"-\"*16}')
    for hv, ndvi in stats:
        print(f'  {hv:>8}  {ndvi:.4f}')
    
    vals = [s[1] for s in stats]
    if vals:
        print(f'  {\"平均\":>8}  {sum(vals)/len(vals):.4f}')
        print(f'  {\"范围\":>8}  {min(vals):.4f}~{max(vals):.4f}')

# 交叉对比（同tile）
print()
print('同tile变化对比:')
print(f'  {\"Tile\":>8}  2015年  2025年  变化')
hv15 = {s[0]:s[1] for s in res.get('2015',[])}
hv25 = {s[0]:s[1] for s in res.get('2025',[])}
common = set(hv15.keys()) & set(hv25.keys())
if common:
    changes = []
    for hv in sorted(common):
        d = hv25[hv] - hv15[hv]
        changes.append(d)
        print(f'  {hv:>8}  {hv15[hv]:.4f}  {hv25[hv]:.4f}  {d:+.4f} ({(d/hv15[hv]*100):+.2f}%)')
    if changes:
        print(f'  {\"平均变化\":>8}  {sum(changes)/len(changes):+.4f}')

# CSV保存
with open('$OUT/ndvi_report.csv','w',newline='') as f:
    w = csv.writer(f)
    w.writerow(['tile','year','ndvi'])
    for yr in ['2015','2025']:
        for hv,ndvi in res.get(yr,[]):
            w.writerow([hv,yr,ndvi])
print()
print('报告保存: $OUT/ndvi_report.csv')
" 2>&1 | tee -a "$LOG"