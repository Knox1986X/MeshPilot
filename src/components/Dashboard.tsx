import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface Metrics {
  cpu_percent: number;
  cpu_cores: number[];
  cpu_count: number;
  ram_used_gb: number;
  ram_total_gb: number;
  ram_percent: number;
  disk_used_gb: number;
  disk_total_gb: number;
  disk_percent: number;
  net_rx_mb: number;
  net_tx_mb: number;
  hostname: string;
  kernel: string;
}

export default function Dashboard() {
  const [m, setM] = useState<Metrics | null>(null);
  const [cpuHist, setCpuHist] = useState<number[]>(Array(60).fill(0));
  const [ramHist, setRamHist] = useState<number[]>(Array(60).fill(0));
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const donutRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const tick = async () => {
      try {
        const data = await invoke<Metrics>("get_metrics");
        setM(data);
        setCpuHist((h) => [...h.slice(1), data.cpu_percent]);
        setRamHist((h) => [...h.slice(1), data.ram_percent]);
      } catch {}
    };
    tick();
    const id = setInterval(tick, 1000);
    return () => clearInterval(id);
  }, []);

  // линейный график
  useEffect(() => {
    const c = canvasRef.current;
    if (!c) return;
    const ctx = c.getContext("2d");
    if (!ctx) return;
    const w = (c.width = c.offsetWidth * 2);
    const h = (c.height = c.offsetHeight * 2);
    ctx.clearRect(0, 0, w, h);
    ctx.strokeStyle = "#1e293b";
    ctx.fillStyle = "#64748b";
    ctx.font = "16px Inter";
    for (const pct of [0, 25, 50, 75, 100]) {
      const y = h - (h * pct) / 100;
      ctx.beginPath(); ctx.moveTo(60, y); ctx.lineTo(w, y); ctx.stroke();
      ctx.fillText(`${pct}%`, 4, y + 5);
    }
    const draw = (data: number[], color: string) => {
      ctx.strokeStyle = color; ctx.lineWidth = 3; ctx.beginPath();
      data.forEach((v, i) => {
        const x = 60 + ((w - 60) * i) / (data.length - 1);
        const y = h - (h * Math.max(0, Math.min(100, v))) / 100;
        i === 0 ? ctx.moveTo(x, y) : ctx.lineTo(x, y);
      });
      ctx.stroke();
    };
    draw(cpuHist, "#8b7cf6");
    draw(ramHist, "#34d399");
  }, [cpuHist, ramHist]);

  // донат диска
  useEffect(() => {
    const c = donutRef.current;
    if (!c || !m) return;
    const ctx = c.getContext("2d");
    if (!ctx) return;
    const w = (c.width = 200);
    const h = (c.height = 200);
    ctx.clearRect(0, 0, w, h);
    const cx = w / 2, cy = h / 2, r = 80;
    const parts = [
      { label: "/", pct: m.disk_percent, color: "#8b7cf6" },
      { label: "/boot", pct: 10, color: "#34d399" },
      { label: "/srv", pct: 5, color: "#f59e0b" },
      { label: "/root", pct: 5, color: "#38bdf8" },
      { label: "/home", pct: 5, color: "#f472b6" },
    ];
    let start = -Math.PI / 2;
    for (const p of parts) {
      const sweep = (p.pct / 100) * Math.PI * 2;
      ctx.strokeStyle = p.color; ctx.lineWidth = 22;
      ctx.beginPath(); ctx.arc(cx, cy, r, start, start + sweep); ctx.stroke();
      start += sweep;
    }
  }, [m]);

  if (!m) return <div>Загрузка метрик...</div>;

  return (
    <div>
      <div style={{ display: "flex", gap: 8, marginBottom: 12, alignItems: "center" }}>
        <span className="pill pill-green">● CachyOS online</span>
        <span className="pill pill-gray">Kernel: {m.kernel}</span>
        <span className="pill pill-green">● Snapshot: btrfs ready</span>
        <span style={{ flex: 1 }} />
        <button className="btn btn-primary">✨ Быстрая очистка</button>
      </div>

      <div className="cards">
        <div className="card">
          <div className="label">CPU Load · {m.cpu_count} потоков</div>
          <div className="value">{m.cpu_percent.toFixed(1)}%</div>
          <div className="bar"><div style={{ width: `${m.cpu_percent}%` }} /></div>
        </div>
        <div className="card">
          <div className="label">RAM Memory</div>
          <div className="value">{m.ram_used_gb.toFixed(1)} GB <span style={{ fontSize: 12, color: "#6b7280" }}>/ {m.ram_total_gb.toFixed(1)} GB</span></div>
          <div className="bar green"><div style={{ width: `${m.ram_percent}%` }} /></div>
        </div>
        <div className="card">
          <div className="label">Disk Storage</div>
          <div className="value">{m.disk_used_gb.toFixed(1)} GB <span style={{ fontSize: 12, color: "#6b7280" }}>/ {m.disk_total_gb.toFixed(1)} GB</span></div>
          <div className="bar"><div style={{ width: `${m.disk_percent}%` }} /></div>
        </div>
        <div className="card">
          <div className="label">Network I/O · Live</div>
          <div className="value">{m.net_rx_mb.toFixed(1)} MB <span style={{ fontSize: 12, color: "#6b7280" }}>RX</span></div>
          <div style={{ fontSize: 12, color: "#6b7280" }}>TX {m.net_tx_mb.toFixed(1)} MB</div>
        </div>
      </div>

      <div className="grid-2">
        <div className="card">
          <div style={{ fontWeight: 700, fontSize: 14 }}>CPU & RAM Telemetry (60s)</div>
          <div style={{ fontSize: 12, color: "#6b7280" }}>Скользящее окно 60 секунд · шаг 1 сек</div>
          <canvas ref={canvasRef} className="chart" />
        </div>
        <div className="card">
          <div style={{ fontWeight: 700, fontSize: 14 }}>Распределение диска</div>
          <div style={{ fontSize: 12, color: "#6b7280" }}>Занятое место по mountpoints</div>
          <canvas ref={donutRef} style={{ width: 200, height: 200, display: "block", margin: "10px auto" }} />
        </div>
      </div>

      <div className="grid-2b">
        <div className="card">
          <div style={{ fontWeight: 700, fontSize: 14 }}>Нагрузка по ядрам CPU</div>
          <div style={{ display: "grid", gridTemplateColumns: "repeat(4, 1fr)", gap: 8, marginTop: 10 }}>
            {m.cpu_cores.map((v, i) => (
              <div key={i} style={{ background: "#0b1220", border: "1px solid #1f2937", borderRadius: 8, padding: 8 }}>
                <div style={{ display: "flex", justifyContent: "space-between", fontSize: 11 }}>
                  <span>CPU{i}</span><span>{v.toFixed(0)}%</span>
                </div>
                <div className="bar" style={{ marginTop: 6 }}><div style={{ width: `${v}%` }} /></div>
              </div>
            ))}
          </div>
        </div>
        <div className="card">
          <div style={{ fontWeight: 700, fontSize: 14 }}>Среда CachyOS</div>
          <div style={{ marginTop: 10, display: "grid", gap: 8, fontSize: 12 }}>
            <div style={{ display: "flex", justifyContent: "space-between" }}><span style={{ color: "#6b7280" }}>Hostname</span><span>{m.hostname}</span></div>
            <div style={{ display: "flex", justifyContent: "space-between" }}><span style={{ color: "#6b7280" }}>Kernel</span><span>{m.kernel}</span></div>
            <div style={{ display: "flex", justifyContent: "space-between" }}><span style={{ color: "#6b7280" }}>Snapshot backend</span><span>btrfs</span></div>
            <div style={{ display: "flex", justifyContent: "space-between" }}><span style={{ color: "#6b7280" }}>Dropbox</span><span>не примонтирован</span></div>
          </div>
        </div>
      </div>
    </div>
  );
}
