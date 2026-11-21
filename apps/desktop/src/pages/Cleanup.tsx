import { useCallback, useEffect, useMemo, useState } from 'react';
import type { CleanupApplyResult, CleanupCategory, CleanupReport } from '../types';
import { api } from '../lib/api';
import { formatBytes } from '../lib/format';
import { Badge, Card, Modal, Spinner, type BadgeKind } from '../components/ui';

const RISK_BADGE: Record<CleanupCategory['risk'], { kind: BadgeKind; label: string }> = {
  safe: { kind: 'safe', label: 'безопасно' },
  caution: { kind: 'caution', label: 'внимание' },
  risky: { kind: 'risky', label: 'риск' },
};

interface Props {
  onToast: (kind: 'ok' | 'err' | 'warn', text: string) => void;
}

export function Cleanup({ onToast }: Props) {
  const [report, setReport] = useState<CleanupReport | null>(null);
  const [analyzing, setAnalyzing] = useState(false);
  const [analyzeError, setAnalyzeError] = useState<string | null>(null);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [ackRisky, setAckRisky] = useState(false);
  const [applying, setApplying] = useState(false);
  const [result, setResult] = useState<CleanupApplyResult | null>(null);

  const analyze = useCallback(async () => {
    setAnalyzing(true);
    setAnalyzeError(null);
    try {
      const r = await api.cleanupAnalyze();
      setReport(r);
      setSelected(new Set(r.categories.filter((c) => c.available && c.defaultSelected && c.risk === 'safe').map((c) => c.id)));
    } catch (e) {
      setAnalyzeError(e instanceof Error ? e.message : String(e));
    } finally {
      setAnalyzing(false);
    }
  }, []);

  useEffect(() => {
    void analyze();
  }, [analyze]);

  const toggle = (id: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const selectedCategories = useMemo(
    () => (report?.categories ?? []).filter((c) => selected.has(c.id)),
    [report, selected],
  );
  const selectedBytes = selectedCategories.reduce((sum, c) => sum + (c.estimatedBytes ?? 0), 0);
  const hasOptIn = selectedCategories.some((c) => c.requiresOptIn);
  const needsPrivilege = selectedCategories.some((c) => c.requiresPrivilege);

  const apply = async () => {
    setApplying(true);
    try {
      const r = await api.cleanupApply({
        categoryIds: [...selected],
        dryRun: false,
        confirmRisky: ackRisky,
      });
      setResult(r);
      setConfirmOpen(false);
      setAckRisky(false);
      onToast(r.results.some((x) => !x.ok) ? 'warn' : 'ok', `Освобождено: ${formatBytes(r.freedBytes)}`);
      void analyze();
    } catch (e) {
      onToast('err', e instanceof Error ? e.message : String(e));
    } finally {
      setApplying(false);
    }
  };

  return (
    <div>
      <div className="row spread">
        <div>
          <h1 className="section-title">Очистка</h1>
          <p className="section-sub" style={{ marginBottom: 0 }}>Сначала — анализ. Удаляются только выбранные категории, только кэш. Сессии, пароли и история не трогаются.</p>
        </div>
        <button type="button" className="btn" onClick={() => void analyze()} disabled={analyzing}>Повторить анализ</button>
      </div>

      {analyzing && !report && <Spinner label="Сканируем кэши (размеры каталогов)…" />}
      {analyzeError && <div className="note err" role="alert">Ошибка анализа: {analyzeError}</div>}

      {report && (
        <>
          {report.runningApps.length > 0 && (
            <div className="note warn" style={{ margin: '10px 0' }} role="status">
              Запущены: <b>{report.runningApps.join(', ')}</b>. Их кэши лучше пропустить — закройте приложения и повторите анализ.
            </div>
          )}
          {report.warnings.map((w, i) => <div key={i} className="note warn" style={{ marginBottom: 8 }}>{w}</div>)}

          <Card title="Категории" hint={`оценка ${formatBytes(report.totalEstimatedBytes)}`}
            action={<span className="hint">{selected.size} выбрано · {formatBytes(selectedBytes)}</span>}>
            <table className="table">
              <thead>
                <tr><th style={{ width: 36 }}><span className="sr-only">Выбрать</span></th><th>Категория</th><th>Риск</th><th>Оценка</th><th>Права</th></tr>
              </thead>
              <tbody>
                {report.categories.map((c) => (
                  <tr key={c.id} className={selected.has(c.id) ? 'selected' : ''}>
                    <td>
                      <input
                        type="checkbox"
                        aria-label={`Выбрать ${c.name}`}
                        checked={selected.has(c.id)}
                        disabled={!c.available}
                        onChange={() => toggle(c.id)}
                      />
                    </td>
                    <td>
                      <div style={{ fontWeight: 600 }}>{c.name}</div>
                      <div style={{ color: 'var(--text-faint)', fontSize: 10.5, maxWidth: 520 }}>{c.description}</div>
                      {c.blockedReason && <div style={{ color: 'var(--warn)', fontSize: 10, marginTop: 2 }}>{c.blockedReason}</div>}
                    </td>
                    <td><Badge kind={RISK_BADGE[c.risk].kind}>{RISK_BADGE[c.risk].label}</Badge>{c.requiresOptIn && <span style={{ marginLeft: 4 }}><Badge kind="muted">оптин</Badge></span>}</td>
                    <td className="mono">{c.available ? formatBytes(c.estimatedBytes) : '—'}</td>
                    <td>{c.requiresPrivilege ? <Badge kind="muted">helper</Badge> : <span style={{ color: 'var(--text-faint)' }}>нет</span>}</td>
                  </tr>
                ))}
              </tbody>
            </table>
            <div className="row spread" style={{ marginTop: 12 }}>
              <p className="metric-sub" style={{ margin: 0 }}>
                Итого к освобождению (оценка): <b className="mono">{formatBytes(selectedBytes)}</b>
                {needsPrivilege && <span style={{ color: 'var(--warn)' }}> · часть категорий требует helper → «Разрешения»</span>}
              </p>
              <button type="button" className="btn primary" disabled={selected.size === 0 || applying} onClick={() => { setAckRisky(false); setConfirmOpen(true); }}>
                {applying ? 'Очищаем…' : 'Очистить выбранное'}
              </button>
            </div>
          </Card>
        </>
      )}

      {result && (
        <Card title="Итог очистки" className="" >
          <p className="metric-value" style={{ fontSize: 22 }}>{formatBytes(result.freedBytes)} <small>освобождено</small></p>
          <table className="table" style={{ marginTop: 8 }}>
            <thead><tr><th>Категория</th><th>Результат</th><th>Освобождено</th></tr></thead>
            <tbody>
              {result.results.map((r) => (
                <tr key={r.categoryId}>
                  <td className="mono">{r.categoryId}</td>
                  <td style={{ color: r.ok ? 'var(--ok)' : 'var(--warn)' }}>{r.message}</td>
                  <td className="mono">{formatBytes(r.freedBytes)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </Card>
      )}

      {confirmOpen && (
        <Modal
          title="Подтвердите очистку"
          onClose={() => setConfirmOpen(false)}
          actions={(<>
            <button type="button" className="btn" onClick={() => setConfirmOpen(false)}>Отмена</button>
            <button type="button" className="btn danger" disabled={hasOptIn && !ackRisky} onClick={() => void apply()}>Удалить</button>
          </>)}>
          <p>Будут очищены:</p>
          <ul>
            {selectedCategories.map((c) => (
              <li key={c.id}>
                {c.name} — <span className="mono">{formatBytes(c.estimatedBytes)}</span>
                {c.requiresPrivilege && <span style={{ color: 'var(--warn)' }}> (требуются права helper)</span>}
              </li>
            ))}
          </ul>
          {hasOptIn && (
            <div className="note err" style={{ marginTop: 8 }}>
              <label className="checkbox-row">
                <input type="checkbox" checked={ackRisky} onChange={(e) => setAckRisky(e.target.checked)} />
                <span>Я понимаю: корзина/шейдеры будут удалены безвозвратно.</span>
              </label>
            </div>
          )}
          <p className="metric-sub">Файлы, изменённые после анализа, не удаляются молча: backend повторно проверяет пути и пропускает недоступные.</p>
        </Modal>
      )}
    </div>
  );
}
