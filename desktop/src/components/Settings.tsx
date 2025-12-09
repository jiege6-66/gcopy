import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { enable, disable, isEnabled } from '@tauri-apps/plugin-autostart';
import { useTranslation } from 'react-i18next';

interface AppConfig {
  serverUrl: string;
  autoSync: boolean;
  syncInterval: number;
  autoStart: boolean;
  syncTypes: {
    text: boolean;
    screenshot: boolean;
    file: boolean;
  };
  shortcuts: {
    manualSync: string;
    toggleWindow: string;
  };
}

interface SettingsProps {
  onBack: () => void;
}

export default function Settings({ onBack: _onBack }: SettingsProps) {
  const { t } = useTranslation();
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [autostartEnabled, setAutostartEnabled] = useState(false);

  useEffect(() => {
    // Load config
    invoke<AppConfig>('get_config').then(setConfig);

    // Check autostart status
    isEnabled().then(setAutostartEnabled);
  }, []);

  const handleSave = async () => {
    if (!config) return;

    setSaving(true);
    try {
      await invoke('save_config', { config });

      // Handle autostart
      if (config.autoStart && !autostartEnabled) {
        await enable();
        setAutostartEnabled(true);
      } else if (!config.autoStart && autostartEnabled) {
        await disable();
        setAutostartEnabled(false);
      }

      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch (e) {
      console.error('Failed to save config:', e);
    } finally {
      setSaving(false);
    }
  };

  if (!config) {
    return (
      <div className="flex justify-center py-8">
        <span className="loading loading-spinner loading-md"></span>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {/* Server URL */}
      <div className="form-control">
        <label className="label">
          <span className="label-text">{t('serverUrl')}</span>
        </label>
        <input
          type="url"
          className="input input-bordered w-full"
          value={config.serverUrl}
          onChange={(e) =>
            setConfig({ ...config, serverUrl: e.target.value })
          }
        />
      </div>

      {/* Sync Interval */}
      <div className="form-control">
        <label className="label">
          <span className="label-text">{t('syncInterval')}</span>
        </label>
        <input
          type="number"
          min="1"
          max="60"
          className="input input-bordered w-full"
          value={config.syncInterval}
          onChange={(e) =>
            setConfig({ ...config, syncInterval: parseInt(e.target.value) || 3 })
          }
        />
      </div>

      {/* Auto Sync */}
      <div className="form-control">
        <label className="label cursor-pointer">
          <span className="label-text">{t('autoSync')}</span>
          <input
            type="checkbox"
            className="toggle toggle-primary"
            checked={config.autoSync}
            onChange={(e) =>
              setConfig({ ...config, autoSync: e.target.checked })
            }
          />
        </label>
      </div>

      {/* Auto Start */}
      <div className="form-control">
        <label className="label cursor-pointer">
          <span className="label-text">{t('autoStart')}</span>
          <input
            type="checkbox"
            className="toggle toggle-primary"
            checked={config.autoStart}
            onChange={(e) =>
              setConfig({ ...config, autoStart: e.target.checked })
            }
          />
        </label>
      </div>

      {/* Sync Types */}
      <div className="form-control">
        <label className="label">
          <span className="label-text">{t('syncTypes')}</span>
        </label>
        <div className="space-y-2 pl-2">
          <label className="label cursor-pointer justify-start gap-3">
            <input
              type="checkbox"
              className="checkbox checkbox-sm"
              checked={config.syncTypes.text}
              onChange={(e) =>
                setConfig({
                  ...config,
                  syncTypes: { ...config.syncTypes, text: e.target.checked },
                })
              }
            />
            <span className="label-text">{t('text')}</span>
          </label>
          <label className="label cursor-pointer justify-start gap-3">
            <input
              type="checkbox"
              className="checkbox checkbox-sm"
              checked={config.syncTypes.screenshot}
              onChange={(e) =>
                setConfig({
                  ...config,
                  syncTypes: { ...config.syncTypes, screenshot: e.target.checked },
                })
              }
            />
            <span className="label-text">{t('screenshot')}</span>
          </label>
          <label className="label cursor-pointer justify-start gap-3">
            <input
              type="checkbox"
              className="checkbox checkbox-sm"
              checked={config.syncTypes.file}
              onChange={(e) =>
                setConfig({
                  ...config,
                  syncTypes: { ...config.syncTypes, file: e.target.checked },
                })
              }
            />
            <span className="label-text">{t('file')}</span>
          </label>
        </div>
      </div>

      {/* Save Button */}
      <button
        className={`btn btn-primary w-full ${saving ? 'loading' : ''}`}
        onClick={handleSave}
        disabled={saving}
      >
        {saving ? (
          <span className="loading loading-spinner loading-sm"></span>
        ) : null}
        {t('save')}
      </button>

      {saved && (
        <div className="alert alert-success">
          <span>{t('saved')}</span>
        </div>
      )}
    </div>
  );
}
