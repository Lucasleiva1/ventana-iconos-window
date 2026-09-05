const { chromium } = require(process.env.PLAYWRIGHT_PACKAGE || 'playwright');
const assert = require('node:assert/strict');
(async () => {
  const browser = await chromium.launch({ channel: 'msedge', headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 430, height: 300 } });
    const errors = [];
    page.on('pageerror', error => errors.push(String(error)));
    await page.route('**/reorder-fixture.html', route => route.fulfill({ contentType: 'text/html', body: '<html><body><div id="root"></div><script type="module">import RefreshRuntime from "/@react-refresh"; RefreshRuntime.injectIntoGlobalHook(window); window.$RefreshReg$ = () => {}; window.$RefreshSig$ = () => type => type; window.__vite_plugin_react_preamble_installed__ = true;</script><script type="module" src="/tests/reorder-fixture.tsx"></script></body></html>' }));
    await page.goto('http://127.0.0.1:1420/reorder-fixture.html');
    await page.locator('[data-item-id="a"]').waitFor();
    const order = () => page.locator('[data-item-id]').evaluateAll(nodes => nodes.map(node => node.dataset.itemId));
    for (const id of ['a', 'b', 'c']) await page.locator(`[data-item-id="${id}"]`).dblclick();
    await page.waitForFunction(() => window.openedItems?.join(',') === 'a,b,c');
    assert.deepEqual(await order(), ['a', 'b', 'c', 'd'], 'double-click must open files, folders, and shortcuts without reordering');
    async function drag(from, to) {
      const a = await page.locator(`[data-item-id="${from}"]`).boundingBox();
      const b = await page.locator(`[data-item-id="${to}"]`).boundingBox();
      await page.mouse.move(a.x + a.width / 2, a.y + 40);
      await page.mouse.down();
      await page.mouse.move(b.x + b.width / 2, b.y + 40, { steps: 8 });
    }
    await drag('a', 'c');
    assert.deepEqual(await order(), ['b', 'c', 'a', 'd'], 'neighbors must move BEFORE release');
    assert.equal(await page.evaluate(() => window.savedOrder), undefined, 'preview must not persist before release');
    await page.mouse.up();
    await page.waitForFunction(() => window.savedOrder?.join(',') === 'b,c,a,d');
    await drag('d', 'b');
    await page.keyboard.press('Escape');
    await page.mouse.up();
    assert.deepEqual(await order(), ['b', 'c', 'a', 'd'], 'Escape restores saved order');
    assert.deepEqual(errors, []);
    console.log('PASS: double-click opens files/folders/shortcuts; pointer drag, animated preview, save, and Escape work');
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
