import assert from "node:assert/strict";

export async function testNetworkIo(browser, origin) {
  const page = await browser.newPage();
  const errors = [];
  page.on("pageerror", e => errors.push(e.message));
  page.on("request", r => { if (new URL(r.url()).origin !== origin) errors.push(r.url()); });
  const stamp = "2026-11-20T12:34:56.789Z";
  const rates = (rx, tx) => ({ rx_delta_bytes: rx, tx_delta_bytes: tx, rx_bytes_per_sec: rx*1.6, tx_bytes_per_sec: tx*1.6 });
  const lo = { name: "lo", rates: rates(0, 0) };
  const dev = { name: "veth-1234567890", rates: rates(1280, 640) };
  const sample = { source: "/proc/net/dev", observed_at_utc: stamp, interval_ms: 625, interfaces: [lo, dev] };
  await page.addInitScript(sample => {
    window.isTauri = true;
    window.commands = [];
    window.nextNet = sample;
    window.__TAURI_INTERNALS__ = { invoke: async (cmd, args) => {
      window.commands.push([cmd, args]);
      if (cmd !== "read_network_io") return null;
      if (!window.finishNet) await new Promise(resolve => { window.finishNet = resolve; });
      if (window.nextNet === "error") throw new Error("обрыв IPC");
      return window.nextNet;
    } };
  }, sample);
  try {
    await page.goto(origin);
    const card = page.locator("article.net"), refresh = card.getByRole("button"), select = card.getByRole("combobox");
    const footer = page.locator("footer");
    await page.waitForFunction(() => !!window.finishNet);
    assert.equal(await refresh.isDisabled(), true);
    assert.equal(await page.locator(".metric.cpu").getAttribute("aria-busy"), "false");
    await page.evaluate(() => window.finishNet());
    await card.locator("time").waitFor();
    assert.equal(await select.inputValue(), dev.name, "по умолчанию отдельный нелокальный интерфейс, не сумма");
    assert.match(await card.locator("#net-value").innerText(), /Приём 2 КиБ\/с\s+Передача 1 КиБ\/с/);
    assert.match(await footer.innerText(), /veth-1234567890 ↓ 2 КиБ\/с ↑ 1 КиБ\/с/);
    for (const width of [1281, 800, 640]) {
      await page.setViewportSize({width, height: 900});
      assert.ok(await card.evaluate(el => el.scrollWidth <= el.clientWidth));
      assert.ok(await footer.evaluate(el => el.scrollWidth <= el.clientWidth));
    }
    await select.selectOption("lo");
    assert.match(await footer.innerText(), /lo ↓ 0 Б\/с ↑ 0 Б\/с/);
    const commands = () => page.evaluate(() => window.commands);
    const initial = await commands();
    assert.equal(initial.length, 8); // Одна команда на источник, ноль для выбора интерфейса.
    let refreshes = 0;
    const read = async (reply) => {
      await page.evaluate(reply => { window.nextNet = reply; }, reply);
      await refresh.click();
      await page.waitForFunction(n => window.commands.length === n && document.querySelector(".metric.net").getAttribute("aria-busy") === "false", 8 + ++refreshes);
    };
    await read({...sample, interfaces:[lo]});
    await page.getByRole("article", {name: "Трафик сети lo только локальная петля Приём 0 Б/с Передача 0 Б/с за 625 мс /proc/net/dev 2026-11-20 12:34:56 UTC", exact:true}).waitFor();
    assert.equal(await card.locator("time").getAttribute("datetime"), stamp);
    await read({...sample, interfaces:[{name:"new0", rates:null}]});
    assert.equal(await card.locator("#net-value").innerText(), "Нет данных за интервал");
    assert.match(await footer.innerText(), /new0 ↓ — ↑ —/);
    await read({...sample, interfaces:[]});
    assert.equal(await card.locator("#net-value").innerText(), "Нет интерфейсов");
    assert.match(await footer.innerText(), /сеть ↓ — ↑ —/);
    for (const bad of [null, {...sample, interval_ms: 499}, {...sample, source: "eth0"}, {...sample, observed_at_utc:""},
      {...sample, interfaces:[dev, dev]}, {...sample, interfaces:Array(257).fill(lo)},
      ...[{name:"eth0\n", rates:null}, {...dev,rates:{}}, {...dev,rates:{...dev.rates, rx_bytes_per_sec:0}},
        {...dev,rates:rates(2**53,0)}, {...dev,rates:rates(0,-1)}, {...dev,rates:rates(0.5,0)}].map(i => ({...sample,interfaces:[i]})), "error"]) {
      await read(bad);
      const why = bad === null ? "Нет достоверного замера" : bad === "error" ? "Канал недоступен" : "Некорректный ответ";
      assert.equal(await card.locator("#net-meta").innerText(), `${why}: /proc/net/dev`);
      assert.equal(await card.locator("#net-value").innerText(), "—");
      assert.equal(await card.locator("time").count(), 0);
      assert.match(await footer.innerText(), /сеть ↓ — ↑ —/);
    }
    await read({...sample, interfaces:[dev]});
    assert.match(await footer.innerText(), /veth-1234567890 ↓ 2 КиБ\/с ↑ 1 КиБ\/с/);
    assert.deepEqual((await commands()).slice(initial.length), Array.from({length:refreshes}, () => ["read_network_io", {}]));
    assert.deepEqual(errors, []);
    console.log("RX/TX: выбор без IPC, источник/интервал/UTC, loopback, отсутствие интерфейса, сброс и восстановление: ОК");
  } finally { await page.close(); }
}
