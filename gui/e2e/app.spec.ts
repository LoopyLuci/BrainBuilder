import { test, expect } from '@playwright/test';
import { installTauriMock } from './tauriMock';

test.describe('app shell', () => {
  test.beforeEach(async ({ page }) => {
    await installTauriMock(page);
  });

  test('loads without a blank screen or console error, and shows chrome', async ({ page }) => {
    const pageErrors: string[] = [];
    page.on('pageerror', (err) => pageErrors.push(String(err)));

    await page.goto('/');

    // Header brand text from src/shell/Header.tsx.
    await expect(page.locator('.bb-header')).toBeVisible();
    await expect(page.getByText('BrainBuilder', { exact: true })).toBeVisible();

    // The reactflow canvas mounts inside .canvas-container (src/App.tsx).
    await expect(page.locator('.canvas-container .react-flow')).toBeVisible();

    expect(pageErrors, `Uncaught page errors: ${pageErrors.join('\n')}`).toEqual([]);
  });
});
