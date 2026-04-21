import type { ThemeConfig } from 'antd';
import { theme as antdTheme } from 'antd';

const sharedToken = {
  colorPrimary: '#2563eb',
  colorSuccess: '#059669',
  colorError: '#dc2626',
  colorWarning: '#d97706',
  colorInfo: '#0891b2',
  colorLink: '#2563eb',
  borderRadius: 6,
  borderRadiusLG: 8,
  borderRadiusSM: 4,
  fontFamily:
    "'Fira Sans', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif",
  fontFamilyCode: "'Fira Code', ui-monospace, 'SF Mono', Menlo, Consolas, monospace",
  fontSize: 14,
  controlHeight: 36,
  controlHeightSM: 30,
  controlHeightLG: 44,
};

export const lightTheme: ThemeConfig = {
  algorithm: antdTheme.defaultAlgorithm,
  token: {
    ...sharedToken,
    colorBgBase: '#ffffff',
    colorBgLayout: '#f8fafc',
    colorBgContainer: '#ffffff',
    colorTextBase: '#0f172a',
    colorBorder: '#e2e8f0',
    colorBorderSecondary: '#f1f5f9',
  },
  components: {
    Layout: {
      siderBg: '#0f172a',
      headerBg: '#ffffff',
      bodyBg: '#f8fafc',
      headerPadding: '0 24px',
      headerHeight: 56,
    },
    Menu: {
      darkItemBg: '#0f172a',
      darkSubMenuItemBg: '#020617',
      darkItemSelectedBg: '#2563eb',
      darkItemHoverBg: '#1e293b',
    },
    Table: {
      headerBg: '#f1f5f9',
      rowHoverBg: '#f8fafc',
    },
    Button: {
      fontWeight: 500,
    },
    Card: {
      headerBg: 'transparent',
    },
  },
};

export const darkTheme: ThemeConfig = {
  algorithm: antdTheme.darkAlgorithm,
  token: {
    ...sharedToken,
    colorBgBase: '#0b1220',
    colorBgLayout: '#0b1220',
    colorBgContainer: '#111827',
    colorTextBase: '#f1f5f9',
    colorBorder: '#1f2937',
    colorBorderSecondary: '#111827',
  },
  components: {
    Layout: {
      siderBg: '#020617',
      headerBg: '#111827',
      bodyBg: '#0b1220',
      headerPadding: '0 24px',
      headerHeight: 56,
    },
    Menu: {
      darkItemBg: '#020617',
      darkSubMenuItemBg: '#0b1220',
      darkItemSelectedBg: '#2563eb',
      darkItemHoverBg: '#1f2937',
    },
    Table: {
      headerBg: '#111827',
      rowHoverBg: '#1f2937',
    },
    Button: {
      fontWeight: 500,
    },
  },
};
