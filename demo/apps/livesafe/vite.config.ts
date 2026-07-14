import { defineConfig } from 'vite';
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
    port: 5175,
    proxy: {
      '/api': {
        target: 'http://localhost:3011',
        changeOrigin: true,
      },
    },
  },
  preview: {
    // Production serving on Railway (`npm run start`). Host check disabled: preview only
    // serves static dist/, and Railway's healthchecker uses an internal Host header
    // (healthcheck.railway.app) that an allowlist would 403, failing every deploy.
    allowedHosts: true,
  },
});
