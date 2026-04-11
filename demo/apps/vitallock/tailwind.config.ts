import type { Config } from 'tailwindcss';

export default {
  content: ['./index.html', './src/**/*.{ts,tsx}'],
  theme: {
    extend: {
      colors: {
        vl: {
          emerald: '#34d399',
          dark: '#0a0a0a',
          zinc: {
            800: '#27272a',
            900: '#18181b',
            950: '#09090b',
          },
        },
      },
    },
  },
  plugins: [],
} satisfies Config;
