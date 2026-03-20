import { defineWorkspace } from 'vitest/config';

export default defineWorkspace([
  {
    test: {
      name: 'services',
      include: ['services/*/src/*.test.js'],
      environment: 'node',
      coverage: {
        provider: 'v8',
        include: ['services/*/src/index.js'],
        thresholds: { lines: 80, functions: 80, branches: 70, statements: 80 },
      },
    },
  },
  {
    esbuild: { jsx: 'automatic' },
    test: {
      name: 'web',
      include: ['web/src/**/*.test.{js,jsx}'],
      environment: 'jsdom',
      globals: true,
      setupFiles: ['web/src/test-setup.js'],
      coverage: {
        provider: 'v8',
        include: ['web/src/**/*.{js,jsx}'],
        exclude: ['web/src/main.jsx'],
        thresholds: { lines: 70, functions: 70, branches: 60, statements: 70 },
      },
    },
  },
]);
