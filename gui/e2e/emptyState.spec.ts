import { test, expect } from '@playwright/test';
import { installTauriMock } from './tauriMock';

// Covers src/canvas/CanvasEmptyState.tsx: shown when the graph has 0 nodes,
// and should disappear once a template is applied (InfiniteCanvas.tsx only
// renders it when `nodes.length === 0`).
test.describe('canvas empty state', () => {
  test.beforeEach(async ({ page }) => {
    await installTauriMock(page);
    await page.goto('/');
  });

  test('shows the start-from-template card on a fresh graph, then hides it once applied', async ({ page }) => {
    const emptyCard = page.locator('.bb-canvas-empty__card');
    await expect(emptyCard).toBeVisible();
    await expect(emptyCard.locator('.bb-canvas-empty__title')).toHaveText('Start from a template');

    const mlpRow = emptyCard.locator('li.bb-list-item', { hasText: 'MLP Classifier' });
    const useButton = mlpRow.getByRole('button', { name: 'Use' });
    await expect(useButton).toBeEnabled({ timeout: 10_000 });
    await useButton.click();

    await expect(page.locator('.react-flow__node')).toHaveCount(3);
    await expect(emptyCard).toHaveCount(0);
  });
});
