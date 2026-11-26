import { expect, test } from '@playwright/test';

const baseURL = process.env.BASE_URL ?? 'http://127.0.0.1:8080';
const apiToken = process.env.YANSHI_API_TOKEN ?? '';

test.describe('Content Studio (Dioxus 0.7) — /ui/notebooks lifecycle', () => {
  test('creates, renames, ingests multi-chunk text, reads passages, searches, edits raw_text, and deletes', async ({
    page,
  }) => {
    await page.goto(`${baseURL}/ui/notebooks`);
    await expect(page.getByTestId('notebook-workspace')).toBeVisible();

    const uniqueTenant = `e2e-nb-${Date.now()}`;
    await page.getByTestId('nb-tenant-input').fill(uniqueTenant);
    if (apiToken) {
      await page.getByTestId('nb-token-input').fill(apiToken);
    }

    // 1. Create notebook
    await page.getByTestId('new-notebook-title-input').fill('Tidal Draft');
    await page.getByTestId('create-notebook-btn').click();
    await page.waitForURL(/\/ui\/notebooks\/[0-9a-f-]+$/i);

    // 2. Rename inline
    await page.getByTestId('rename-notebook-btn').click();
    await page.getByTestId('rename-notebook-input').fill('Macrotidal Estuaries');
    await page.getByTestId('rename-notebook-save-btn').click();
    await expect(page.getByTestId('notebook-selector')).toContainText('Macrotidal Estuaries');

    // 3. Ingest multi-paragraph source
    const paraA =
      'Severn Bore macrotidal range reaches fifteen meters during equinoctial spring tides. ' +
      'Resonance along the funnel-shaped Bristol Channel amplifies incoming Atlantic tidal waves. '.repeat(
        8,
      );
    const paraB =
      'Bay of Fundy in eastern Canada exhibits a sixteen point three meter peak tidal amplitude. ' +
      'Natural basin oscillation period matches the twelve point four hour lunar semidiurnal tide. '.repeat(
        8,
      );
    await page.getByTestId('source-title-input').fill('Estuary Notes');
    await page.getByTestId('source-text-input').fill(`${paraA}\n\n${paraB}`);
    await page.getByTestId('ingest-source-submit-btn').click();
    await expect(page.getByTestId('notebook-sources-list')).toContainText('Estuary Notes', {
      timeout: 15_000,
    });
    await expect(page.getByTestId('sources-scope-summary')).toContainText('1/1', {
      timeout: 15_000,
    });

    // 4. Passage chunk reader
    await page.locator('[data-testid^="source-browse-chunks-"]').first().click();
    await expect(page.getByTestId('chunk-reader-dialog')).toBeVisible();
    await expect(page.getByTestId('chunk-reader-content')).toContainText('Severn Bore');
    await page.getByTestId('chunk-next-btn').click();
    await expect(page.getByTestId('chunk-reader-content')).toContainText('Bay of Fundy');
    await page.keyboard.press('Escape');
    await expect(page.getByTestId('chunk-reader-dialog')).not.toBeVisible();

    // 5. Lossless raw_text source edit + immediate BM25 re-index
    await page.locator('[data-testid^="source-edit-"]').first().click();
    await expect(page.getByTestId('edit-source-dialog')).toBeVisible();
    const editText = page.getByTestId('edit-source-text-input');
    await expect(editText).toHaveValue(`${paraA}\n\n${paraB}`);
    await page.getByTestId('edit-source-title-input').fill('Macrotidal Field Notes');
    await editText.fill(
      `${paraA}\n\n${paraB}\n\nMont-Saint-Michel spring tidal amplitude reaches fourteen meters.`,
    );
    await page.getByTestId('edit-source-save-btn').click();
    await expect(page.getByTestId('notebook-sources-list')).toContainText('Macrotidal Field Notes');

    // 6. BM25 search finds updated text
    await page.getByTestId('notebook-search-input').fill('Mont-Saint-Michel');
    await page.getByTestId('notebook-search-btn').click();
    await expect(page.getByTestId('notebook-search-results')).toContainText('Mont-Saint-Michel');

    // 7. Delete notebook
    await page.getByTestId('delete-notebook-btn').click();
    await expect(page.getByTestId('delete-notebook-dialog')).toBeVisible();
    await page.getByTestId('confirm-delete-notebook-btn').click();
    await page.waitForURL(/\/ui\/notebooks$/);
  });
});
