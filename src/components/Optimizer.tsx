import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export default function Optimizer() {
  const [log, setLog] = useState("Лог оптимизатора...\n");
  const [dry, setDry] = useState(true);
  const [busy, setBusy] = useState(false);

  const run = async (cmd: string, args: Record<string, unknown>) => {
    setBusy(true);
    setLog((l) => l + `\n$ ${cmd} ${JSON.stringify(args)}\n`);
    try {
      const out = await invoke<string>(cmd, args);
      setLog((l) => l + out + "\n");
    } catch (e) {
      setLog((l) => l + `Ошибка: ${e}\n`);
    }
    setBusy(false);
  };

  return (
    <div>
      <h1 style={{ fontSize: 20, fontWeight: 700, color: "white", marginBottom: 4 }}>Оптимизатор CachyOS</h1>
      <p style={{ color: "#6b7280", marginBottom: 12 }}>Чистка, зеркала, система. Dry-run по умолчанию безопасен.</p>
      <label style={{ display: "flex", gap: 8, alignItems: "center", marginBottom: 12 }}>
        <input type="checkbox" checked={dry} onChange={(e) => setDry(e.target.checked)} />
        Dry-run (без удаления)
      </label>
      <div className="log">{log}</div>
      <div style={{ display: "flex", gap: 8, marginTop: 12, flexWrap: "wrap" }}>
        <button className="btn btn-primary" disabled={busy} onClick={() => run("clean_all", { dryRun: dry })}>🧹 Полная чистка</button>
        <button className="btn" disabled={busy} onClick={() => run("clean_pacman_cache", { dryRun: dry })}>📦 Кэш pacman</button>
        <button className="btn" disabled={busy} onClick={() => run("remove_orphans", { dryRun: dry })}>🗑️ Orphan-пакеты</button>
        <button className="btn" disabled={busy} onClick={() => run("optimize_mirrors", { dryRun: dry })}>🪞 Зеркала</button>
        <button className="btn" disabled={busy} onClick={() => run("system_health", {})}>❤️ Health-check</button>
      </div>
    </div>
  );
}
