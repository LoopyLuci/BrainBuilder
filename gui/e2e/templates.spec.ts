import { test, expect } from '@playwright/test';
import { installTauriMock } from './tauriMock';

// Covers src/templates/TemplatesPanel.tsx: picking a template should
// instantiate its nodes onto the reactflow canvas via useApplyTemplate.
test.describe('templates panel', () => {
  test.beforeEach(async ({ page }) => {
    await installTauriMock(page);
    await page.goto('/');
    // Descriptors load async (ComponentPalette's useEffect); wait for the
    // template's "Use" button to become enabled, which only happens once
    // missingComponents() sees the mocked `linear`/`gelu` descriptors.
    await expect(page.locator('.react-flow')).toBeVisible();
  });

  test('applying MLP Classifier renders nodes on the canvas', async ({ page }) => {
    const templatesPanel = page.locator('text=Templates').first();
    await expect(templatesPanel).toBeVisible();

    const mlpRow = page.locator('li.bb-list-item', { hasText: 'MLP Classifier' });
    await expect(mlpRow).toBeVisible();

    const useButton = mlpRow.getByRole('button', { name: 'Use' });
    await expect(useButton).toBeEnabled({ timeout: 10_000 });
    await useButton.click();

    // The template lays out 3 nodes (linear -> gelu -> linear); reactflow
    // renders each as a `.react-flow__node`.
    await expect(page.locator('.react-flow__node')).toHaveCount(3);
  });
});
