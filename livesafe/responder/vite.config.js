import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  server: {
    port: 3002,
    strictPort: false,
    host: true,
    allowedHosts: [
      'responder.livesafe.ai',
      'livesafe.ai',
      'localhost',
    ],
    proxy: {
      '/api': {
        target: 'http://localhost:3001',
        changeOrigin: true,
      },
    },
  },
});
