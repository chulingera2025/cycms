import { Dropdown, type MenuProps } from 'antd';
import { Monitor, Moon, Sun } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useTheme, type ThemeMode } from '@/lib/theme-provider';

export function ThemeSwitcher() {
  const { mode, setMode } = useTheme();
  const { t } = useTranslation('common');

  const items: MenuProps['items'] = [
    { key: 'light', label: t('theme.light'), icon: <Sun size={14} /> },
    { key: 'dark', label: t('theme.dark'), icon: <Moon size={14} /> },
    { key: 'system', label: t('theme.system'), icon: <Monitor size={14} /> },
  ];

  const Icon = mode === 'light' ? Sun : mode === 'dark' ? Moon : Monitor;

  return (
    <Dropdown
      menu={{
        items,
        selectedKeys: [mode],
        onClick: ({ key }) => setMode(key as ThemeMode),
      }}
      placement="bottomRight"
      trigger={['click']}
    >
      <button
        type="button"
        aria-label={t('theme.toggle')}
        className="inline-flex h-8 w-8 items-center justify-center rounded text-text-secondary transition-colors hover:bg-surface-alt hover:text-text"
      >
        <Icon size={16} />
      </button>
    </Dropdown>
  );
}
