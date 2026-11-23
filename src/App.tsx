import { useEffect, useState } from "react";
import Dashboard from "./components/Dashboard";
import Optimizer from "./components/Optimizer";
import Zapret from "./components/Zapret";
import Chat from "./components/Chat";
import Settings from "./components/Settings";
import Avatar from "./components/Avatar";

const PAGES = [
  { id: "dashboard", label: "📊  Dashboard" },
  { id: "cleanup", label: "🧹  CleanUp" },
  { id: "mirrors", label: "🌐  Mirrors & DNS" },
  { id: "chat", label: "💬  Чат / Голос" },
  { id: "settings", label: "⚙️  Settings & Tray" },
];

export default function App() {
  const [page, setPage] = useState("dashboard");
  const [isAvatarWindow, setIsAvatarWindow] = useState(false);

  // Tauri: avatar-окно (label="avatar") показывает ТОЛЬКО модель, без сайдбара
  useEffect(() => {
    (async () => {
      try {
        const { getCurrentWindow } = await import("@tauri-apps/api/window");
        const label = getCurrentWindow().label;
        if (label === "avatar") {
          setIsAvatarWindow(true);
          document.documentElement.classList.add("avatar-window");
          document.body.classList.add("avatar-window");
        }
      } catch {
        // dev в браузере (без Tauri) — не avatar
      }
    })();
  }, []);

  if (isAvatarWindow) {
    return <Avatar />;
  }

  return (
    <div className="layout">
      <aside className="sidebar">
        <div className="logo">◈ MeshPilot</div>
        <div className="ver">v0.2.0 · CachyOS · Tauri</div>
        <div className="section">СИСТЕМА · MVP</div>
        {PAGES.map((p) => (
          <button key={p.id} className={`nav-btn ${page === p.id ? "active" : ""}`} onClick={() => setPage(p.id)}>
            {p.label}
          </button>
        ))}
        <div className="section">ДОРОЖНАЯ КАРТА</div>
        <button className="nav-btn">📦 Sing-box / WG <span className="phase">Phase 2</span></button>
        <button className="nav-btn">🔧 Kernel & Tuning <span className="phase">Phase 2</span></button>
        <div className="zc-card">
          <div><b>Zero-Cost Stack</b> <span className="free">Free</span></div>
          <div style={{ fontSize: 11, color: "#6b7280", marginTop: 4 }}>GitHub Releases + Dropbox Sync</div>
        </div>
        <div style={{ fontSize: 11, color: "#6b7280", marginTop: 8 }}>● готов · Голос: Джарвис</div>
      </aside>
      <main className="main">
        <div className="topbar">
          <div>
            <h1>{PAGES.find((p) => p.id === page)?.label.replace(/^\S+\s+/, "") || "Dashboard"}</h1>
            <div className="sub">Живые метрики CachyOS в реальном времени</div>
          </div>
          <span style={{ flex: 1 }} />
          <span className="pill pill-green">● CachyOS online</span>
          <span className="pill pill-gray">Tauri · Rust</span>
        </div>
        <div className="content">
          {page === "dashboard" && <Dashboard />}
          {page === "cleanup" && <Optimizer />}
          {page === "mirrors" && <Zapret />}
          {page === "chat" && <Chat />}
          {page === "settings" && <Settings />}
        </div>
      </main>
      <Avatar />
    </div>
  );
}
