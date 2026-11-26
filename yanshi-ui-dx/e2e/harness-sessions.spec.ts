import { expect, test } from '@playwright/test';

const baseURL = process.env.BASE_URL ?? 'http://127.0.0.1:8080';
const apiToken = process.env.YANSHI_API_TOKEN ?? '';

test.describe('Harness Mode (Dioxus 0.7) — /ui/harness/sessions', () => {
  test('submits a session, opens detail view, and navigates via tenant-scoped list', async ({
    page,
  }) => {
    const tenant = `e2e-harness-${Date.now()}`;

    await page.goto(`${baseURL}/ui/harness/sessions/new`);
    await expect(page.getByTestId('dx-harness-new-page')).toBeVisible();

    if (apiToken) {
      await page.getByTestId('harness-new-token').fill(apiToken);
    }
    await page.getByTestId('harness-new-tenant').fill(tenant);
    await page.getByTestId('harness-new-profile').selectOption('local');
    await page.getByTestId('harness-new-runtime').selectOption('react');
    await page.getByTestId('harness-new-model').fill(process.env.E2E_MOCK_MODEL ?? 'e2e-mock');
    await page.getByTestId('harness-new-workspace').fill('/tmp');
    await page.getByTestId('harness-new-prompt').fill('Reply with a one-word answer.');

    await page.getByTestId('harness-new-submit').click();
    await page.waitForURL(/\/ui\/harness\/sessions\/[0-9a-f-]+(\?.*)?$/i);
    await expect(page.getByTestId('dx-harness-detail-page')).toBeVisible();
    await expect(page.getByTestId('harness-approvals-section')).toBeVisible();

    await page.goto(`${baseURL}/ui/harness/sessions`);
    await expect(page.getByTestId('dx-harness-list-page')).toBeVisible();
    await page.getByTestId('harness-list-tenant').fill(tenant);
    await expect(page.getByTestId('harness-list-row').first()).toBeVisible({ timeout: 10_000 });
    await page.getByTestId('harness-list-row').first().click();
    await page.waitForURL(/\/ui\/harness\/sessions\/[0-9a-f-]+(\?.*)?$/i);
    await expect(page.getByTestId('dx-harness-detail-page')).toBeVisible();
  });
});
