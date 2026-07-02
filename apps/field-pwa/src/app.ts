/// Field PWA — Main application logic.
import { initMap } from './map.js';
import { saveRecord, getAllRecords, clearAll, getUnsyncedRecords, markSynced, type SurveyRecord } from './db.js';
import { estimateCarbonStock, annualSequestrationRate } from './carbon.js';

// ── Init ──

const mapInstance = initMap('map');

// Set today's date
const dateInput = document.getElementById('survey-date') as HTMLInputElement;
dateInput.value = new Date().toISOString().slice(0, 10);

// ── Tab switching ──

document.querySelectorAll('.tab').forEach(tab => {
  tab.addEventListener('click', () => {
    document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
    tab.classList.add('active');
    const tabName = (tab as HTMLElement).dataset.tab;
    ['survey', 'history', 'sync'].forEach(name => {
      const el = document.getElementById(`tab-${name}`);
      if (el) el.style.display = name === tabName ? 'block' : 'none';
    });
    if (tabName === 'history') refreshHistory();
    if (tabName === 'sync') refreshSyncStatus();
  });
});

// ── Draw AOI ──

document.getElementById('btn-draw-aoi')!.addEventListener('click', () => {
  mapInstance.drawPolygon();
});

// ── Calculate Carbon ──

document.getElementById('btn-calculate')!.addEventListener('click', () => {
  const landClass = (document.getElementById('land-class') as HTMLSelectElement).value;
  const areaHa = mapInstance.getDrawnAreaHa();

  if (areaHa < 0.001) {
    alert('请先在左侧地图上圈定调查范围（点击添加顶点，双击完成）');
    return;
  }

  const result = estimateCarbonStock(landClass, areaHa);
  const seqRate = annualSequestrationRate(landClass);

  const resultCard = document.getElementById('carbon-result')!;
  resultCard.style.display = 'block';
  document.getElementById('carbon-value')!.textContent = result.totalTco2e.toFixed(1);
  document.getElementById('aoi-area')!.textContent = result.areaHa.toFixed(2);

  // Store in memory for save
  (window as any).__lastCarbonResult = result;
  (window as any).__lastGeoJSON = mapInstance.getDrawnGeoJSON();
});

// ── Save Record ──

document.getElementById('btn-save')!.addEventListener('click', async () => {
  const plotName = (document.getElementById('plot-name') as HTMLInputElement).value || '未命名样地';
  const landClass = (document.getElementById('land-class') as HTMLSelectElement).value;
  const date = dateInput.value;
  const notes = (document.getElementById('notes') as HTMLInputElement).value;
  const result = (window as any).__lastCarbonResult;
  const geoJSON = (window as any).__lastGeoJSON;

  if (!result) {
    alert('请先计算碳储量');
    return;
  }

  const record: SurveyRecord = {
    plotName,
    landClass,
    date,
    notes,
    areaHa: result.areaHa,
    carbonTco2e: result.totalTco2e,
    geometryGeoJSON: geoJSON ? JSON.stringify(geoJSON) : null,
    photos: (window as any).__photos ?? [],
    createdAt: new Date().toISOString(),
    synced: false,
  };

  try {
    const id = await saveRecord(record);
    alert(`✅ 记录已保存 (ID: ${id})`);

    // Reset form
    (document.getElementById('plot-name') as HTMLInputElement).value = '';
    (document.getElementById('notes') as HTMLInputElement).value = '';
    (window as any).__lastCarbonResult = null;
    (window as any).__lastGeoJSON = null;
    (window as any).__photos = [];
    document.getElementById('carbon-result')!.style.display = 'none';
    mapInstance.clearPolygon();
    updatePhotoList([]);
  } catch (e) {
    alert('❌ 保存失败: ' + (e as Error).message);
  }
});

// ── Photo handling ──

(window as any).__photos = [] as string[];

document.getElementById('photo-input')!.addEventListener('change', async (e) => {
  const files = (e.target as HTMLInputElement).files;
  if (!files) return;

  const photos: string[] = (window as any).__photos ?? [];

  for (const file of Array.from(files)) {
    // Resize and convert to base64
    const dataUrl = await resizeImage(file, 800);
    photos.push(dataUrl);
  }
  (window as any).__photos = photos;
  updatePhotoList(photos);
  (e.target as HTMLInputElement).value = '';
});

