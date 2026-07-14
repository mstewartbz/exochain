import type { Config } from 'tailwindcss';

export default {
  content: ['./index.html', './src/**/*.{ts,tsx}'],
  theme: {
    extend: {
      colors: {
        xc: {
          navy: '#0c1222',
          slate: '#1c2640',
          indigo: { 400: '#818cf8', 500: '#6366f1', 600: '#4f46e5' },
          purple: { 400: '#a78bfa', 500: '#8b5cf6' },
          emerald: '#10b981',
          amber: '#f59e0b',
          red: '#ef4444',
          cyan: '#06b6d4',
        },
      },
      fontFamily: {
        heading: ['Sora', 'system-ui', 'sans-serif'],
        body: ['Inter', 'system-ui', 'sans-serif'],
      },
    },
  },
  plugins: [],
} satisfies Config;
