/**
 * Playwright E2E test for `/ui/notebooks` (`Track AC`).
 *
 * Exercises the Content Studio Notebook workspace against a live `yanshi-server`
 * without requiring any external LLM API keys:
 *   1. Creates a notebook (`POST /v1/notebooks`) and verifies URL routing.
 *   2. Renames the notebook inline (`PATCH /v1/notebooks/:id`).
 *   3. Ingests a pasted text source (`POST /v1/notebooks/:id/sources`) and
 *      waits for background chunking to settle at `已就绪`.
 *   4. Runs BM25 passage search (`GET /v1/notebooks/:id/search`) and opens
 *      the cited chunk inside the native `<dialog>` preview.
 *   5. Deletes the notebook (`DELETE /v1/notebooks/:id`) via the confirmation
 *      `<dialog>`.
 */

// eslint-disable-next-line @typescript-eslint/ban-ts-comment
// @ts-ignore — @playwright/test is an optional dev dep installed in e2e jobs.
import { expect, test } from '@playwright/test';

const baseURL = process.env.BASE_URL ?? 'http://127.0.0.1:8080';
const apiToken = process.env.YANSHI_API_TOKEN ?? '';

test.describe('Content Studio — /ui/notebooks lifecycle & search', () => {
  test('creates, renames, ingests text, searches BM25 chunks, previews citation dialog, and deletes', async ({
    page,
  }) => {
    await page.goto(`${baseURL}/ui/notebooks`);
    await expect(page.getByTestId('notebook-workspace-page')).toBeVisible();

    const uniqueTenant = `e2e-nb-${Date.now()}`;
    await page.locator('#nb-tenant-input').fill(uniqueTenant);
    if (apiToken) {
      await page.locator('#nb-token-input').fill(apiToken);
    }

    // 1. Create notebook
    await page.getByTestId('new-notebook-title-input').fill('Tidal Draft');
    await page.getByTestId('create-notebook-btn').click();
    await page.waitForURL(/\/ui\/notebooks\/[0-9a-f-]+$/i);

    // 2. Inline rename (`PATCH /v1/notebooks/:id`)
    await page.getByTestId('open-rename-notebook-btn').click();
    await page.getByTestId('rename-notebook-input').fill('Macrotidal Estuaries');
    await page.getByTestId('save-rename-notebook-btn').click();
    await expect(page.getByTestId('notebook-selector')).toContainText('Macrotidal Estuaries');

    // 3. Add text source & wait until background ingestion reaches `已就绪`
    await page
      .getByTestId('source-text-input')
      .fill(
        'Severn Bore macrotidal range reaches fifteen meters during equinoctial spring tides.',
      );
    await page.getByTestId('submit-source-btn').click();
    await expect(page.getByTestId('source-scope-bar')).toContainText('1/1 份就绪资料', {
      timeout: 15_000,
    });

    // 4. Run BM25 search & inspect chunk in native `<dialog>`
    await page.getByTestId('notebook-search-input').fill('equinoctial');
    await page.getByTestId('notebook-search-submit').click();
    const hitInspectBtn = page.getByRole('button', { name: /查看分块/ }).first();
    await expect(hitInspectBtn).toBeVisible();
    await hitInspectBtn.click();
    await expect(page.getByTestId('chunk-preview-dialog')).toContainText('Severn Bore');
    await page
      .getByTestId('chunk-preview-dialog')
      .getByRole('button', { name: '关闭' })
      .click();

    // 5. Delete notebook (`DELETE /v1/notebooks/:id`)
    await page.getByTestId('open-delete-notebook-btn').click();
    await page.getByTestId('confirm-delete-notebook-btn').click();
    await page.waitForURL(/\/ui\/notebooks$/);
  });
});