function updatePhotoList(photos: string[]) {
  const list = document.getElementById('photo-list')!;
  list.innerHTML = photos.map(p => `<img src="${p}" class="photo-preview" />`).join('');
  document.getElementById('photo-count')!.textContent = String(photos.length);
}

async function resizeImage(file: File, maxSize: number): Promise<string> {
  return new Promise((resolve) => {
    const reader = new FileReader();
    reader.onload = () => {
      const img = new Image();
      img.onload = () => {
        const canvas = document.createElement('canvas');
        const ratio = Math.min(maxSize / img.width, maxSize / img.height, 1);
        canvas.width = img.width * ratio;
        canvas.height = img.height * ratio;
        canvas.getContext('2d')!.drawImage(img, 0, 0, canvas.width, canvas.height);
        resolve(canvas.toDataURL('image/jpeg', 0.7));
      };
      img.src = reader.result as string;
    };
    reader.readAsDataURL(file);
  });
}

// ── History tab ──

async function refreshHistory() {
  const records = await getAllRecords();
  const list = document.getElementById('history-list')!;
  if (records.length === 0) {
    list.innerHTML = '<p style="color:#999;">暂无记录</p>';
    return;
  }
  list.innerHTML = records
    .slice()
    .reverse()
    .map(r => `
      <div style="background:#fafafa;border-radius:8px;padding:10px;margin-bottom:8px;border-left:3px solid #2e7d32;">
        <div style="font-weight:600;display:flex;justify-content:space-between;">
          <span>${r.plotName}</span>
          <span style="color:#2e7d32;">${r.carbonTco2e.toFixed(1)} tCO₂e</span>
        </div>
        <div style="font-size:11px;color:#999;margin-top:2px;">
          ${r.date} | ${r.landClass} | ${r.areaHa} ha
          ${r.synced ? ' ✅已同步' : ' 📱本地'}
        </div>
        ${r.notes ? `<div style="font-size:11px;color:#666;margin-top:2px;">${r.notes}</div>` : ''}
      </div>
    `)
    .join('');
}

// ── Export ──

document.getElementById('btn-export-json')!.addEventListener('click', async () => {
  const records = await getAllRecords();
  const json = JSON.stringify(records, null, 2);
  downloadFile(`field-survey-${new Date().toISOString().slice(0, 10)}.json`, json, 'application/json');
});

document.getElementById('btn-export-xlsx')!.addEventListener('click', async () => {
  const records = await getAllRecords();
  let csv = '样地名称,土地类型,日期,面积(ha),碳储量(tCO₂e),备注,已同步\n';
  for (const r of records) {
    csv += `${r.plotName},${r.landClass},${r.date},${r.areaHa},${r.carbonTco2e},"${r.notes}",${r.synced}\n`;
  }
  downloadFile(`field-survey-${new Date().toISOString().slice(0, 10)}.csv`, csv, 'text/csv');
});

document.getElementById('btn-clear')!.addEventListener('click', async () => {
  if (confirm('确定要清除所有本地记录吗？此操作不可撤销。')) {
    await clearAll();
    await refreshHistory();
    alert('已清除');
  }
});

// ── Sync ──

async function refreshSyncStatus() {
  const unsynced = await getUnsyncedRecords();
  const status = document.getElementById('sync-status')!;
  status.innerHTML = unsynced.length === 0
    ? '✅ 所有记录已同步'
    : `📱 ${unsynced.length} 条记录等待同步`;
}

document.getElementById('btn-sync')!.addEventListener('click', async () => {
  const unsynced = await getUnsyncedRecords();
  if (unsynced.length === 0) {
    alert('没有需要同步的记录');
    return;
  }
  // Simulated sync — in production, POST to server
  for (const r of unsynced) {
    console.log('Syncing record:', r.id, r.plotName);
    if (r.id) await markSynced(r.id);
  }
  await refreshSyncStatus();
  alert(`✅ 已同步 ${unsynced.length} 条记录`);
});

// ── Online/Offline status ──

function updateOnlineStatus() {
  const el = document.getElementById('status')!;
  if (navigator.onLine) {
    el.textContent = '🟢 在线';
    el.style.background = 'rgba(0,128,0,.7)';
  } else {
    el.textContent = '🔴 离线';
    el.style.background = 'rgba(200,0,0,.7)';
  }
}
window.addEventListener('online', updateOnlineStatus);
window.addEventListener('offline', updateOnlineStatus);
updateOnlineStatus();

// ── Helpers ──

function downloadFile(filename: string, content: string, mime: string) {
  const blob = new Blob([content], { type: mime });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}
