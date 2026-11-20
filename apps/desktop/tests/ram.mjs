import assert from "node:assert/strict";

// Только тест браузерного представления IPC, не замена наблюдателю реального окна в VM.
export async function testRamCard(browser, origin) {
  const page = await browser.newPage({ viewport: { width: 1281, height: 900 } });
  const errors = [];
  page.on("pageerror", e => errors.push(e.message));
  page.on("request", r => { if (new URL(r.url()).origin !== origin) errors.push(r.url()); });
  await page.addInitScript(() => {
    const sample = { used_bytes: 1.5 * 2 ** 30, total_bytes: 8 * 2 ** 30, source: "/proc/meminfo", observed_at_utc: "2026-11-20T12:34:56.789Z" };
    window.isTauri = true;
    window.ramCalls = [];
    window.__TAURI_INTERNALS__ = { invoke: async (cmd, args) => {
      window.ramCalls.push([cmd, args]);
      switch (window.ramCalls.length) {
        case 1: return sample;
        case 2: return { ...sample, used_bytes: 0 };
        case 3: return null;
        case 4: return { ...sample, used_bytes: sample.total_bytes + 1 };
        case 5: return { ...sample, observed_at_utc: "2026-02-30T12:34:56.789Z" };
        default: throw new Error("IPC недоступен");
      }
    } };
  });
  try {
    await page.goto(origin);
    const card = page.getByRole("article", { name: /^Занято RAM/ });
    const refresh = page.getByRole("button", { name: "Обновить снимок RAM" });
    await page.getByRole("article", { name: "Занято RAM 1,5 ГиБ из 8 ГиБ /proc/meminfo 2026-11-20 12:34:56 UTC", exact: true }).waitFor();
    assert.equal(await card.locator("time").getAttribute("datetime"), "2026-11-20T12:34:56.789Z");
    assert.match(await page.locator("footer").innerText(), /ram 1,5 ГиБ/);
    assert.equal(await page.locator(".metric .no-data").count(), 3);
    assert.equal(await page.getByText("Нет истории измерений", { exact: true }).count(), 1);
    for (const width of [1281, 800, 640]) {
      await page.setViewportSize({ width, height: 900 });
      assert.ok(await card.evaluate(el => el.scrollWidth <= el.clientWidth && el.scrollHeight <= el.clientHeight), `RAM: ${width}px`);
    }
    await refresh.click();
    await card.getByText("0 Б", { exact: true }).waitFor();
    assert.match(await page.locator("footer").innerText(), /ram 0 Б/);
    for (let n = 3; n <= 6; n++) {
      await refresh.click();
      await page.waitForFunction(n => window.ramCalls.length === n && document.querySelector(".ram").getAttribute("aria-busy") === "false", n);
      assert.equal(await card.locator(".no-data").innerText(), "нет данных");
      assert.equal(await card.locator(".metric-value").innerText(), "—");
      assert.equal(await card.locator("time").count(), 0);
      assert.match(await page.locator("footer").innerText(), /ram —/);
    }
    assert.deepEqual(await page.evaluate(() => window.ramCalls), Array.from({ length: 6 }, () => ["read_ram", {}]));
    assert.deepEqual(errors, []);
    console.log("Карточка и строка состояния: замер IPC, измеренный ноль, отказ и неверные данные; источник, UTC, единицы и ручное обновление: ОК");
  } finally { await page.close(); }
}
