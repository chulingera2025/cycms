import type { ReactNode } from 'react';
import { RouterProvider } from 'react-router-dom';
import { QueryClientProvider } from '@tanstack/react-query';
import { ReactQueryDevtools } from '@tanstack/react-query-devtools';
import { App as AntApp, ConfigProvider } from 'antd';
import zhCN from 'antd/locale/zh_CN';
import { ErrorBoundary } from '@/components/shared/ErrorBoundary';
import { router } from '@/routes';
import { queryClient } from '@/lib/query-client';
import { ThemeProvider, useTheme } from '@/lib/theme-provider';
import { darkTheme, lightTheme } from '@/styles/theme';
import { useBindAntdApi } from '@/lib/toast';

function AntdApiBinder({ children }: { children: ReactNode }) {
  useBindAntdApi();
  return <>{children}</>;
}

function ThemedApp() {
  const { resolved } = useTheme();
  return (
    <ConfigProvider theme={resolved === 'dark' ? darkTheme : lightTheme} locale={zhCN}>
      <AntApp>
        <AntdApiBinder>
          <ErrorBoundary>
            <RouterProvider router={router} />
          </ErrorBoundary>
        </AntdApiBinder>
      </AntApp>
    </ConfigProvider>
  );
}

export default function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <ThemeProvider>
        <ThemedApp />
      </ThemeProvider>
      {import.meta.env.DEV && <ReactQueryDevtools initialIsOpen={false} buttonPosition="bottom-left" />}
    </QueryClientProvider>
  );
}
