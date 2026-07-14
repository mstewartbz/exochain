import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'path';

const previewAllowedHosts = [
  'localhost',
  '127.0.0.1',
  'healthcheck.railway.app',
  '.railway.app',
];

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
    allowedHosts: previewAllowedHosts,
  },
});
