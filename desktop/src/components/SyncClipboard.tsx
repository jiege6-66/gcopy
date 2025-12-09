import { useState, useEffect, useCallback } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { useTranslation } from 'react-i18next';
import { useLiveQuery } from 'dexie-react-hooks';
import {
  ArrowPathIcon,
  ClipboardDocumentIcon,
} from '@heroicons/react/24/outline';
import { db, addHistoryItem } from '../models/db';
import { ClipboardContent } from '../lib/clipboard';
import HistoryList from './HistoryList';

interface SyncClipboardProps {
  syncStatus: {
    autoSyncEnabled: boolean;
    isSyncing: boolean;
    lastServerIndex: number;
  };
}

export default function SyncClipboard({ syncStatus }: SyncClipboardProps) {
  const { t } = useTranslation();
  const [status, setStatus] = useState<'idle' | 'syncing' | 'success' | 'error'>('idle');
  const [message, setMessage] = useState('');

  // Live query for history
  const history = useLiveQuery(() =>
    db.history.orderBy('createdAt').reverse().limit(50).toArray()
  );

  // Handle clipboard changes from Rust backend
  useEffect(() => {
    const unlisten = listen<ClipboardContent>('clipboard-changed', async (event) => {
      const content = event.payload;

      // Add to history
      if (content.type === 'Text') {
        await addHistoryItem({
          type: 'text',
          data: content.data as string,
          createdAt: Date.now(),
          pinned: false,
        });
      } else if (content.type === 'Image') {
        const bytes = new Uint8Array(content.data as number[]);
        await addHistoryItem({
          type: 'screenshot',
          data: bytes.buffer,
          dataType: 'image/png',
          createdAt: Date.now(),
          pinned: false,
        });
      }

      // If auto-sync is enabled, push to server
      if (syncStatus.autoSyncEnabled) {
        try {
          await invoke('sync_now');
        } catch (e) {
          console.error('Auto sync failed:', e);
        }
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [syncStatus.autoSyncEnabled]);

  // Handle sync events
  useEffect(() => {
    const unlisten = listen<{ type: string; content_type?: string; message?: string }>(
      'sync-event',
      (event) => {
        const { type, content_type, message: errorMsg } = event.payload;

        switch (type) {
          case 'Started':
            setStatus('syncing');
            setMessage(t('syncing'));
            break;
          case 'Pushed':
            setStatus('success');
            setMessage(`${t('pushed')} (${content_type})`);
            break;
          case 'Pulled':
            setStatus('success');
            setMessage(`${t('pulled')} (${content_type})`);
            break;
          case 'Error':
            setStatus('error');
            setMessage(errorMsg || t('error'));
            break;
          case 'Completed':
            setTimeout(() => {
              setStatus('idle');
              setMessage('');
            }, 2000);
            break;
        }
      }
    );

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [t]);

  const handleSyncNow = useCallback(async () => {
    try {
      await invoke('sync_now');
    } catch (e) {
      setStatus('error');
      setMessage(String(e));
    }
  }, []);

  return (
    <div className="space-y-4">
      {/* Sync Button */}
      <div className="card bg-base-100 shadow-sm">
        <div className="card-body p-4">
          <button
            className={`btn btn-primary btn-lg w-full ${
              syncStatus.isSyncing ? 'loading' : ''
            }`}
            onClick={handleSyncNow}
            disabled={syncStatus.isSyncing}
          >
            {syncStatus.isSyncing ? (
              <span className="loading loading-spinner"></span>
            ) : (
              <ArrowPathIcon className="w-6 h-6" />
            )}
            {syncStatus.isSyncing ? t('syncing') : t('syncNow')}
          </button>

          {/* Status message */}
          {message && (
            <div
              className={`alert mt-2 ${
                status === 'success'
                  ? 'alert-success'
                  : status === 'error'
                  ? 'alert-error'
                  : 'alert-info'
              }`}
            >
              <span className="text-sm">{message}</span>
            </div>
          )}
        </div>
      </div>

      {/* Current Clipboard */}
      <div className="card bg-base-100 shadow-sm">
        <div className="card-body p-4">
          <h3 className="card-title text-sm flex items-center gap-2">
            <ClipboardDocumentIcon className="w-4 h-4" />
            {t('history')}
          </h3>
          <HistoryList items={history || []} />
        </div>
      </div>
    </div>
  );
}
