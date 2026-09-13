import { test, expect } from '@playwright/test';

async function ready(page) {
  await page.goto('./');
  await expect(page.getByRole('button', {name:'Next target'})).toBeEnabled();
  await expect(page.locator('#terminal')).toContainText('All Targets Latency Overlay');
}
async function add(page, text) {
  await page.getByRole('textbox').fill(text);
  await page.getByRole('textbox').press('Enter');
}

test('minimal page, buttons, arrow keys and Vim navigation', async ({page}, info) => {
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await ready(page);
  await expect(page.getByRole('link')).toHaveCount(0);
  await expect(page.getByRole('button')).toHaveCount(2);
  await page.getByRole('button', {name:'Next target'}).click();
  await expect(page.locator('#terminal')).toContainText('Target: Gateway');
  await page.keyboard.press('l');
  await expect(page.locator('#terminal')).toContainText('Target: DNS');
  await page.keyboard.press('ArrowRight');
  await expect(page.locator('#terminal')).toContainText('Target: API');
  await page.keyboard.press('h');
  await expect(page.locator('#terminal')).toContainText('Target: DNS');
  await page.keyboard.press('ArrowLeft');
  await expect(page.locator('#terminal')).toContainText('Target: Gateway');
  await page.getByRole('button', {name:'Previous target'}).click();
  await expect(page.locator('#terminal')).toContainText('All Targets Overview');
  await page.keyboard.press('p');
  await expect(page.locator('#terminal')).toContainText('All Targets Ping Latency');
  await page.keyboard.press('p');
  await expect(page.locator('#terminal')).toContainText('No failures recorded');
  await page.keyboard.press('p');
  expect(await page.locator('#terminal span').evaluateAll(spans => new Set(spans.map(span => getComputedStyle(span).color)).size)).toBeGreaterThan(4);
  await page.screenshot({path:info.outputPath('minimal.png')});
  expect(errors).toEqual([]);
});

test('add named URLs and hosts, keep updating beyond four targets, validate input', async ({page}, info) => {
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await ready(page);
  await add(page, 'My API = https://example.com/v1?health=1');
  await expect(page.locator('#terminal')).toContainText('Target: My API (example.com)');
  await expect(page.getByRole('textbox')).toHaveValue('');
  await page.waitForTimeout(1200);
  await expect(page.locator('#terminal')).toContainText('Mean:');
  // Typing Vim keys and moving the input caret must not navigate the monitor.
  await page.getByRole('textbox').fill('hello');
  await page.getByRole('textbox').press('ArrowLeft');
  await page.getByRole('textbox').press('h');
  await expect(page.locator('#terminal')).toContainText('Target: My API');
  await add(page, 'Router = 10.0.0.1');
  await expect(page.locator('#terminal')).toContainText('Target: Router (10.0.0.1)');
  await add(page, 'https://www.example.org/path?q=1');
  await expect(page.locator('#terminal')).toContainText('Target: www.example.org (www.example.org)');
  await add(page, 'Local = localhost');
  await expect(page.locator('#terminal')).toContainText('Target: Local (localhost)');
  await add(page, 'IPv6 = [::1]');
  await expect(page.locator('#terminal')).toContainText('Target: IPv6 ([::1])');
  await add(page, 'Duplicate = https://EXAMPLE.COM/path');
  await expect(page.locator('#status')).toContainText('already');
  await add(page, 'Bad = ftp://example.net');
  await expect(page.locator('#status')).toContainText('HTTP(S)');
  await add(page, 'not a host');
  await expect(page.locator('#status')).toHaveClass('error');
  await add(page, 'Bad = https://user:password@example.net');
  await expect(page.locator('#status')).toContainText('credentials');
  for (let i=0;i<7;i++) await add(page, `Host ${i} = host${i}.example`);
  await add(page, 'Overflow = overflow.example');
  await expect(page.locator('#status')).toContainText('16 hosts');
  await page.getByRole('button', {name:'Next target'}).click();
  await expect(page.locator('#terminal')).toContainText('Monitoring 16 targets');
  await page.screenshot({path:info.outputPath('added-hosts.png')});
  expect(errors).toEqual([]);
});

test('mobile stays within viewport and can add a host', async ({page}, info) => {
  await page.setViewportSize({width:390,height:844});
  await ready(page);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBeTruthy();
  await add(page, 'Home = home.example');
  await expect(page.locator('#terminal')).toContainText('Target: Home');
  await page.getByRole('button', {name:'Previous target'}).click();
  await expect(page.locator('#terminal')).toContainText('Target: Edge');
  await page.screenshot({path:info.outputPath('mobile.png'),fullPage:true});
});

test('failed Wasm load gives clear reload recovery', async ({page}) => {
  await page.route('**/*.wasm', route => route.abort());
  await page.goto('./');
  await expect(page.locator('#status')).toContainText('Reload');
  await expect(page.getByRole('textbox')).toBeDisabled();
  await page.unroute('**/*.wasm');
  await ready(page);
});

test('adding from failure view opens latency and keeps cells mounted on tick', async ({page}) => {
  await ready(page);
  await page.locator('#terminal').focus();
  await page.keyboard.press('p');
  await page.keyboard.press('p');
  await expect(page.locator('#terminal')).toContainText('No failures recorded');
  await add(page, 'Google = google.com');
  await expect(page.locator('#terminal')).toContainText('Target: Google (google.com)');
  await expect(page.locator('#terminal')).toContainText('Ping Latency (ms)');
  await expect(page.locator('#terminal')).toBeFocused();
  await page.locator('#terminal .cell').first().evaluate(cell => { window.originalCell = cell; });
  await page.waitForTimeout(1200);
  expect(await page.evaluate(() => window.originalCell === document.querySelector('#terminal .cell'))).toBeTruthy();
  await page.keyboard.press('h');
  await expect(page.locator('#terminal')).toContainText('Target: Edge');
});
