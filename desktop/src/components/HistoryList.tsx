import { useTranslation } from 'react-i18next';
import moment from 'moment';
import {
  DocumentTextIcon,
  PhotoIcon,
  DocumentIcon,
  TrashIcon,
} from '@heroicons/react/24/outline';
import { StarIcon } from '@heroicons/react/24/solid';
import { HistoryItem, deleteHistoryItem, togglePin } from '../models/db';
import { writeClipboard, ClipboardContent } from '../lib/clipboard';

interface HistoryListProps {
  items: HistoryItem[];
}

export default function HistoryList({ items }: HistoryListProps) {
  const { t } = useTranslation();

  if (items.length === 0) {
    return (
      <div className="text-center text-base-content/50 py-8">
        {t('noHistory')}
      </div>
    );
  }

  const handleCopy = async (item: HistoryItem) => {
    let content: ClipboardContent;

    if (item.type === 'text') {
      content = { type: 'Text', data: item.data as string };
    } else if (item.type === 'screenshot') {
      const arrayBuffer = item.data as ArrayBuffer;
      const bytes = Array.from(new Uint8Array(arrayBuffer));
      content = { type: 'Image', data: bytes };
    } else {
      return; // Files not supported yet
    }

    try {
      await writeClipboard(content);
    } catch (e) {
      console.error('Failed to write clipboard:', e);
    }
  };

  const handleDelete = async (id: number) => {
    await deleteHistoryItem(id);
  };

  const handleTogglePin = async (id: number) => {
    await togglePin(id);
  };

  const formatTime = (timestamp: number) => {
    const diff = Date.now() - timestamp;
    const minutes = Math.floor(diff / 60000);

    if (minutes < 1) return t('justNow');
    if (minutes < 60) return t('minutesAgo', { count: minutes });

    const hours = Math.floor(minutes / 60);
    if (hours < 24) return t('hoursAgo', { count: hours });

    return moment(timestamp).format('MM/DD HH:mm');
  };

  const getIcon = (type: string) => {
    switch (type) {
      case 'text':
        return <DocumentTextIcon className="w-4 h-4" />;
      case 'screenshot':
        return <PhotoIcon className="w-4 h-4" />;
      case 'file':
        return <DocumentIcon className="w-4 h-4" />;
      default:
        return <DocumentTextIcon className="w-4 h-4" />;
    }
  };

  const getPreview = (item: HistoryItem) => {
    if (item.type === 'text') {
      const text = item.data as string;
      return text.length > 100 ? text.substring(0, 100) + '...' : text;
    }

    if (item.type === 'screenshot') {
      try {
        const arrayBuffer = item.data as ArrayBuffer;
        const blob = new Blob([arrayBuffer], { type: 'image/png' });
        const url = URL.createObjectURL(blob);
        return (
          <img
            src={url}
            alt="Screenshot"
            className="max-h-20 rounded object-contain"
            onLoad={() => URL.revokeObjectURL(url)}
          />
        );
      } catch {
        return <span className="text-base-content/50">Image</span>;
      }
    }

    if (item.type === 'file') {
      return item.fileName || 'File';
    }

    return '';
  };

  return (
    <div className="space-y-2 max-h-96 overflow-y-auto">
      {items.map((item) => (
        <div
          key={item.id}
          className={`card bg-base-200 cursor-pointer hover:bg-base-300 transition-colors ${
            item.pinned ? 'border-l-4 border-primary' : ''
          }`}
          onClick={() => handleCopy(item)}
        >
          <div className="card-body p-3">
            <div className="flex items-start justify-between gap-2">
              <div className="flex items-start gap-2 flex-1 min-w-0">
                <span className="text-base-content/70 mt-0.5">
                  {getIcon(item.type)}
                </span>
                <div className="flex-1 min-w-0">
                  {item.type === 'text' ? (
                    <p className="text-sm break-words whitespace-pre-wrap">
                      {getPreview(item)}
                    </p>
                  ) : (
                    getPreview(item)
                  )}
                  <p className="text-xs text-base-content/50 mt-1">
                    {formatTime(item.createdAt)}
                  </p>
                </div>
              </div>
              <div className="flex items-center gap-1">
                <button
                  className="btn btn-ghost btn-xs"
                  onClick={(e) => {
                    e.stopPropagation();
                    handleTogglePin(item.id!);
                  }}
                  title={item.pinned ? t('unpin') : t('pin')}
                >
                  <StarIcon
                    className={`w-4 h-4 ${
                      item.pinned ? 'text-warning' : 'text-base-content/30'
                    }`}
                  />
                </button>
                <button
                  className="btn btn-ghost btn-xs"
                  onClick={(e) => {
                    e.stopPropagation();
                    handleDelete(item.id!);
                  }}
                  title={t('delete')}
                >
                  <TrashIcon className="w-4 h-4 text-error" />
                </button>
              </div>
            </div>
          </div>
        </div>
      ))}
    </div>
  );
}
