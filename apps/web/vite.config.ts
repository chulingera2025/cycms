import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';
import path from 'path';

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  server: {
    port: 3000,
    strictPort: true,
    proxy: {
      '/api': 'http://localhost:8080',
      '/uploads': 'http://localhost:8080',
    },
  },
  build: {
    outDir: 'dist',
    manifest: true,
    sourcemap: false,
    rollupOptions: {
      preserveEntrySignatures: 'exports-only',
      input: {
        app: path.resolve(__dirname, 'index.html'),
        'admin-content-workspace': path.resolve(
          __dirname,
          'src/islands/content-workspace.tsx',
        ),
        'admin-media-workspace': path.resolve(
          __dirname,
          'src/islands/media-workspace.tsx',
        ),
      },
      output: {
        entryFileNames: 'assets/[name].js',
      },
    },
  },
  test: {
    environment: 'jsdom',
    setupFiles: ['./src/test/setup.ts'],
    environmentOptions: {
      jsdom: {
        url: 'http://localhost:3000/admin/plugins',
      },
    },
  },
});
