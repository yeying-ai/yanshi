/**
 * Playwright E2E test for `/ui/notebooks` (`Track AC` & `Track AD`).
 *
 * Exercises the Content Studio Notebook workspace against a live `yanshi-server`
 * without requiring any external LLM API keys:
 *   1. Creates a notebook (`POST /v1/notebooks`) and verifies URL routing.
 *   2. Renames the notebook inline (`PATCH /v1/notebooks/:id`).
 *   3. Ingests a pasted text source (`POST /v1/notebooks/:id/sources`) and
 *      waits for background chunking to settle at `已就绪`.
 *   4. Runs BM25 passage search (`GET /v1/notebooks/:id/search`) and opens
 *      the cited chunk inside the native `<dialog>` preview.
 *   5. Exports a portable `.json` bundle (`GET /v1/notebooks/:id/export`),
 *      re-imports it (`POST /v1/notebooks/import`), and verifies immediate BM25
 *      search on the imported notebook.
 *   6. Deletes both notebooks (`DELETE /v1/notebooks/:id`) via the confirmation
 *      `<dialog>`.
 */

// eslint-disable-next-line @typescript-eslint/ban-ts-comment
// @ts-ignore — @playwright/test is an optional dev dep installed in e2e jobs.
import { expect, test } from '@playwright/test';

const env = (globalThis as unknown as { process: { env: Record<string, string | undefined> } })
  .process.env;
const baseURL = env.BASE_URL ?? 'http://127.0.0.1:8080';
const apiToken = env.YANSHI_API_TOKEN ?? '';
const mockModel = env.E2E_MOCK_MODEL ?? '';

