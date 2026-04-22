import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import '@fontsource/fira-sans/400.css';
import '@fontsource/fira-sans/500.css';
import '@fontsource/fira-sans/600.css';
import '@fontsource/fira-sans/700.css';
import '@fontsource/fira-code/400.css';
import '@fontsource/fira-code/500.css';
import '@/lib/i18n';
import { bootstrapHostIslands, hasHostIslandBoot } from '@/features/islands/runtime';
import './index.css';

const rootElement = document.getElementById('root');

async function start() {
  if (rootElement) {
    const { default: App } = await import('./App');
    createRoot(rootElement).render(
      <StrictMode>
        <App />
      </StrictMode>,
    );
    return;
  }

  if (hasHostIslandBoot(document)) {
    await bootstrapHostIslands(document);
  }
}

void start().catch((error) => {
  console.error('failed to start web runtime', error);
});
