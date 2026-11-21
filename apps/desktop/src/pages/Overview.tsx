import { useEffect, useRef } from 'react';
import type { Snapshot } from '../types';
import { formatBytes, formatPercent, formatRate, formatUptime, clamp } from '../lib/format';
import { RingHistory, toPolylineSegments } from '../lib/history';
import { Card, EmptyState } from '../components/ui';
import type { SectionId } from '../components/Sidebar';

interface Props {
  snapshot: Snapshot | null;
  loading: boolean;
  onNavigate: (s: SectionId) => void;
}

function meterClass(pct: number): string {
  if (pct >= 90) return 'meter err';
  if (pct >= 70) return 'meter warn';
  return 'meter';
}

export function Overview({ snapshot, loading, onNavigate }: Props) {
  const cpuHist = useRef(new RingHistory<number>(60));
  const ramHist = useRef(new RingHistory<number>(60));

  useEffect(() => {
    const cpu = snapshot?.cpuTotalPercent ?? null;
    const ramTotal = snapshot?.memoryTotalBytes ?? null;
    const ramUsed = snapshot?.memoryUsedBytes ?? null;
    const ramPct =
      ramTotal && ramUsed != null && ramTotal > 0 ? (ramUsed / ramTotal) * 100 : null;
    cpuHist.current.push(cpu);
    ramHist.current.push(ramPct);
  }, [snapshot]);

  if (!snapshot && loading) {
    return (
      <div>
        <h1 className="section-title">Обзор</h1>
        <p className="section-sub">Снимок системы собирается backend…</p>
        <div className="grid-overview">
          {[0, 1, 2, 3].map((i) => (
            <Card key={i} className="span-3"><div className="skeleton" style={{ height: 90 }} /></Card>
          ))}
        </div>
      </div>
    );
  }
  if (!snapshot) {
    return (
      <div>
        <h1 className="section-title">Обзор</h1>
        <EmptyState title="Backend не ответил. Показать данные нечем — это не «нулевая нагрузка»." />
      </div>
    );
  }

  const ramPct =
    snapshot.memoryTotalBytes && snapshot.memoryUsedBytes != null
      ? (snapshot.memoryUsedBytes / snapshot.memoryTotalBytes) * 100
      : 0;
  const swapPct =
    snapshot.swapTotalBytes && snapshot.swapUsedBytes != null
      ? (snapshot.swapUsedBytes / snapshot.swapTotalBytes) * 100
      : 0;
  const diskPct =
    snapshot.diskTotalBytes && snapshot.diskUsedBytes != null
      ? (snapshot.diskUsedBytes / snapshot.diskTotalBytes) * 100
      : 0;

  const cpuSegments = toPolylineSegments(cpuHist.current.toArray(), 640, 96, 100);
  const ramSegments = toPolylineSegments(ramHist.current.toArray(), 640, 96, 100);

  return (
    <div>
      <h1 className="section-title">Обзор</h1>
      <p className="section-sub">Фактические измерения backend (procfs / statvfs). Шаг графика ≈ 1 с, окно 60 с.</p>

      <div className="grid-overview">
        {/* CPU */}
        <Card title="CPU" hint="всего" className="span-3"
          action={snapshot.temperatureCelsius != null
            ? <span className="mono" style={{ color: 'var(--text-dim)' }}>{snapshot.temperatureCelsius.toFixed(0)} °C</span>
            : <span className="mono" style={{ color: 'var(--text-faint)' }} title="Датчик температуры недоступен">темп.: нет датчика</span>}>
          <div className="metric-value">{formatPercent(snapshot.cpuTotalPercent)}</div>
          <div className={meterClass(snapshot.cpuTotalPercent ?? 0)}><div style={{ width: `${clamp(snapshot.cpuTotalPercent ?? 0, 0, 100)}%` }} /></div>
          <div className="metric-sub">{snapshot.cpuPerCore.length > 0 ? `${snapshot.cpuPerCore.length} логических ядер` : 'ядер: нет данных'}</div>
        </Card>

        {/* RAM */}
        <Card title="Память" hint="ОЗУ + swap" className="span-3">
          <div className="metric-value">{formatBytes(snapshot.memoryUsedBytes, 1)}<small> / {formatBytes(snapshot.memoryTotalBytes, 1)}</small></div>
          <div className={meterClass(ramPct)}><div style={{ width: `${clamp(ramPct, 0, 100)}%` }} /></div>
          <div className="metric-sub">
            swap: {formatBytes(snapshot.swapUsedBytes)} / {formatBytes(snapshot.swapTotalBytes)} ({formatPercent(swapPct)})
          </div>
        </Card>

        {/* Диск */}
        <Card title="Диск" hint="без двойного счёта" className="span-3">
          <div className="metric-value">{formatBytes(snapshot.diskUsedBytes, 1)}<small> / {formatBytes(snapshot.diskTotalBytes, 1)}</small></div>
          <div className={meterClass(diskPct)}><div style={{ width: `${clamp(diskPct, 0, 100)}%` }} /></div>
          <div className="metric-sub">{snapshot.disks.length} уник. ФС · дубли монтирования учтены один раз</div>
        </Card>

        {/* Сеть */}
        <Card title="Сеть" hint="текущая скорость" className="span-3">
          <div className="metric-value" style={{ fontSize: 22 }}>↓ {formatRate(snapshot.networkRxBytesPerSec)}</div>
          <div className="metric-value" style={{ fontSize: 22, marginTop: 2 }}>↑ {formatRate(snapshot.networkTxBytesPerSec)}</div>
          <div className="metric-sub">сумма по интерфейсам, кроме loopback</div>
        </Card>

        {/* График CPU/RAM */}
        <Card title="CPU и RAM · 60 с" className="span-8"
          action={<span className="hint"><span style={{ color: 'var(--accent-2)' }}>— CPU</span>&nbsp;&nbsp;<span style={{ color: '#41b8a6' }}>— RAM</span></span>}>
          {cpuSegments.length === 0 && ramSegments.length === 0 ? (
            <div className="empty-state" style={{ padding: '18px 10px' }}>Набираем историю — первые точки появятся через несколько секунд…</div>
          ) : (
            <svg className="chart-wrap" viewBox="0 0 640 96" preserveAspectRatio="none" role="img" aria-label="График загрузки CPU и памяти за 60 секунд">
              {[24, 48, 72].map((y) => <line key={y} className="chart-grid-line" x1="0" y1={y} x2="640" y2={y} />)}
              {ramSegments.map((pts, i) => <polyline key={`r${i}`} className="chart-line ram" points={pts} />)}
              {cpuSegments.map((pts, i) => <polyline key={`c${i}`} className="chart-line" points={pts} />)}
            </svg>
          )}
        </Card>

        {/* Окружение */}
        <Card title="Окружение" className="span-4">
          <dl className="kv">
            <dt>ОС</dt><dd>{snapshot.osName ?? '—'}</dd>
            <dt>Ядро</dt><dd>{snapshot.kernel ?? '—'}</dd>
            <dt>Стол</dt><dd>{snapshot.desktop ?? '—'}</dd>
            <dt>Узел</dt><dd>{snapshot.hostname ?? '—'}</dd>
            <dt>Аптайм</dt><dd>{formatUptime(snapshot.uptimeSeconds)}</dd>
          </dl>
        </Card>

        {/* Ядра */}
        <Card title="Логические ядра" hint="нагрузка сейчас" className="span-12">
          <div className="cores-grid">
            {snapshot.cpuPerCore.map((v, i) => (
              <div className="core-cell" key={i}>
                <div className="core-label">CPU{i}</div>
                <div className="core-val">{v == null ? '—' : v.toFixed(0)}</div>
                <div className="core-bar"><div style={{ width: `${clamp(v ?? 0, 0, 100)}%` }} /></div>
              </div>
            ))}
          </div>
        </Card>

        {/* Точки монтирования */}
        <Card title="Файловые системы" hint="дубли не суммируются" className="span-8">
          <table className="table">
            <thead>
              <tr><th>Устройство</th><th>Монтирование</th><th>Тип</th><th>Занято</th><th>Свободно</th><th>Всего</th></tr>
            </thead>
            <tbody>
              {snapshot.disks.map((d) => (
                <tr key={`${d.name}-${d.mountPoint}`}>
                  <td className="mono">{d.name}</td>
                  <td className="mono" title={d.dedupedFrom.length ? `Также смонтирована как: ${d.dedupedFrom.join(', ')}` : undefined}>
                    {d.mountPoint}
                    {d.dedupedFrom.length > 0 && <span className="badge muted" style={{ marginLeft: 6 }}>+{d.dedupedFrom.length}</span>}
                  </td>
                  <td>{d.fsType ?? '—'}</td>
                  <td className="mono">{formatBytes(d.usedBytes)}</td>
                  <td className="mono">{formatBytes(d.availableBytes)}</td>
                  <td className="mono">{formatBytes(d.totalBytes)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </Card>

        {/* Быстрые действия */}
        <Card title="Быстрые действия" className="span-4">
          <div className="quick-actions" style={{ flexDirection: 'column', alignItems: 'stretch' }}>
            <button type="button" className="btn" onClick={() => onNavigate('cleanup')}>Анализ очистки <span className="mono" style={{ color: 'var(--text-faint)', marginLeft: 'auto' }}>безопасно</span></button>
            <button type="button" className="btn" onClick={() => onNavigate('network')}>Проверка зеркал</button>
            <button type="button" className="btn" onClick={() => onNavigate('network')}>Тест DNS</button>
            <button type="button" className="btn primary" onClick={() => onNavigate('jarvis')}>Открыть Джарвиса</button>
          </div>
        </Card>
      </div>
    </div>
  );
}