test.describe('Content Studio — /ui/notebooks lifecycle, chunk reader, batch studio & bundle round-trip', () => {
  test('creates, renames, reads multi-chunk source, batch-generates deliverables, exports/imports, and deletes', async ({
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
    const originalUrl = page.url();

    // 2. Inline rename (`PATCH /v1/notebooks/:id`)
    await page.getByTestId('open-rename-notebook-btn').click();
    await page.getByTestId('rename-notebook-input').fill('Macrotidal Estuaries');
    await page.getByTestId('save-rename-notebook-btn').click();
    await expect(page.getByTestId('notebook-selector')).toContainText('Macrotidal Estuaries');

    // 3. Add a multi-paragraph (>1200 chars) text source so chunking produces >= 2 passages
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
    await page.getByTestId('source-text-input').fill(`${paraA}\n\n${paraB}`);
    await page.getByTestId('submit-source-btn').click();
    await expect(page.getByTestId('source-scope-bar')).toContainText('1/1 份就绪资料', {
      timeout: 15_000,
    });

    // 4. Open Full Source Passage Reader (`GET /v1/notebooks/:id/sources/:source_id/chunks`) and step Next/Prev
    await page.getByRole('button', { name: /阅读分块/ }).first().click();
    await expect(page.getByTestId('chunk-preview-dialog')).toContainText('第 1 /');
    await expect(page.getByTestId('chunk-reader-body')).toContainText('Severn Bore');
    await page.getByTestId('chunk-reader-next-btn').click();
    await expect(page.getByTestId('chunk-preview-dialog')).toContainText('第 2 /');
    await expect(page.getByTestId('chunk-reader-body')).toContainText('Bay of Fundy');
    await page.getByTestId('chunk-reader-close-btn').click();

    // 5. Run BM25 search & inspect chunk in native `<dialog>`
    await page.getByTestId('notebook-search-input').fill('equinoctial');
    await page.getByTestId('notebook-search-submit').click();
    const hitInspectBtn = page.getByRole('button', { name: /查看分块/ }).first();
    await expect(hitInspectBtn).toBeVisible();
    await hitInspectBtn.click();
    await expect(page.getByTestId('chunk-preview-dialog')).toContainText('Severn Bore');
    await page.getByTestId('chunk-reader-close-btn').click();

    // 5b. Inline source edit (`PATCH /v1/notebooks/:id/sources/:source_id`) -> immediate BM25 re-index
    await page.getByRole('button', { name: /编辑来源/ }).first().click();
    await expect(page.getByTestId('edit-source-dialog')).toBeVisible();
    const editTextInput = page.getByTestId('edit-source-text-input');
    await expect(editTextInput).toBeEnabled();
    const existingText = await editTextInput.inputValue();
    expect(existingText).toContain('Severn Bore');
    await page.getByTestId('edit-source-title-input').fill('Macrotidal Field Notes');
    await editTextInput.fill(
      `${existingText}\n\nMont-Saint-Michel spring tidal amplitude reaches fourteen meters.`,
    );
    await page.getByTestId('save-edit-source-btn').click();
    await expect(page.locator('.nb-source-list')).toContainText('Macrotidal Field Notes');

    await page.getByTestId('notebook-search-input').fill('Mont-Saint-Michel');
    await page.getByTestId('notebook-search-submit').click();
    await expect(page.locator('.nb-hit-card').first()).toContainText('Mont-Saint-Michel');

    // 6. When `E2E_MOCK_MODEL` is configured (CI + local E2E harness), verify multi-select
    //    `generate-batch` (`flashcards` + `quiz`) -> live concurrent tracking -> interactive viewers +
    //    non-default tenant `blob:` media preview + `.zip` download!
    if (mockModel) {
      await page.getByTestId('notebook-model-input').fill(mockModel);
      await page.getByTestId('notebook-cost-limit-input').fill('4.00');
      await page.getByTestId('batch-select-flashcards').check();
      await page.getByTestId('batch-select-quiz').check();
      await page.getByTestId('launch-batch-selected-btn').click();

      await expect(page.getByTestId('notebook-live-run')).toBeVisible();
      await expect(page.locator('.nb-artifact-block')).toHaveCount(2, { timeout: 20_000 });

      // Interactive flashcards flip & quiz self-grading
      await expect(page.getByTestId('interactive-flashcards')).toBeVisible();
      await expect(page.getByTestId('flashcard-flip-btn')).toHaveAttribute('aria-pressed', 'false');
      await page.getByTestId('flashcard-flip-btn').click();
      await expect(page.getByTestId('flashcard-flip-btn')).toHaveAttribute('aria-pressed', 'true');

      await expect(page.getByTestId('interactive-quiz')).toBeVisible();
      await page.getByTestId('interactive-quiz').locator('input[type="radio"]').nth(1).check();
      await expect(page.getByTestId('quiz-score-banner')).toContainText('自测得分：1 / 1');

      // AuthorizedMedia under custom non-`default` tenant loads via authenticated `blob:` URL
      await expect(page.locator('.studio-media-error')).toHaveCount(0);
      await expect(page.locator('.studio-media-iframe').first()).toHaveAttribute('src', /^blob:/);

      const zipDownloadPromise = page.waitForEvent('download');
      await page.getByTestId('download-notebook-zip-btn').click();
      const zipDownload = await zipDownloadPromise;
      expect(zipDownload.suggestedFilename()).toMatch(
        /^yanshi-notebook-[0-9a-f]{8}-artifacts\.zip$/i,
      );
    }

    // 5. Export portable `.json` bundle & re-import into a second notebook
    const downloadPromise = page.waitForEvent('download');
    await page.getByTestId('export-notebook-json-btn').click();
    const download = await downloadPromise;
    expect(download.suggestedFilename()).toMatch(/^yanshi-notebook-[0-9a-f]{8}\.json$/i);
    const downloadedPath = await download.path();
    expect(downloadedPath).toBeTruthy();

    await page.getByTestId('import-notebook-file-input').setInputFiles(downloadedPath!);
    await page.getByTestId('import-notebook-submit-btn').click();
    await expect
      .poll(() => page.url(), { timeout: 10_000 })
      .not.toBe(originalUrl);
    await expect(page.getByTestId('source-scope-bar')).toContainText('1/1 份就绪资料');

    // Verify imported notebook is immediately searchable via BM25
    await page.getByTestId('notebook-search-input').fill('equinoctial');
    await page.getByTestId('notebook-search-submit').click();
    await expect(page.getByRole('button', { name: /查看分块/ }).first()).toBeVisible();

    // 6. Delete imported notebook, then delete original notebook
    await page.getByTestId('open-delete-notebook-btn').click();
    await page.getByTestId('confirm-delete-notebook-btn').click();
    await expect(page).toHaveURL(originalUrl);

    await page.getByTestId('open-delete-notebook-btn').click();
    await page.getByTestId('confirm-delete-notebook-btn').click();
    await page.waitForURL(/\/ui\/notebooks$/);
  });
});
