import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export default function Zapret() {
  const [log, setLog] = useState("Статус Zapret2...\n");
  const [dry, setDry] = useState(true);

  const run = async (cmd: string, args: Record<string, unknown> = {}) => {
    setLog((l) => l + `\n$ ${cmd}\n`);
    try {
      const out = await invoke<string>(cmd, args);
      setLog((l) => l + out + "\n");
    } catch (e) {
      setLog((l) => l + `Ошибка: ${e}\n`);
    }
  };

  return (
    <div>
      <h1 style={{ fontSize: 20, fontWeight: 700, color: "white", marginBottom: 4 }}>Zapret2 + DNS</h1>
      <p style={{ color: "#6b7280", marginBottom: 12 }}>DPI-обход для RU + лучшие DNS.</p>
      <div className="log">{log}</div>
      <div style={{ display: "flex", gap: 8, marginTop: 12, flexWrap: "wrap" }}>
        <button className="btn" onClick={() => run("zapret_status")}>📊 Статус</button>
        <button className="btn" onClick={() => run("zapret_action", { action: "restart" })}>▶️ Перезапуск</button>
        <button className="btn" onClick={() => run("zapret_action", { action: "logs" })}>📜 Логи</button>
        <button className="btn" onClick={() => run("optimize_dns", { dryRun: dry })}>🌐 DNS</button>
      </div>
      <label style={{ display: "flex", gap: 8, alignItems: "center", marginTop: 12 }}>
        <input type="checkbox" checked={dry} onChange={(e) => setDry(e.target.checked)} />
        Dry-run
      </label>
    </div>
  );
}
