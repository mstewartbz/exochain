import '@testing-library/jest-dom';

// Mock fetch for all tests - can be overridden per-test
global.fetch = vi.fn(() =>
  Promise.resolve({ ok: true, json: () => Promise.resolve([]) })
);
