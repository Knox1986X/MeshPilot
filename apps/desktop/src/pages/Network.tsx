import { useCallback, useEffect, useState } from 'react';
import type {
  BypassStatus,
  BypassTestResult,
  DnsBenchmarkResult,
  DnsStatus,
  MirrorsStatus,
  ScheduleInfo,
} from '../types';
import { api } from '../lib/api';
import { formatDateTime } from '../lib/format';
import { Badge, Card, Modal, Spinner, Switch } from '../components/ui';

type ConfirmAction = 'mirrors' | 'dns-apply' | 'dns-rollback' | 'bypass-enable' | 'bypass-disable' | null;

interface Props {
  onToast: (kind: 'ok' | 'err' | 'warn', text: string) => void;
}

export function Network({ onToast }: Props) {
  // ---- Зеркала
  const [mirrors, setMirrors] = useState<MirrorsStatus | null>(null);
  const [mirrorsBusy, setMirrorsBusy] = useState(false);
  // ---- DNS
  const [dns, setDns] = useState<DnsStatus | null>(null);
  const [bench, setBench] = useState<DnsBenchmarkResult | null>(null);
  const [dnsBusy, setDnsBusy] = useState(false);
  // ---- Обход
  const [bypass, setBypass] = useState<BypassStatus | null>(null);
  const [bypassBusy, setBypassBusy] = useState(false);
  const [checkUrls, setCheckUrls] = useState('');
  const [bypassResult, setBypassResult] = useState<BypassTestResult | null>(null);
  // ---- Расписания
  const [schedules, setSchedules] = useState<ScheduleInfo[] | null>(null);
  // ---- Подтверждение
  const [confirm, setConfirm] = useState<ConfirmAction>(null);

  const refresh = useCallback(async () => {
    const [m, d, b, s] = await Promise.allSettled([
      api.mirrorsStatus(),
      api.dnsStatus(),
      api.bypassStatus(),
      api.schedulesList(),
    ]);
    if (m.status === 'fulfilled') setMirrors(m.value);
    if (d.status === 'fulfilled') setDns(d.value);
    if (b.status === 'fulfilled') {
      setBypass(b.value);
      setCheckUrls((prev) => (prev ? prev : b.value.checkUrls.join('\n')));
    }
    if (s.status === 'fulfilled') setSchedules(s.value);
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const runConfirm = async () => {
    const action = confirm;
    setConfirm(null);
    try {
      if (action === 'mirrors') {
        setMirrorsBusy(true);
        const r = await api.mirrorsApply();
        onToast(r.ok ? 'ok' : 'warn', r.message + (r.backupPath ? ` · бэкап: ${r.backupPath}` : ''));
      } else if (action === 'dns-apply') {
        setDnsBusy(true);
        const r = await api.dnsApply();
        onToast(r.ok ? 'ok' : 'warn', r.message + (r.backupPath ? ` · бэкап: ${r.backupPath}` : ''));
      } else if (action === 'dns-rollback') {
        setDnsBusy(true);
        const r = await api.dnsRollback();
        onToast(r.ok ? 'ok' : 'warn', r.message);
      } else if (action === 'bypass-enable') {
        setBypassBusy(true);
        const r = await api.bypassEnable();
        onToast(r.ok ? 'ok' : 'warn', r.message);
      } else if (action === 'bypass-disable') {
        setBypassBusy(true);
        const r = await api.bypassDisable();
        onToast(r.ok ? 'ok' : 'warn', r.message);
      }
    } catch (e) {
      onToast('err', e instanceof Error ? e.message : String(e));
    } finally {
      setMirrorsBusy(false); setDnsBusy(false); setBypassBusy(false);
      void refresh();
    }
  };

  const runBenchmark = async () => {
    setDnsBusy(true);
    try {
      setBench(await api.dnsBenchmark());
    } catch (e) {
      onToast('err', e instanceof Error ? e.message : String(e));
    } finally {
      setDnsBusy(false);
    }
  };

  const runBypassTest = async () => {
    setBypassBusy(true);
    setBypassResult(null);
    try {
      const urls = checkUrls.split('\n').map((s) => s.trim()).filter(Boolean).slice(0, 10);
      setBypassResult(await api.bypassTest(urls));
    } catch (e) {
      onToast('err', e instanceof Error ? e.message : String(e));
    } finally {
      setBypassBusy(false);
    }
  };

  const toggleSchedule = async (id: string, enabled: boolean) => {
    try {
      const r = await api.scheduleSet(id, enabled);
      onToast(r.ok ? 'ok' : 'warn', r.message);
      setSchedules(await api.schedulesList());
    } catch (e) {
      onToast('err', e instanceof Error ? e.message : String(e));
    } finally {
    }
  };

  const confirmCopy: Record<Exclude<ConfirmAction, null>, { title: string; body: string; danger: boolean }> = {
    'mirrors': { title: 'Применить зеркала?', body: 'Текущий mirrorlist будут скопирован в бэкап (~/.local/state/meshpilot/backups/), затем выбранный инструмент заменит его атомарно. Проверка подписей пакетов не отключается. Откат возможен вручную из бэкапа.', danger: false },
    'dns-apply': { title: 'Применить лучший DNS?', body: 'Текущая конфигурация DNS будет скопирована в бэкап; затем будет записан drop-in (systemd-resolved) или применена настройка NetworkManager. Резолвинг и подключение VPN/split DNS после применения будут проверены. При ошибке — автоматический откат.', danger: false },
    'dns-rollback': { title: 'Откатить DNS?', body: 'Будет восстановлен последний бэкап конфигурации DNS, созданный MeshPilot. Если бэкапа нет — откат невозможен, будет показана ошибка.', danger: false },
    'bypass-enable': { title: 'Включить локальный прокси обхода?', body: 'Прокси будет слушать только 127.0.0.1 (loopback). Приложения придётся настроить вручную на этот порт — молча ничьи настройки не меняются. Прокси не делает вас анонимным.', danger: false },
    'bypass-disable': { title: 'Выключить прокси обхода?', body: 'Локальный сервис byedpi/zapret будет остановлен и отключён из автозапуска.', danger: false },
  };

  return (
    <div>
      <h1 className="section-title">Зеркала и DNS</h1>
      <p className="section-sub">Только подтверждённые операции с бэкапом и откатом. Ничего не меняется без вашего явного действия.</p>

      {/* ---------- ЗЕРКАЛА ---------- */}
      <Card title="Зеркала Arch / CachyOS" hint="pacman"
        action={<button type="button" className="btn small ghost" onClick={() => void refresh()}>Обновить статус</button>}>
        {!mirrors ? <Spinner /> : (
          <>
            <div className="row spread" style={{ flexWrap: 'wrap', gap: 12 }}>
              <dl className="kv" style={{ minWidth: 320 }}>
                <dt>Платформа</dt><dd>{mirrors.osName ?? 'не определена'}{mirrors.isArchLike ? '' : ' (не Arch — только чтение)'}</dd>
                <dt>Инструменты</dt><dd>{mirrors.detectedTools.length ? mirrors.detectedTools.join(', ') : 'не найдены'}</dd>
                <dt>Mirrorlist</dt><dd>{mirrors.mirrorlistPath ?? '—'}{mirrors.mirrorlistExists ? '' : ' (отсутствует)'}</dd>
                <dt>Бэкап</dt><dd>{mirrors.lastBackup ?? 'нет'}</dd>
              </dl>
              <div className="quick-actions">
                <button type="button" className="btn primary" disabled={!mirrors.isArchLike || mirrors.detectedTools.length === 0 || mirrorsBusy}
                  onClick={() => setConfirm('mirrors')}>
                  {mirrorsBusy ? 'Применяем…' : 'Применить лучшие зеркала'}
                </button>
              </div>
            </div>
            {!mirrors.isArchLike && <p className="note" style={{ marginTop: 10 }}>Зеркала управляют только на Arch/CachyOS. На этом хосте операция недоступна, кнопка отключена осознанно.</p>}
            {mirrors.isArchLike && mirrors.detectedTools.length === 0 && (
              <p className="note warn" style={{ marginTop: 10 }}>Ничего не найдено (cachyos-rate-mirrors / rate-mirrors / reflector). Установите, например: <span className="mono">sudo pacman -S cachyos-rate-mirrors</span></p>
            )}
            <p className="metric-sub" style={{ marginTop: 8 }}>Порядок: бэкап → подмена кандидатами тестом скорости → атомарная запись → проверка, что файл содержит ≥1 корректную строку mirror, иначе откат. Reflector не используется как замена обновления зеркал CachyOS.</p>
          </>
        )}
      </Card>

      {/* ---------- DNS ---------- */}
      <Card title="DNS" hint="бенчмарк с медианой, DoT где возможно">
        {!dns ? <Spinner /> : (
          <>
            <dl className="kv" style={{ gridTemplateColumns: 'auto 1fr' }}>
              <dt>Управление</dt><dd>{dns.manager} — {dns.managerDetail}</dd>
              <dt>Текущие серверы</dt><dd>{dns.currentServers.join(', ') || 'не определены'}</dd>
              <dt>DNSOverTLS</dt><dd>{dns.dnsOverTls ?? 'не раскрывается'}</dd>
              <dt>DNSSEC</dt><dd>{dns.dnssec ?? 'не раскрывается'}</dd>
              <dt>Провайдер</dt><dd>{dns.providerHint ?? 'не определён'}</dd>
              <dt>Бэкап</dt><dd>{dns.lastBackup ?? 'нет'}</dd>
            </dl>
            <div className="row" style={{ gap: 8, marginTop: 10, flexWrap: 'wrap' }}>
              <button type="button" className="btn" onClick={() => void runBenchmark()} disabled={dnsBusy}>{dnsBusy ? 'Замеряем…' : 'Замерить кандидатов'}</button>
              <button type="button" className="btn primary" onClick={() => setConfirm('dns-apply')} disabled={dnsBusy || dns.manager === 'unknown'}>Применить лучший</button>
              <button type="button" className="btn" onClick={() => setConfirm('dns-rollback')} disabled={dnsBusy || !dns.lastBackup}>Откатить</button>
            </div>
            {dns.needsPrivilege && <p className="metric-sub" style={{ marginTop: 8 }}>Применение требует root (drop-in systemd-resolved) — через будущий привилегированный helper с polkit, см. «Разрешения». В этой сборке изменения DNS не выполняются.</p>}
            <p className="metric-sub">VPN, split-DNS и локальные домены не перезаписываются. DNSSEC и DoT не отключаются. Выигрыш &lt; 5 мс относительно действующего — не повод менять сервер.</p>

            {bench && (
              <div style={{ marginTop: 12 }}>
                <table className="table">
                  <thead><tr><th>Кандидат</th><th>Адрес</th><th>Медиана, мс</th><th>Ответы</th><th>Фильтрация</th></tr></thead>
                  <tbody>
                    {bench.candidates.map((c) => (
                      <tr key={c.address}>
                        <td>{c.name}</td>
                        <td className="mono">{c.address}{c.tlsName ? ` · ${c.tlsName}` : ''}</td>
                        <td className="mono">{c.medianMs == null ? '—' : c.medianMs.toFixed(1)}</td>
                        <td className="mono">{c.okCount}/{c.okCount + c.failCount}</td>
                        <td>{c.filtered ? <Badge kind="info">фильтр.</Badge> : '—'}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
                <p className="metric-sub">Замер: {bench.domains.join(', ')} · {formatDateTime(bench.testedAtUtc)} · стабильность важнее единичного минимума.{bench.note ? ` · ${bench.note}` : ''}</p>
              </div>
            )}
          </>
        )}
      </Card>

      {/* ---------- ОБХОД ---------- */}
      <Card title="Сеть и обход блокировок" hint="byedpi / zapret, loopback">
        {!bypass ? <Spinner /> : (
          <>
            <dl className="kv">
              <dt>byedpi (ciadpi)</dt><dd>{bypass.ciadpiFound ? 'найден' : 'не найден'}</dd>
              <dt>zapret</dt><dd>{bypass.zapretFound ? 'найден' : 'не найден'}</dd>
              <dt>Локальный прокси</dt><dd>127.0.0.1:{bypass.port} · {bypass.activeService ? 'включён' : 'выключен'}</dd>
              <dt>Профиль</dt><dd>{bypass.lastProfile ?? 'не применялся'}</dd>
            </dl>
            <label htmlFor="check-urls" style={{ display: 'block', fontSize: 11, color: 'var(--text-dim)', margin: '10px 0 5px' }}>URL для проверки доступности (по одному в строке, до 10)</label>
            <textarea id="check-urls" className="mono" style={{ width: '100%', minHeight: 64, background: 'var(--bg-2)', color: 'var(--text)', border: '1px solid var(--border-2)', borderRadius: 8, padding: 8, fontSize: 11 }} value={checkUrls} onChange={(e) => setCheckUrls(e.target.value)} spellCheck={false} />
            <div className="row" style={{ gap: 8, marginTop: 8, flexWrap: 'wrap' }}>
              <button type="button" className="btn" onClick={() => void runBypassTest()} disabled={bypassBusy}>{bypassBusy ? 'Проверяем…' : 'Проверить и подобрать стратегию'}</button>
              {bypass.activeService
                ? <button type="button" className="btn" onClick={() => setConfirm('bypass-disable')} disabled={bypassBusy}>Выключить прокси</button>
                : <button type="button" className="btn primary" onClick={() => setConfirm('bypass-enable')} disabled={bypassBusy || (!bypass.ciadpiFound && !bypass.zapretFound)}>Включить прокси</button>}
            </div>
            {bypassResult && (
              <div style={{ marginTop: 10 }}>
                <p className="metric-sub">Напрямую: {bypassResult.direct.map((d) => `${d.ok ? '✓' : '✗'} ${d.url}`).join('  ·  ') || '—'}</p>
                {bypassResult.strategies.length > 0 && (
                  <table className="table" style={{ marginTop: 6 }}>
                    <thead><tr><th>Стратегия</th><th>Успех</th><th></th></tr></thead>
                    <tbody>
                      {bypassResult.strategies.map((s) => (
                        <tr key={s.label}>
                          <td className="mono" style={{ fontSize: 10.5 }}>{s.label}</td>
                          <td className="mono">{s.passed}/{s.total}</td>
                          <td>{s.works ? <Badge kind="safe">работает</Badge> : <Badge kind="muted">нет</Badge>}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                )}
              </div>
            )}
            <p className="metric-sub" style={{ marginTop: 8 }}>Прокси слушает только loopback. Настройте приложения на него явно. Никаких «лучших стратегий» из интернета автоматически. Анонимность сменой DNS не обеспечивается.</p>
          </>
        )}
      </Card>

      {/* ---------- РАСПИСАНИЯ ---------- */}
      <Card title="Расписания (systemd user timers)" hint="отчёт по умолчанию">
        {!schedules ? <Spinner /> : (
          <table className="table">
            <thead><tr><th>Расписание</th><th>Когда</th><th>Режим</th><th>Последний запуск</th><th>Следующий</th><th style={{ textAlign: 'right' }}>Вкл</th></tr></thead>
            <tbody>
              {schedules.map((s) => (
                <tr key={s.id}>
                  <td>
                    <div style={{ fontWeight: 600 }}>{s.name}</div>
                    <div style={{ color: 'var(--text-faint)', fontSize: 10.5, maxWidth: 460 }}>{s.description}</div>
                    {!s.supported && <div style={{ color: 'var(--warn)', fontSize: 10.5 }}>{s.unsupportedReason ?? 'недоступно на этом хосте'}</div>}
                  </td>
                  <td className="mono" style={{ fontSize: 11 }}>{s.calendar}</td>
                  <td>{s.mode === 'apply' ? <Badge kind="caution">применить</Badge> : <Badge kind="info">только отчёт</Badge>}</td>
                  <td className="mono" style={{ fontSize: 11 }}>{s.lastRun ? formatDateTime(s.lastRun) : '—'}{s.lastResult ? ` · ${s.lastResult}` : ''}</td>
                  <td className="mono" style={{ fontSize: 11 }}>{s.nextRun ? formatDateTime(s.nextRun) : '—'}</td>
                  <td style={{ textAlign: 'right' }}>
                    <Switch checked={s.enabled} label={`${s.name}: ${s.enabled ? 'выключить' : 'включить'}`} onChange={(v) => void toggleSchedule(s.id, v)} />
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
        <p className="metric-sub" style={{ marginTop: 8 }}>Ежедневно 05:00 зеркала и 05:15 DNS, еженедельно вс 05:20 сетевой профиль, вс 06:00 безопасная очистка. Таймеры с защитой от параллельных запусков (<span className="mono">flock</span>), <span className="mono">Persistent=true</span> (догон после сна). Автоприменение включается отдельно в настройках — по умолчанию только отчёт.</p>
      </Card>

      {confirm && (
        <Modal title={confirmCopy[confirm].title} onClose={() => setConfirm(null)}
          actions={(<>
            <button type="button" className="btn" onClick={() => setConfirm(null)}>Отмена</button>
            <button type="button" className={`btn ${confirmCopy[confirm].danger ? 'danger' : 'primary'}`} onClick={() => void runConfirm()}>Подтвердить</button>
          </>)}>
          <p>{confirmCopy[confirm].body}</p>
        </Modal>
      )}
    </div>
  );
}
