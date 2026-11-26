import { expect, test } from '@playwright/test';

const baseURL = process.env.BASE_URL ?? 'http://127.0.0.1:8080';

test.describe('Yanshi Studio (Dioxus 0.7) — /ui/studio, redirects, strict CSP & diagnostics', () => {
  test('redirects / and /ui-dx/* to canonical /ui/* and renders DAG Studio with zero CSP violations', async ({
    page,
    request,
  }) => {
    const rootRes = await request.get(`${baseURL}/`, { maxRedirects: 0 });
    expect(rootRes.status()).toBe(307);
    expect(rootRes.headers()['location']).toBe('/ui/studio');

    const dxRes = await request.get(`${baseURL}/ui-dx/notebooks/demo?tab=sources`, {
      maxRedirects: 0,
    });
    expect(dxRes.status()).toBe(307);
    expect(dxRes.headers()['location']).toBe('/ui/notebooks/demo?tab=sources');

    const studioRes = await request.get(`${baseURL}/ui/studio`);
    expect(studioRes.status()).toBe(200);
    const csp = studioRes.headers()['content-security-policy'] ?? '';
    expect(csp).toContain("script-src 'self' 'wasm-unsafe-eval'");
    expect(csp).not.toContain("'unsafe-eval'");

    const cspViolations: string[] = [];
    const pageErrors: string[] = [];
    page.on('pageerror', (err) => pageErrors.push(err.message));
    await page.addInitScript(() => {
      window.addEventListener('securitypolicyviolation', (ev) => {
        (window as unknown as { __cspViolations?: string[] }).__cspViolations = [
          ...((window as unknown as { __cspViolations?: string[] }).__cspViolations ?? []),
          `${ev.violatedDirective}:${ev.blockedURI}`,
        ];
      });
    });

    await page.goto(`${baseURL}/ui/studio`);
    await expect(page.getByTestId('workflow-studio-page')).toBeVisible();
    await expect(page.getByTestId('workflow-graph-canvas')).toBeVisible();

    // Switch to Release (App Runner) mode and verify preset selector mounts.
    await page.getByTestId('mode-switch-release').click();
    await expect(page.getByTestId('release-preset-select')).toBeVisible();

    // Navigate to Diagnostics via top header bar.
    await page.getByTestId('nav-diagnostics').click();
    await page.waitForURL(/\/ui\/diagnostics$/);
    await expect(page.getByTestId('dx-diagnostics-page')).toBeVisible();

    const collected = await page.evaluate(
      () => (window as unknown as { __cspViolations?: string[] }).__cspViolations ?? [],
    );
    cspViolations.push(...collected);
    expect(cspViolations).toEqual([]);
    expect(pageErrors).toEqual([]);
  });
});
