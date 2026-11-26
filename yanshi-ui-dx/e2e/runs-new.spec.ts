import { expect, test } from '@playwright/test';

const baseURL = process.env.BASE_URL ?? 'http://127.0.0.1:8080';
const apiToken = process.env.YANSHI_API_TOKEN ?? '';

test.describe('Run Create (Dioxus 0.7) — /ui/runs/new', () => {
  test('submits a workflow and renders live run id', async ({ page }) => {
    await page.goto(`${baseURL}/ui/runs/new`);
    await expect(page.getByTestId('dx-run-create-page')).toBeVisible();

    await page.getByTestId('create-tenant').fill(`e2e-run-${Date.now()}`);
    await page.getByTestId('create-profile').selectOption('local');
    if (apiToken) {
      await page.getByTestId('create-token').fill(apiToken);
    }
    await page.getByTestId('create-workflow').fill(
      [
        'name: e2e-dx-run',
        'version: "1.0"',
        'nodes:',
        '  - id: greet',
        '    type: template',
        '    parameters:',
        '      template: "hi from dioxus playwright"',
      ].join('\n'),
    );
    await page.getByTestId('create-inputs').fill('{}');
    await page.getByTestId('create-submit').click();

    await expect(page.getByTestId('create-run-id')).toHaveText(/[0-9a-f-]{8,}/i, {
      timeout: 15_000,
    });
  });

  test('does NOT persist the API token in localStorage across reloads', async ({ page }) => {
    await page.goto(`${baseURL}/ui/runs/new`);
    await page.getByTestId('create-token').fill('SECRET_DO_NOT_PERSIST');
    await page.reload();

    const persistedValues = await page.evaluate(() => {
      try {
        return Object.values(window.localStorage);
      } catch {
        return [];
      }
    });
    expect(persistedValues.join('\n')).not.toContain('SECRET_DO_NOT_PERSIST');
  });
});
