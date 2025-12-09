import { useState, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import SyncClipboard from './components/SyncClipboard';
import Settings from './components/Settings';
import Navbar from './components/Navbar';

type View = 'main' | 'settings';

function App() {
  const [view, setView] = useState<View>('main');
  const [syncStatus, setSyncStatus] = useState({
    autoSyncEnabled: true,
    isSyncing: false,
    lastServerIndex: 0,
  });

  useEffect(() => {
    // Load initial sync status
    invoke<typeof syncStatus>('get_sync_status').then(setSyncStatus);

    // Listen for settings open event from tray
    const unlistenSettings = listen('open-settings', () => {
      setView('settings');
    });

    // Listen for sync events
    const unlistenSync = listen<{ type: string }>('sync-event', (event) => {
      if (event.payload.type === 'Started') {
        setSyncStatus((prev) => ({ ...prev, isSyncing: true }));
      } else if (event.payload.type === 'Completed') {
        setSyncStatus((prev) => ({ ...prev, isSyncing: false }));
        invoke<typeof syncStatus>('get_sync_status').then(setSyncStatus);
      }
    });

    return () => {
      unlistenSettings.then((fn) => fn());
      unlistenSync.then((fn) => fn());
    };
  }, []);

  return (
    <div className="min-h-screen bg-base-200" data-theme="light">
      <Navbar
        onSettingsClick={() => setView('settings')}
        onBackClick={() => setView('main')}
        showBack={view === 'settings'}
        syncStatus={syncStatus}
      />

      <main className="container mx-auto p-4 max-w-md">
        {view === 'main' ? (
          <SyncClipboard syncStatus={syncStatus} />
        ) : (
          <Settings onBack={() => setView('main')} />
        )}
      </main>
    </div>
  );
}

export default App;
