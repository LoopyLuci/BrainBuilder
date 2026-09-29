import { defineConfig } from 'vitest/config';

// Frontend unit tests. jsdom gives the pure stores a `localStorage` +
// `window` to exercise their persistence paths without a browser.
export default defineConfig({
  test: {
    environment: 'jsdom',
    include: ['src/**/*.test.ts'],
  },
});
