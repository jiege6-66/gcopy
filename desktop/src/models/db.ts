import Dexie, { Table } from 'dexie';

export interface HistoryItem {
  id?: number;
  type: 'text' | 'screenshot' | 'file';
  data: string | ArrayBuffer;
  dataType?: string;
  fileName?: string;
  createdAt: number;
  pinned: boolean;
}

class GCopyDB extends Dexie {
  history!: Table<HistoryItem>;

  constructor() {
    super('gcopy-desktop');
    this.version(1).stores({
      history: '++id, type, createdAt, pinned',
    });
  }
}

export const db = new GCopyDB();

export async function addHistoryItem(item: Omit<HistoryItem, 'id'>): Promise<number> {
  // Limit to 50 unpinned items
  const unpinnedCount = await db.history.where('pinned').equals(0).count();
  if (unpinnedCount >= 50) {
    const oldest = await db.history
      .where('pinned')
      .equals(0)
      .sortBy('createdAt');
    if (oldest.length > 0 && oldest[0].id) {
      await db.history.delete(oldest[0].id);
    }
  }

  return db.history.add(item);
}

export async function deleteHistoryItem(id: number): Promise<void> {
  await db.history.delete(id);
}

export async function togglePin(id: number): Promise<void> {
  const item = await db.history.get(id);
  if (item) {
    await db.history.update(id, { pinned: !item.pinned });
  }
}

export async function getHistory(): Promise<HistoryItem[]> {
  return db.history.orderBy('createdAt').reverse().toArray();
}
