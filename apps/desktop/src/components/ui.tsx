import { useEffect, type ReactNode, type MouseEvent } from 'react';

export function Card({ title, hint, action, children, className = '' }: {
  title?: string;
  hint?: string;
  action?: ReactNode;
  children: ReactNode;
  className?: string;
}) {
  return (
    <section className={`card ${className}`} aria-label={title}>
      {(title || action) && (
        <div className="card-title">
          <span>{title}{hint && <span className="hint"> · {hint}</span>}</span>
          {action && <span>{action}</span>}
        </div>
      )}
      {children}
    </section>
  );
}

export function Switch({ checked, onChange, label, id }: {
  checked: boolean;
  onChange: (v: boolean) => void;
  label: string;
  id?: string;
}) {
  return (
    <button
      type="button"
      role="switch"
      id={id}
      aria-checked={checked}
      aria-label={label}
      className="switch"
      onClick={() => onChange(!checked)}
    />
  );
}

export function Modal({ title, children, onClose, actions, width }: {
  title: string;
  children: ReactNode;
  onClose: () => void;
  actions?: ReactNode;
  width?: number;
}) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [onClose]);

  const stop = (e: MouseEvent) => e.stopPropagation();

  return (
    <div className="modal-backdrop" role="dialog" aria-modal="true" aria-label={title} onClick={onClose}>
      <div className="modal" style={width ? { maxWidth: width } : undefined} onClick={stop}>
        <h2>{title}</h2>
        <div>{children}</div>
        {actions && <div className="modal-actions">{actions}</div>}
      </div>
    </div>
  );
}

export function Spinner({ label = 'Загрузка…' }: { label?: string }) {
  return (
    <div className="empty-state" role="status" aria-live="polite">
      <div className="skeleton" style={{ height: 18, width: 180, margin: '0 auto 10px' }} />
      <div className="skeleton" style={{ height: 12, width: 260, margin: '0 auto' }} />
      <p style={{ marginTop: 12 }}>{label}</p>
    </div>
  );
}

export function EmptyState({ title }: { title: string }) {
  return <div className="empty-state">{title}</div>;
}

export type BadgeKind = 'safe' | 'caution' | 'risky' | 'info' | 'muted';

export function Badge({ kind, children }: { kind: BadgeKind; children: ReactNode }) {
  return <span className={`badge ${kind}`}>{children}</span>;
}

export function Divider() {
  return <div className="divider" role="separator" />;
}
