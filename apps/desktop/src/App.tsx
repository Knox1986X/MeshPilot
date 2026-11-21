import { useCallback, useEffect, useRef, useState } from 'react';
import { Sidebar, type SectionId } from './components/Sidebar';
import { Topbar } from './components/Topbar';
import { Statusbar } from './components/Statusbar';
import { Overview } from './pages/Overview';
import { Jarvis } from './pages/Jarvis';
import { Cleanup } from './pages/Cleanup';
import { Network } from './pages/Network';
import { Permissions } from './pages/Permissions';
import { SettingsPage } from './pages/Settings';
import { api } from './lib/api';
import type { Settings, Snapshot } from './types';

interface Toast {
  id: number;
  kind: 'ok' | 'err' | 'warn';
  text: string;
}

export default function App() {
  const [section, setSection] = useState<SectionId>('overview');
  const [settings, setSettings] = useState<Settings | null>(null);
  const [settingsError, setSettingsError] = useState<string | null>(null);
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [snapshotLoading, setSnapshotLoading] = useState(true);
  const [apiConnected, setApiConnected] = useState<boolean | null>(null);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const toastId = useRef(0);

  const toast = useCallback((kind: Toast['kind'], text: string) => {
    toastId.current += 1;
    const id = toastId.current;
    setToasts((t) => [...t, { id, kind, text }]);
    window.setTimeout(() => setToasts((t) => t.filter((x) => x.id !== id)), 5000);
  }, []);

  // Настройки
  useEffect(() => {
    let cancelled = false;
    api.settingsGet()
      .then((s) => { if (!cancelled) setSettings(s); })
      .catch((e) => { if (!cancelled) setSettingsError(e instanceof Error ? e.message : String(e)); });
    return () => { cancelled = true; };
  }, []);

  const applySettingsPatch = useCallback(async (patch: Partial<Settings>) => {
    const next = await api.settingsUpdate(patch);
    setSettings(next);
  }, []);

  // Снимок системы: шаг ~1 с, пауза когда окно скрыто
  useEffect(() => {
    let cancelled = false;
    let timer: number | undefined;
    const tick = async () => {
      if (document.visibilityState === 'hidden') return;
      try {
        const s = await api.getSnapshot();
        if (!cancelled) setSnapshot(s);
      } catch {
        /* оставляем прежний снимок; статусбар покажет «нет данных» только при полном отсутствии */
      } finally {
        if (!cancelled) setSnapshotLoading(false);
      }
    };
    void tick();
    timer = window.setInterval(() => void tick(), 1000);
    return () => {
      cancelled = true;
      if (timer !== undefined) window.clearInterval(timer);
    };
  }, []);

  // Горячие клавиши Ctrl+1..6 для разделов
  useEffect(() => {
    const order: SectionId[] = ['overview', 'jarvis', 'cleanup', 'network', 'permissions', 'settings'];
    const onKey = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null;
      const tag = target?.tagName?.toLowerCase();
      if (tag === 'input' || tag === 'textarea' || tag === 'select' || target?.isContentEditable) return;
      if (!e.ctrlKey || e.shiftKey || e.altKey || e.metaKey) return;
      const n = Number(e.key);
      if (n >= 1 && n <= order.length) {
        e.preventDefault();
        setSection(order[n - 1]);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);

  const demoMode = api.isDemo;

  if (settingsError) {
    return (
      <div style={{ padding: 40, color: 'var(--err)', fontFamily: 'var(--sans)' }}>
        <h1>Настройки не загрузились</h1>
        <p>{settingsError}</p>
        <button className="btn primary" onClick={() => window.location.reload()}>Повторить</button>
      </div>
    );
  }

  return (
    <div className="app-shell">
      <Topbar section={section} snapshot={snapshot} onQuickCleanup={() => setSection('cleanup')} />
      <Sidebar active={section} onSelect={setSection} demoMode={demoMode} />
      <main className="main" id="main">
        {!settings ? (
          <div className="empty-state" style={{ marginTop: 60 }}>Загружаем настройки…</div>
        ) : section === 'overview' ? (
          <Overview snapshot={snapshot} loading={snapshotLoading} onNavigate={setSection} />
        ) : section === 'jarvis' ? (
          <Jarvis
            settings={settings}
            onOpenSettings={() => setSection('settings')}
            onNavigate={setSection}
            onToast={toast}
            onApiStateChange={setApiConnected}
            onSettingsChange={applySettingsPatch}
          />
        ) : section === 'cleanup' ? (
          <Cleanup onToast={toast} />
        ) : section === 'network' ? (
          <Network onToast={toast} />
        ) : section === 'permissions' ? (
          <Permissions />
        ) : (
          <SettingsPage settings={settings} onSettingsChange={applySettingsPatch} onToast={toast} />
        )}
      </main>
      <Statusbar snapshot={snapshot} loading={snapshotLoading} apiConnected={apiConnected} demoMode={demoMode} />

      <div className="toast-wrap" aria-live="polite">
        {toasts.map((t) => (
          <div key={t.id} className={`toast ${t.kind}`} role="status">{t.text}</div>
        ))}
      </div>
    </div>
  );
}
