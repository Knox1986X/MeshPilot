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
    const disk = { used_bytes: 1.25 * 2 ** 30, total_bytes: 4 * 2 ** 30, available_bytes: 2 * 2 ** 30, source: "statvfs(/)", observed_at_utc };
    const uptime = { uptime_seconds: 11931, source: "/proc/uptime", observed_at_utc };
    const kernel = { version: "6.12.9-cachyos1-1", source: "/proc/version", observed_at_utc };
    const replies = {
      uptime: [uptime, { ...uptime, uptime_seconds: 0 }, null, { ...uptime, source: "/proc/stat" }, { ...uptime, observed_at_utc: "" }, "error"],
      kernel: [kernel, null, { ...kernel, source: "/proc/stat" }, "error"],
      net: [{ source: "/proc/net/dev", observed_at_utc, interval_ms: 500, interfaces: [] }],
      disk: [disk, { ...disk, used_bytes: 0, available_bytes: 0 }, null,
        { ...disk, available_bytes: disk.total_bytes - disk.used_bytes + 1 },
        { ...disk, available_bytes: -1 }, { ...disk, used_bytes: 0.5 }, { ...disk, total_bytes: 2 ** 53 },
        { ...disk, source: "statvfs(/home)" }, { ...disk, observed_at_utc: "" }, { ...disk, available_bytes: undefined }],
      ram: [ram, { ...ram, used_bytes: 0 }, null, { ...ram, used_bytes: ram.total_bytes + 1 }, { ...ram, observed_at_utc: "2026-02-30T12:34:56.789Z" }],
      cpu: [cpu, { ...cpu, busy_ticks: 0, usage_percent: 0 }, null, { ...cpu, usage_percent: 101 }, { ...cpu, interval_ms: 0 }, { ...cpu, total_ticks: 0 }, { ...cpu, usage_percent: 50 }, { ...cpu, source: "/wrong" }],
    };
    window.isTauri = true;
    window.calls = { ram: [], cpu: [], disk: [], net: [], uptime: [], kernel: [] };
    window.__TAURI_INTERNALS__ = { invoke: async (cmd, args) => {
      const key = { read_ram: "ram", read_cpu: "cpu", read_root_fs: "disk", read_network_io: "net", read_uptime: "uptime", read_kernel: "kernel" }[cmd];
      if (!key) throw new Error("Неизвестная команда");
      const n = window.calls[key].push([cmd, args]);
      if (key === "cpu" && n === 1) await new Promise(resolve => { window.finishCpu = resolve; });
      if (n > replies[key].length) throw new Error("IPC недоступен");
      const reply = replies[key][n - 1];
      if (reply === "error") throw new Error("обрыв IPC");
      return reply;
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
    const diskLabel = "Занято на / 1,25 ГиБ из 4 ГиБ Доступно пользователю: 2 ГиБ statvfs(/) 2026-11-20 12:34:56 UTC";
    await reading(diskLabel, "disk");
    const legend = page.locator(".disk-legend dd");
    assert.equal(await page.locator(".donut-panel time").getAttribute("datetime"), "2026-11-20T12:34:56.789Z");
    assert.deepEqual(await legend.allTextContents(), ["1,25 ГиБ", "2 ГиБ", "768 МиБ", "4 ГиБ"]);
    assert.equal(await page.locator(".donut-used").getAttribute("stroke-dasharray"), "31.25 100");
    assert.equal(await page.locator(".donut-available").getAttribute("stroke-dasharray"), "50 100");
    assert.equal(await button("cpu").isDisabled(), true, "ожидание CPU не задерживает RAM и ФС");
    assert.equal(await card("cpu").getAttribute("aria-busy"), "true");
    const labelledValue = await card("cpu").locator("#cpu-value").elementHandle();
    await page.evaluate(() => window.finishCpu());
    await reading("Загрузка CPU 37,5 % за 637 мс /proc/stat 2026-11-20 12:34:56 UTC", "cpu");
    assert.equal(await labelledValue.evaluate(el => el === document.getElementById("cpu-value")), true, "цель aria-labelledby сохранилась после асинхронного чтения");
    const footer = page.locator("footer");
    assert.match(await footer.innerText(), /cpu 37,5 %.*ram 1,5 ГиБ/s);
    assert.equal(await page.locator(".metric .no-data").count(), 0);
    assert.equal(await page.getByText("Нет истории измерений", { exact: true }).count(), 1);
    for (const width of [1281, 1061, 800, 640]) {
      await page.setViewportSize({ width, height: 900 });
      assert.ok(await page.locator(".donut-panel").evaluate(el => {
        const box = s => el.querySelector(s).getBoundingClientRect();
        return el.scrollWidth <= el.clientWidth && box(".panel-heading").bottom <= box(".disk-legend").top && box(".disk-legend").bottom <= box(".chart-foot").top;
      }), `легенда ФС и подписи: ${width}px`);
      for (const kind of ["ram", "cpu", "disk"]) assert.ok(await card(kind).evaluate(el => el.scrollWidth <= el.clientWidth && el.scrollHeight <= el.clientHeight), `${kind}: ${width}px`);
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
    // Отказы и ручные чтения RAM/CPU не перезапрашивали и не стирали снимок ФС.
    await reading(diskLabel, "disk");
    assert.equal(await page.evaluate(() => window.calls.disk.length), 1);
    await button("disk").click();
    await reading("Занято на / 0 Б из 4 ГиБ Доступно пользователю: 0 Б statvfs(/) 2026-11-20 12:34:56 UTC", "disk");
    assert.deepEqual(await legend.allTextContents(), ["0 Б", "0 Б", "4 ГиБ", "4 ГиБ"]);
    for (let n = 3; n <= 11; n++) {
      await button("disk").click();
      const expected = n === 3 ? "Нет достоверного замера" : n === 11 ? "Канал недоступен" : "Некорректный ответ";
      await card("disk").getByText(`${expected}: statvfs(/)`, { exact: true }).waitFor();
      assert.equal(await card("disk").locator("time").count(), 0);
      assert.equal(await card("disk").locator("#disk-value").innerText(), "—");
      assert.equal(await card("disk").locator("#disk-available").innerText(), "");
      assert.deepEqual(await legend.allTextContents(), ["—", "—", "—", "—"]);
      assert.equal(await page.locator(".donut-used, .donut-available, .donut-panel time").count(), 0);
    }
    assert.deepEqual(await page.evaluate(() => window.calls.disk), Array.from({ length: 11 }, () => ["read_root_fs", {}]));
    assert.deepEqual(await page.evaluate(() => window.calls.net), [["read_network_io", {}]]);
    // Панель «Среда»: по одному чтению аптайма и ядра при открытии, ручные чтения по кнопкам строк.
    assert.equal(await page.locator("#uptime-value").innerText(), "3 ч 18 м 51 с");
    assert.equal(await page.locator("#uptime-row time").getAttribute("datetime"), "2026-11-20T12:34:56.789Z");
    assert.match((await page.locator("#uptime-meta").innerText()).replace(/\s+/g, " "), /^\/proc\/uptime 2026-11-20 12:34:56 UTC$/);
    const uptimeButton = page.getByRole("button", { name: "Обновить время работы" });
    for (let n = 2; n <= 6; n++) {
      await uptimeButton.click();
      await page.waitForFunction(n => window.calls.uptime.length === n, n);
      const expected = n === 2 ? "0 с" : null;
      if (expected) {
        assert.equal(await page.locator("#uptime-value").innerText(), expected);
        assert.equal(await page.locator("#uptime-row time").getAttribute("datetime"), "2026-11-20T12:34:56.789Z");
      } else {
        assert.equal(await page.locator("#uptime-value").innerText(), "—");
        assert.equal(await page.locator("#uptime-row time").count(), 0);
        const why = n === 3 ? "Нет достоверного замера" : n === 6 ? "Канал недоступен" : "Некорректный ответ";
        assert.equal(await page.locator("#uptime-meta").innerText(), `${why}: /proc/uptime`);
      }
    }
    assert.deepEqual(await page.evaluate(() => window.calls.uptime), Array.from({ length: 6 }, () => ["read_uptime", {}]));
    assert.equal(await page.locator("#kernel-value").innerText(), "6.12.9-cachyos1-1");
    assert.match(await page.locator(".pill.blue").innerText(), /Kernel: 6\.12\.9-cachyos1-1/);
    assert.equal(await page.locator("#kernel-row time").getAttribute("datetime"), "2026-11-20T12:34:56.789Z");
    assert.match((await page.locator("#kernel-meta").innerText()).replace(/\s+/g, " "), /^\/proc\/version 2026-11-20 12:34:56 UTC$/);
    const kernelButton = page.getByRole("button", { name: "Обновить версию ядра" });
    for (let n = 2; n <= 4; n++) {
      await kernelButton.click();
      await page.waitForFunction(n => window.calls.kernel.length === n, n);
      assert.equal(await page.locator("#kernel-value").innerText(), "—");
      assert.equal(await page.locator("#kernel-row time").count(), 0);
      const why = n === 2 ? "Нет достоверного замера" : n === 3 ? "Некорректный ответ" : "Канал недоступен";
      assert.equal(await page.locator("#kernel-meta").innerText(), `${why}: /proc/version`);
      assert.match(await page.locator(".pill.blue").innerText(), /Kernel: —/);
    }
    assert.deepEqual(await page.evaluate(() => window.calls.kernel), Array.from({ length: 4 }, () => ["read_kernel", {}]));
    // Отказы ядра не стирали строку аптайма и наоборот.
    assert.equal(await page.locator("#uptime-value").innerText(), "—");
    assert.deepEqual(errors, []);
    console.log("CPU/RAM/ФС /, аптайм и ядро: источник, UTC, единицы, интервал CPU, ноль, отказ и независимое обновление: ОК");
  } finally { await page.close(); }
}
