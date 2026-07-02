/// IndexedDB storage for field survey records.
import { openDB, type IDBPDatabase } from 'idb';

export interface SurveyRecord {
  id?: number;
  plotName: string;
  landClass: string;
  date: string;
  notes: string;
  areaHa: number;
  carbonTco2e: number;
  geometryGeoJSON: string | null;
  photos: string[]; // base64 data URLs
  createdAt: string;
  synced: boolean;
}

const DB_NAME = 'geotoolbox-field';
const STORE_NAME = 'surveys';

let dbPromise: Promise<IDBPDatabase> | null = null;

function getDb(): Promise<IDBPDatabase> {
  if (!dbPromise) {
    dbPromise = openDB(DB_NAME, 1, {
      upgrade(db) {
        if (!db.objectStoreNames.contains(STORE_NAME)) {
          const store = db.createObjectStore(STORE_NAME, {
            keyPath: 'id',
            autoIncrement: true,
          });
          store.createIndex('synced', 'synced');
          store.createIndex('date', 'date');
        }
      },
    });
  }
  return dbPromise;
}

export async function saveRecord(record: SurveyRecord): Promise<number> {
  const db = await getDb();
  return db.add(STORE_NAME, { ...record, synced: false });
}

export async function getAllRecords(): Promise<SurveyRecord[]> {
  const db = await getDb();
  return db.getAll(STORE_NAME);
}

export async function getUnsyncedRecords(): Promise<SurveyRecord[]> {
  const db = await getDb();
  return db.getAllFromIndex(STORE_NAME, 'synced', false);
}

export async function markSynced(id: number): Promise<void> {
  const db = await getDb();
  const tx = db.transaction(STORE_NAME, 'readwrite');
  const record = await tx.store.get(id);
  if (record) {
    record.synced = true;
    await tx.store.put(record);
  }
  await tx.done;
}

export async function clearAll(): Promise<void> {
  const db = await getDb();
  await db.clear(STORE_NAME);
}

export async function getRecordCount(): Promise<number> {
  const db = await getDb();
  return db.count(STORE_NAME);
}
