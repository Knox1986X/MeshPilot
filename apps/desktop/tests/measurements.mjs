import assert from "node:assert/strict";

// Фикстуры IPC проверяют представление. Нативные значения отдельно наблюдает CachyOS VM.
export async function testSnapshotCards(browser, origin) {
  const page = await browser.newPage({ viewport: { width: 1281, height: 900 } });
  const errors = [];
  page.on("pageerror", e => errors.push(e.message));
  page.on("request", r => { if (new URL(r.url()).origin !== origin) errors.push(r.url()); });
  await page.addInitScript(() => {
    const observed_at_utc = "2026-11-20T12:34:56.789Z";
    const ram = { used_bytes: 1.5 * 2 ** 30, total_bytes: 8 * 2 ** 30, source: "/proc/meminfo", observed_at_utc };
    const cpu = { busy_ticks: 3, total_ticks: 8, usage_percent: 37.5, interval_ms: 637, source: "/proc/stat", observed_at_utc };
    const replies = {
      ram: [ram, { ...ram, used_bytes: 0 }, null, { ...ram, used_bytes: ram.total_bytes + 1 }, { ...ram, observed_at_utc: "2026-02-30T12:34:56.789Z" }],
      cpu: [cpu, { ...cpu, busy_ticks: 0, usage_percent: 0 }, null, { ...cpu, usage_percent: 101 }, { ...cpu, interval_ms: 0 }, { ...cpu, total_ticks: 0 }, { ...cpu, usage_percent: 50 }, { ...cpu, source: "/wrong" }],
    };
    window.isTauri = true;
    window.calls = { ram: [], cpu: [] };
    window.__TAURI_INTERNALS__ = { invoke: async (cmd, args) => {
      const key = { read_ram: "ram", read_cpu: "cpu" }[cmd];
      if (!key) throw new Error("Неизвестная команда");
      const n = window.calls[key].push([cmd, args]);
      if (key === "cpu" && n === 1) await new Promise(resolve => { window.finishCpu = resolve; });
      if (n > replies[key].length) throw new Error("IPC недоступен");
      return replies[key][n - 1];
    } };
  });
  try {
    await page.goto(origin);
    const card = kind => page.locator(`article.metric.${kind}`);
    const button = kind => card(kind).getByRole("button");
    const reading = async (name, kind) => {
      await page.getByRole("article", { name, exact: true }).waitFor();
      assert.equal(await card(kind).locator("time").getAttribute("datetime"), "2026-11-20T12:34:56.789Z");
    };
    await reading("Занято RAM 1,5 ГиБ из 8 ГиБ /proc/meminfo 2026-11-20 12:34:56 UTC", "ram");
    assert.equal(await button("cpu").isDisabled(), true, "ожидание CPU не задерживает RAM");
    assert.equal(await card("cpu").getAttribute("aria-busy"), "true");
    const labelledValue = await card("cpu").locator("#cpu-value").elementHandle();
    await page.evaluate(() => window.finishCpu());
    await reading("Загрузка CPU 37,5 % за 637 мс /proc/stat 2026-11-20 12:34:56 UTC", "cpu");
    assert.equal(await labelledValue.evaluate(el => el === document.getElementById("cpu-value")), true, "цель aria-labelledby сохранилась после асинхронного чтения");
    const footer = page.locator("footer");
    assert.match(await footer.innerText(), /cpu 37,5 %.*ram 1,5 ГиБ/s);
    assert.equal(await page.locator(".metric .no-data").count(), 2);
    assert.equal(await page.getByText("Нет истории измерений", { exact: true }).count(), 1);
    for (const width of [1281, 800, 640]) {
      await page.setViewportSize({ width, height: 900 });
      for (const kind of ["ram", "cpu"]) assert.ok(await card(kind).evaluate(el => el.scrollWidth <= el.clientWidth && el.scrollHeight <= el.clientHeight), `${kind}: ${width}px`);
    }
    for (const [kind, zero, calls] of [["cpu", "0 %", 9], ["ram", "0 Б", 6]]) {
      await button(kind).click();
      await card(kind).getByText(zero, { exact: true }).waitFor();
      assert.ok((await footer.innerText()).includes(`${kind} ${zero}`));
      for (let n = 3; n <= calls; n++) {
        await button(kind).click();
        await page.waitForFunction(([k, i]) => window.calls[k].length === i && document.querySelector(`.metric.${k}`).getAttribute("aria-busy") === "false", [kind, n]);
        assert.equal(await card(kind).locator(".no-data").innerText(), "нет данных");
        assert.equal(await card(kind).locator(".metric-value").innerText(), "—");
        assert.equal(await card(kind).locator("time").count(), 0);
        const expected = n === 3 ? "Нет достоверного замера" : n === calls ? "Канал недоступен" : "Некорректный ответ";
        assert.ok((await card(kind).locator(".metric-sub").innerText()).includes(expected));
        assert.ok((await footer.innerText()).includes(`${kind} —`));
      }
      if (kind === "cpu") assert.match(await footer.innerText(), /ram 1,5 ГиБ/, "отказ CPU не стирает RAM");
      assert.deepEqual(await page.evaluate(k => window.calls[k], kind), Array.from({ length: calls }, () => [`read_${kind}`, {}]));
    }
    assert.deepEqual(errors, []);
    console.log("CPU/RAM: источник, UTC, единицы, фактический интервал CPU, ноль, отказ и независимое обновление карточек: ОК");
  } finally { await page.close(); }
}
