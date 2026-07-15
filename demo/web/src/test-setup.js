import '@testing-library/jest-dom';
import { vi } from 'vitest';

// Mock fetch for all tests - can be overridden per-test
global.fetch = vi.fn(() =>
  Promise.resolve({ ok: true, json: () => Promise.resolve([]) })
);
