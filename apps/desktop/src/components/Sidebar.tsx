import type { ReactNode } from 'react';

export type SectionId = 'overview' | 'jarvis' | 'cleanup' | 'network' | 'permissions' | 'settings';

export const SECTIONS: Array<{ id: SectionId; label: string; icon: ReactNode }> = [
  {
    id: 'overview',
    label: 'Обзор',
    icon: (
      <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <rect x="3" y="3" width="7" height="7" rx="1.5" /><rect x="14" y="3" width="7" height="7" rx="1.5" /><rect x="3" y="14" width="7" height="7" rx="1.5" /><rect x="14" y="14" width="7" height="7" rx="1.5" />
      </svg>
    ),
  },
  {
    id: 'jarvis',
    label: 'Джарвис',
    icon: (
      <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <path d="M12 3a4 4 0 0 1 4 4v5a4 4 0 0 1-8 0V7a4 4 0 0 1 4-4z" /><path d="M6 11a6 6 0 0 0 12 0" /><path d="M12 17v4" /><path d="M9 21h6" />
      </svg>
    ),
  },
  {
    id: 'cleanup',
    label: 'Очистка',
    icon: (
      <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <path d="M4 17l6-6" /><path d="M9.5 4.5l10 10L15 19H5l-4.5-4.5 9-10z" transform="translate(1.5 0)" />
      </svg>
    ),
  },
  {
    id: 'network',
    label: 'Зеркала и DNS',
    icon: (
      <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <circle cx="12" cy="12" r="9" /><path d="M3 12h18" /><path d="M12 3a15 15 0 0 1 0 18" /><path d="M12 3a15 15 0 0 0 0 18" />
      </svg>
    ),
  },
  {
    id: 'permissions',
    label: 'Разрешения',
    icon: (
      <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <path d="M12 3l7 3v5c0 4.5-3 8.5-7 10-4-1.5-7-5.5-7-10V6l7-3z" /><path d="M9.5 12l2 2 3.5-4" />
      </svg>
    ),
  },
  {
    id: 'settings',
    label: 'Настройки',
    icon: (
      <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <circle cx="12" cy="12" r="3.2" /><path d="M19.4 15a1.7 1.7 0 0 0 .34 1.87l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.7 1.7 0 0 0-1.87-.34 1.7 1.7 0 0 0-1 1.55V21a2 2 0 1 1-4 0v-.09a1.7 1.7 0 0 0-1-1.55 1.7 1.7 0 0 0-1.87.34l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.7 1.7 0 0 0 .34-1.87 1.7 1.7 0 0 0-1.55-1H3a2 2 0 1 1 0-4h.09a1.7 1.7 0 0 0 1.55-1 1.7 1.7 0 0 0-.34-1.87l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.7 1.7 0 0 0 1.87.34h0a1.7 1.7 0 0 0 1-1.55V3a2 2 0 1 1 4 0v.09a1.7 1.7 0 0 0 1 1.55h0a1.7 1.7 0 0 0 1.87-.34l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.7 1.7 0 0 0-.34 1.87v0a1.7 1.7 0 0 0 1.55 1H21a2 2 0 1 1 0 4h-.09a1.7 1.7 0 0 0-1.51 1z" />
      </svg>
    ),
  },
];

interface Props {
  active: SectionId;
  onSelect: (id: SectionId) => void;
  demoMode: boolean;
}

export function Sidebar({ active, onSelect, demoMode }: Props) {
  return (
    <nav className="sidebar" aria-label="Разделы MeshPilot">
      {SECTIONS.map((s) => (
        <button
          key={s.id}
          type="button"
          className={`nav-item${active === s.id ? ' active' : ''}`}
          aria-current={active === s.id ? 'page' : undefined}
          onClick={() => onSelect(s.id)}
        >
          {s.icon}
          <span className="label">{s.label}</span>
        </button>
      ))}
      <div className="foot">
        {demoMode && <span className="demo">ДЕМО</span>}
        <div>MeshPilot · Джарвис</div>
        <div>Приватность по умолчанию.<br />Телеметрии нет.</div>
      </div>
    </nav>
  );
}
