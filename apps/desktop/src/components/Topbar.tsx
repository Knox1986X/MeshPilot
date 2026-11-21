import type { Snapshot } from '../types';
import { formatClock } from '../lib/format';
import { SECTIONS, type SectionId } from './Sidebar';

interface Props {
  section: SectionId;
  snapshot: Snapshot | null;
  onQuickCleanup: () => void;
}

export function Topbar({ section, snapshot, onQuickCleanup }: Props) {
  const sectionLabel = SECTIONS.find((s) => s.id === section)?.label ?? 'Обзор';
  const kernel = snapshot?.kernel ?? '…';
  const os = snapshot?.osName ?? '…';

  return (
    <header className="topbar">
      <div className="logo" aria-label="MeshPilot">
        <span className="dot" aria-hidden="true">M</span>
        MeshPilot
        <span className="version">v0.2.0</span>
      </div>
      <div className="section">Раздел: <b>{sectionLabel}</b></div>
      <div className="spacer" />
      <div className="os-info" title="Фактически определённые ОС и ядро">
        <span>{os}</span>
        <span>·</span>
        <span>ядро {kernel}</span>
        <span>·</span>
        <span>{formatClock(new Date())}</span>
      </div>
      <button type="button" className="icon-btn" title="Быстрый анализ очистки" aria-label="Быстрый анализ очистки" onClick={onQuickCleanup}>
        <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
          <path d="M4 17l6-6" /><path d="M10.5 3.5l10 10-4.5 4.5H6L1.5 13.5 10.5 3.5z" />
        </svg>
      </button>
      <button type="button" className="icon-btn" title="О MeshPilot" aria-label="О приложении" onClick={() => window.alert('MeshPilot 0.2.0 — Джарвис и инструменты обслуживания CachyOS.\nЛокально по умолчанию, телеметрии нет.')}>
        <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
          <circle cx="12" cy="12" r="9" /><path d="M12 11v5" /><path d="M12 8h.01" />
        </svg>
      </button>
    </header>
  );
}
