import { useTranslation } from 'react-i18next';
import { Cog6ToothIcon, ArrowLeftIcon } from '@heroicons/react/24/outline';
import Logo from './Logo';

interface NavbarProps {
  onSettingsClick: () => void;
  onBackClick: () => void;
  showBack: boolean;
  syncStatus: {
    autoSyncEnabled: boolean;
    isSyncing: boolean;
  };
}

export default function Navbar({
  onSettingsClick,
  onBackClick,
  showBack,
  syncStatus,
}: NavbarProps) {
  const { t } = useTranslation();

  return (
    <div className="navbar bg-base-100 shadow-sm">
      <div className="flex-1">
        {showBack ? (
          <button className="btn btn-ghost btn-sm" onClick={onBackClick}>
            <ArrowLeftIcon className="w-5 h-5" />
            {t('back')}
          </button>
        ) : (
          <div className="flex items-center gap-2">
            <Logo />
            <span className="font-semibold text-lg">GCopy</span>
            {syncStatus.isSyncing && (
              <span className="loading loading-spinner loading-xs"></span>
            )}
          </div>
        )}
      </div>
      <div className="flex-none">
        {syncStatus.autoSyncEnabled && (
          <span className="badge badge-success badge-sm mr-2">
            {t('autoSync')}
          </span>
        )}
        {!showBack && (
          <button
            className="btn btn-ghost btn-circle btn-sm"
            onClick={onSettingsClick}
          >
            <Cog6ToothIcon className="w-5 h-5" />
          </button>
        )}
      </div>
    </div>
  );
}
