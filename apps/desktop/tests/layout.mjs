import assert from "node:assert/strict";
import { chromium } from "playwright-core";
import { preview } from "vite";
import { testRamCard } from "./ram.mjs";

// Только собранный локальный интерфейс. Системный браузер задаёт среда проверки.
const server = await preview({ preview: { host: "127.0.0.1", port: 0 } });
let browser;
try {
  browser = await chromium.launch({ executablePath: process.env.CHROMIUM_BINARY || "google-chrome", args: ["--disable-dev-shm-usage"] });
  const page = await browser.newPage();
  const errors = [];
  const origin = `http://127.0.0.1:${server.httpServer.address().port}`;
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("request", (req) => { if (new URL(req.url()).origin !== origin) errors.push(req.url()); });
  await page.addInitScript(() => {
    window.previewInvokes = 0;
    window.__TAURI_INTERNALS__ = { invoke: () => { window.previewInvokes++; } };
  });
  await page.goto(origin);
  await page.getByRole("heading", { name: "Обзор системы" }).waitFor();
  await page.evaluate(() => document.fonts.ready);
  for (const [width, cols] of [[1920, 4], [1280, 2], [768, 1]]) {
    await page.setViewportSize({ width, height: 960 });
    const layout = await page.evaluate(() => {
      const rect = (s) => document.querySelector(s).getBoundingClientRect();
      return { cards: [...document.querySelectorAll(".metric")].map(c => ({ x: c.offsetLeft, y: c.offsetTop })),
        sidebar: rect(".sidebar").right, main: rect("main").left,
        donut: rect(".donut-panel").top, chart: rect(".chart-panel").top,
        content: document.querySelector("main").scrollWidth, available: document.querySelector("main").clientWidth,
        fullWidth: document.documentElement.scrollWidth };
    });
    assert.equal(layout.cards.length, 4);
    assert.equal(new Set(layout.cards.map(c => c.x)).size, cols);
    assert.equal(layout.cards.filter(c => c.y === layout.cards[0].y).length, cols);
    assert.equal(layout.sidebar, 260);
    assert.ok(layout.main >= layout.sidebar && layout.fullWidth === width);
    assert.ok(layout.content <= layout.available, "Содержимое не должно вытеснять правый край");
    if (width === 1920) assert.equal(layout.chart, layout.donut);
    assert.equal(await page.locator(".metric .no-data").filter({ hasText: "нет данных" }).count(), 4);
    for (const text of await page.locator(".metric-value").allTextContents()) { assert.match(text, /—/); assert.doesNotMatch(text, /\d|NaN/); }
    if (process.env.SCREENSHOT_DIR) await page.screenshot({ path: `${process.env.SCREENSHOT_DIR}/meshpilot-${width}.png` });
    console.log(`${width}px: ${cols} колонки, без перекрытия, нет подставных измерений`);
  }
  for (const name of ["CleanUp", "Mirrors & DNS", "Permissions", "Settings & Tray"]) {
    await page.getByRole("navigation").getByRole("link", { name, exact: true }).click();
    await page.getByRole("link", { name, exact: true, current: "page" }).waitFor();
    await page.getByRole("heading", { name: "Предварительный экран" }).waitFor();
  }
  await page.getByRole("link", { name: "На Dashboard" }).click();
  await page.getByRole("link", { name: "Быстрая очистка" }).click();
  await page.getByRole("heading", { name: "План очистки" }).waitFor();
  assert.equal(await page.locator("nav button:disabled").count(), 2);
  assert.deepEqual(errors, []);
  assert.equal(await page.evaluate(() => window.previewInvokes), 0);
  await testRamCard(browser, origin);
  console.log("Навигация, пустые состояния и отсутствие внешних запросов: ОК");
} finally {
  await browser?.close();
  await new Promise(resolve => server.httpServer.close(resolve));
}
