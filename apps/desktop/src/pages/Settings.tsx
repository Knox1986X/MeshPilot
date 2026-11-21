import { useCallback, useEffect, useMemo, useState } from 'react';
import type { ConnectionTestResult, ProviderId, ProviderStatus, Settings as SettingsType } from '../types';
import { api } from '../lib/api';
import { Badge, Card, Divider, Modal, Switch } from '../components/ui';

interface Props {
  settings: SettingsType;
  onSettingsChange: (patch: Partial<SettingsType>) => Promise<void>;
  onToast: (kind: 'ok' | 'err' | 'warn', text: string) => void;
}

type Tab = 'connections' | 'jarvis' | 'cleanup' | 'network' | 'about';

const CURATED: Record<ProviderId, string[]> = {
  groq: ['llama-3.3-70b-versatile', 'llama-3.1-8b-instant', 'openai/gpt-oss-120b', 'openai/gpt-oss-20b', 'qwen/qwen3-32b', 'meta-llama/llama-4-scout-17b-16e-instruct'],
  openrouter: ['openai/gpt-4o-mini', 'anthropic/claude-3.5-haiku', 'google/gemini-2.0-flash-001'],
};

const PROVIDER_LABEL: Record<ProviderId, string> = {
  groq: 'Groq (api.groq.com)',
  openrouter: 'OpenRouter (openrouter.ai)',
};

const TEST_TEXT: Record<ConnectionTestResult['code'], { kind: 'ok' | 'err' | 'warn'; text: string }> = {
  'ok': { kind: 'ok', text: 'Подключение успешно' },
  'no-network': { kind: 'err', text: 'Нет сети или хост недоступен — локальные функции продолжают работать' },
  'invalid-key': { kind: 'err', text: 'Неверный ключ (401/403). Удалите и сохраните действующий ключ' },
  'rate-limited': { kind: 'warn', text: 'Лимит запросов (429). Подождите или проверьте квоты в консоли провайдера' },
  'model-unavailable': { kind: 'warn', text: 'Модель недоступна для этого ключа (404/400). Выберите другую модель' },
  'server-error': { kind: 'err', text: 'Сервис провайдера недоступен (5xx). Повторите позже' },
  'timeout': { kind: 'warn', text: 'Таймаут — увеличьте «Таймаут, с» или проверьте сеть' },
  'unknown': { kind: 'err', text: 'Неизвестная ошибка — см. подробности выше' },
};

export function SettingsPage({ settings, onSettingsChange, onToast }: Props) {
  const [tab, setTab] = useState<Tab>('connections');

  return (
    <div>
      <h1 className="section-title">Настройки</h1>
      <p className="section-sub">Секреты — только в системном хранилище (Secret Service/KWallet) или до закрытия приложения. В JSON настроек ключей нет.</p>

      <div className="row" style={{ gap: 6, marginBottom: 12, flexWrap: 'wrap' }} role="tablist" aria-label="Разделы настроек">
        {([
          ['connections', 'Помощник · Подключения'],
          ['jarvis', 'Джарвис'],
          ['cleanup', 'Очистка'],
          ['network', 'Сеть'],
          ['about', 'Приложение и данные'],
        ] as Array<[Tab, string]>).map(([id, label]) => (
          <button key={id} role="tab" aria-selected={tab === id} type="button" className={`btn small${tab === id ? ' primary' : ''}`} onClick={() => setTab(id)}>{label}</button>
        ))}
      </div>

      {tab === 'connections' && <Connections settings={settings} onSettingsChange={onSettingsChange} onToast={onToast} />}
      {tab === 'jarvis' && <JarvisTab settings={settings} onSettingsChange={onSettingsChange} onToast={onToast} />}
      {tab === 'cleanup' && <CleanupTab settings={settings} onSettingsChange={onSettingsChange} />}
      {tab === 'network' && <NetworkTab settings={settings} onSettingsChange={onSettingsChange} />}
      {tab === 'about' && <AboutTab settings={settings} onSettingsChange={onSettingsChange} onToast={onToast} />}
    </div>
  );
}

// ================= Подключения =================

function Connections({ settings, onSettingsChange, onToast }: Props) {
  const a = settings.assistant;
  const [status, setStatus] = useState<Record<ProviderId, ProviderStatus | null>>({ groq: null, openrouter: null });
  const [keyDraft, setKeyDraft] = useState<Record<ProviderId, string>>({ groq: '', openrouter: '' });
  const [showKey, setShowKey] = useState<Record<ProviderId, boolean>>({ groq: false, openrouter: false });
  const [models, setModels] = useState<string[]>([]);
  const [modelsLive, setModelsLive] = useState(false);
  const [testing, setTesting] = useState<ProviderId | null>(null);
  const [testResult, setTestResult] = useState<ConnectionTestResult | null>(null);
  const [busy, setBusy] = useState(false);

  const refreshStatus = useCallback(async () => {
    const [g, o] = await Promise.all([api.providerStatus('groq').catch(() => null), api.providerStatus('openrouter').catch(() => null)]);
    setStatus({ groq: g, openrouter: o });
  }, []);

  useEffect(() => { void refreshStatus(); }, [refreshStatus]);

  const refreshModels = useCallback(async () => {
    setModelsLive(false);
    try {
      const live = await api.listModels(a.provider);
      setModels(live.length ? live : CURATED[a.provider]);
      setModelsLive(live.length > 0);
    } catch {
      setModels(CURATED[a.provider]);
      setModelsLive(false);
    }
  }, [a.provider]);

  useEffect(() => { void refreshModels(); }, [refreshModels]);

  const saveKey = async (provider: ProviderId) => {
    const key = keyDraft[provider].trim();
    if (key.length < 8) {
      onToast('err', 'Похоже, ключ слишком короткий');
      return;
    }
    setBusy(true);
    try {
      const r = await api.saveKey(provider, key);
      setKeyDraft((d) => ({ ...d, [provider]: '' }));
      onToast(r.storage === 'keyring' ? 'ok' : 'warn', r.message);
      await refreshStatus();
      void refreshModels();
    } catch (e) {
      onToast('err', e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const deleteKey = async (provider: ProviderId) => {
    setBusy(true);
    try {
      const r = await api.deleteKey(provider);
      onToast(r.ok ? 'ok' : 'warn', r.message);
      await refreshStatus();
    } catch (e) {
      onToast('err', e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const test = async (provider: ProviderId, model: string) => {
    setTesting(provider);
    setTestResult(null);
    try {
      setTestResult(await api.testConnection(provider, model));
    } catch (e) {
      setTestResult({ ok: false, code: 'unknown', message: e instanceof Error ? e.message : String(e), latencyMs: null });
    } finally {
      setTesting(null);
    }
  };

  const providerBlock = (provider: ProviderId, isBackup: boolean) => {
    const st = status[provider];
    const title = isBackup ? 'Резервный провайдер' : 'Основной провайдер';
    return (
      <div key={`${provider}-${isBackup ? 'b' : 'm'}`} className="card-2" style={{ border: '1px solid var(--border)', borderRadius: 12, padding: 12, marginBottom: 12, background: 'var(--card-2)' }}>
        <div className="row spread" style={{ marginBottom: 8 }}>
          <b>{title}: {PROVIDER_LABEL[provider]}</b>
          {st ? (
            st.hasKey
              ? <Badge kind="safe">ключ сохранён · {st.storage === 'keyring' ? 'Secret Service' : 'только до закрытия'}</Badge>
              : <Badge kind="muted">ключ не сохранён</Badge>
          ) : <Badge kind="muted">…</Badge>}
        </div>

        <div className="field">
          <label htmlFor={`key-${provider}-${isBackup}`}>API-ключ {isBackup ? `(${PROVIDER_LABEL[provider]})` : ''}</label>
          <div className="key-row">
            <input
              id={`key-${provider}-${isBackup}`}
              type={showKey[provider] ? 'text' : 'password'}
              value={keyDraft[provider]}
              onChange={(e) => setKeyDraft((d) => ({ ...d, [provider]: e.target.value }))}
              placeholder="Вставьте ключ…"
              autoComplete="off"
              spellCheck={false}
              aria-describedby={`key-help-${provider}`}
            />
            <button type="button" className="btn small" onClick={() => setShowKey((s) => ({ ...s, [provider]: !s[provider] }))} aria-label={showKey[provider] ? 'Скрыть ключ' : 'Показать ключ'}>
              {showKey[provider] ? 'Скрыть' : 'Показать'}
            </button>
            <button type="button" className="btn small primary" disabled={busy || !keyDraft[provider].trim()} onClick={() => void saveKey(provider)}>Сохранить</button>
          </div>
          <p className="help" id={`key-help-${provider}`}>Ключ хранится в Secret Service/KWallet. Если хранилище недоступно — только до закрытия приложения; вы будете предупреждены. Поле очищается после сохранения; ключ никогда не возвращается в интерфейс.</p>
        </div>

        <div className="row" style={{ gap: 8, flexWrap: 'wrap' }}>
          <button type="button" className="btn" disabled={testing !== null || !st?.hasKey} onClick={() => void test(provider, a.provider === provider ? a.model : (a.backupModel ?? a.model))}>
            {testing === provider ? 'Проверяем…' : 'Проверить подключение'}
          </button>
          <button type="button" className="btn danger" disabled={busy || !st?.hasKey} onClick={() => void deleteKey(provider)}>Удалить ключ</button>
        </div>

        {testResult && a.provider === provider && (
          <div className={`note ${TEST_TEXT[testResult.code].kind === 'ok' ? 'ok' : TEST_TEXT[testResult.code].kind === 'warn' ? 'warn' : 'err'}`} style={{ marginTop: 8 }} role="status">
            {TEST_TEXT[testResult.code].text}
            {testResult.latencyMs != null && <span className="mono"> · {testResult.latencyMs} мс</span>}
            {testResult.message && testResult.code !== 'ok' && <div style={{ marginTop: 4, fontSize: 10.5, opacity: 0.85 }}>{testResult.message}</div>}
          </div>
        )}
      </div>
    );
  };

  return (
    <>
      <Card title="Провайдер">
        <div className="field">
          <label htmlFor="provider">Основной провайдер</label>
          <select id="provider" value={a.provider} onChange={(e) => void onSettingsChange({ assistant: { ...a, provider: e.target.value as ProviderId } })}>
            <option value="groq">Groq — официальный API (api.groq.com), рекомендуется</option>
            <option value="openrouter">OpenRouter — OpenAI-совместимый шлюз (опционально)</option>
          </select>
          <p className="help">Запросы идут только к выбранному провайдеру. Файлы, буфер обмена и системные журналы автоматически не прикрепляются. Бесплатный API не без ограничений: действуют RPM/TPM/TPD провайдера — при 429 приложение повторяет с задержкой и показывает честную ошибку.</p>
        </div>

        <div className="field">
          <label htmlFor="model">Модель {modelsLive ? <span style={{ color: 'var(--ok)' }}>· список из API</span> : <span style={{ color: 'var(--text-faint)' }}>· встроенный список (API недоступен)</span>}</label>
          <select id="model" value={a.model} onChange={(e) => void onSettingsChange({ assistant: { ...a, model: e.target.value } })}>
            {models.map((m) => <option key={m} value={m}>{m}</option>)}
          </select>
          <p className="help">Актуальные названия и лимиты: console.groq.com/docs/models. Llama 3.3 70B — 128K контекст · ~100K токенов/день на бесплатном ярусе.</p>
        </div>

        <div className="row" style={{ gap: 12, flexWrap: 'wrap' }}>
          <div className="field" style={{ width: 140 }}>
            <label htmlFor="timeout">Таймаут, с</label>
            <input id="timeout" type="number" min={5} max={120} value={a.timeoutSeconds} onChange={(e) => void onSettingsChange({ assistant: { ...a, timeoutSeconds: Number(e.target.value) || 30 } })} />
          </div>
          <div className="field" style={{ width: 140 }}>
            <label htmlFor="maxtokens">Макс. токенов ответа</label>
            <input id="maxtokens" type="number" min={64} max={8192} step={64} value={a.maxTokens} onChange={(e) => void onSettingsChange({ assistant: { ...a, maxTokens: Number(e.target.value) || 1024 } })} />
          </div>
          <div className="field" style={{ width: 220 }}>
            <label htmlFor="temperature">Температура: {a.temperature.toFixed(2)}</label>
            <input id="temperature" type="range" min={0} max={1} step={0.05} value={a.temperature} onChange={(e) => void onSettingsChange({ assistant: { ...a, temperature: Number(e.target.value) } })} />
          </div>
        </div>

        <Divider />

        {providerBlock(a.provider, false)}
      </Card>

      <Card title="Резервный провайдер" hint="включается отдельно">
        <div className="checkbox-row" style={{ justifyContent: 'space-between', marginBottom: 8 }}>
          <label htmlFor="backup-enabled">Использовать резервный провайдер при недоступности основного</label>
          <Switch id="backup-enabled" checked={a.backupEnabled} label="Резервный провайдер" onChange={(v) => void onSettingsChange({ assistant: { ...a, backupEnabled: v, backupProvider: v ? (a.provider === 'groq' ? 'openrouter' : 'groq') : null } })} />
        </div>
        {a.backupEnabled && (
          <>
            <div className="note warn" style={{ marginBottom: 10 }}>
              Понимайте, куда уйдут запросы: при недоступности основного провайдера текст ваших сообщений будет отправлен резервному
              ({a.backupProvider ? PROVIDER_LABEL[a.backupProvider] : '—'}) со своим ключом. Без вашего включения этого не происходит.
            </div>
            <div className="field" style={{ maxWidth: 340 }}>
              <label htmlFor="backup-provider">Резервный провайдер</label>
              <select id="backup-provider" value={a.backupProvider ?? 'openrouter'} onChange={(e) => void onSettingsChange({ assistant: { ...a, backupProvider: e.target.value as ProviderId } })}>
                {a.provider !== 'groq' && <option value="groq">Groq (api.groq.com)</option>}
                {a.provider !== 'openrouter' && <option value="openrouter">OpenRouter (openrouter.ai)</option>}
              </select>
            </div>
            <div className="field" style={{ maxWidth: 340 }}>
              <label htmlFor="backup-model">Модель резервного провайдера</label>
              <input id="backup-model" value={a.backupModel ?? ''} placeholder="например, openai/gpt-4o-mini" spellCheck={false} onChange={(e) => void onSettingsChange({ assistant: { ...a, backupModel: e.target.value } })} />
            </div>
            {a.backupProvider && providerBlock(a.backupProvider, true)}
          </>
        )}
        <p className="metric-sub">Удаление ключа резервного провайдера: кнопка «Удалить ключ» в его блоке выше. Отключение — переключателем.</p>
      </Card>
    </>
  );
}

// ================= Джарвис =================

function JarvisTab({ settings, onSettingsChange, onToast }: Props) {
  const j = settings.jarvis;
  const a = settings.assistant;
  const [clearing, setClearing] = useState(false);
  const [confirmClear, setConfirmClear] = useState(false);

  return (
    <Card title="Джарвис">
      <div className="checkbox-row" style={{ justifyContent: 'space-between', marginBottom: 10 }}>
        <label htmlFor="sir-addr">Обращение «сэр» в ответах</label>
        <Switch checked={a.sirAddressing} label="Обращение сэр" onChange={(v) => void onSettingsChange({ assistant: { ...a, sirAddressing: v } })} />
      </div>
      <div className="field" style={{ maxWidth: 300 }}>
        <label htmlFor="user-name">Имя пользователя (необязательно)</label>
        <input id="user-name" value={a.userName} placeholder="Как к вам обращаться" onChange={(e) => void onSettingsChange({ assistant: { ...a, userName: e.target.value } })} />
      </div>
      <Divider />
      <div className="checkbox-row" style={{ justifyContent: 'space-between', marginBottom: 10 }}>
        <div>
          <label htmlFor="save-history">Сохранять историю чата локально</label>
          <p className="help" style={{ margin: '2px 0 0' }}>Выключено — переписка нигде не хранится. Включено — ~/.local/share/meshpilot/history.json (можно удалить в любой момент).</p>
        </div>
        <Switch checked={j.saveHistory} label="Сохранять историю" onChange={(v) => void onSettingsChange({ jarvis: { ...j, saveHistory: v } })} />
      </div>
      <div className="field" style={{ maxWidth: 200 }}>
        <label htmlFor="history-limit">Лимит сообщений в истории</label>
        <input id="history-limit" type="number" min={10} max={2000} step={10} value={j.historyLimit} onChange={(e) => void onSettingsChange({ jarvis: { ...j, historyLimit: Number(e.target.value) || 200 } })} />
      </div>
      <div className="row" style={{ gap: 8 }}>
        <button type="button" className="btn small danger" disabled={clearing} onClick={() => setConfirmClear(true)}>Удалить историю сейчас</button>
      </div>
      <Divider />
      <div className="field" style={{ maxWidth: 340 }}>
        <label htmlFor="hotkey">Глобальная горячая клавиша (push-to-talk)</label>
        <input id="hotkey" value={j.pushToTalkHotkey} readOnly aria-readonly="true" />
        <p className="help">На Wayland произвольный перехват клавиатуры не поддерживается. Глобальный хоткей будет работать через портals (XDG GlobalShortcuts) или KDE-шорткат, когда голосовой конвейер будет подключён. Сейчас значение — зарезервированное отображение.</p>
      </div>

      {confirmClear && (
        <Modal title="Удалить историю чата?" onClose={() => setConfirmClear(false)}
          actions={(<>
            <button type="button" className="btn" onClick={() => setConfirmClear(false)}>Отмена</button>
            <button type="button" className="btn danger" onClick={async () => {
              setConfirmClear(false);
              setClearing(true);
              try { await api.historyClear(); onToast('ok', 'История удалена'); }
              catch { onToast('warn', 'История удалена только в окне'); }
              finally { setClearing(false); }
            }}>Удалить</button>
          </>)}>
          <p>Все сохранённые сообщения будут удалены с диска. Отменить нельзя.</p>
        </Modal>
      )}
    </Card>
  );
}

// ================= Очистка =================

function CleanupTab({ settings, onSettingsChange }: Pick<Props, 'settings' | 'onSettingsChange'>) {
  const c = settings.cleanup;
  return (
    <Card title="Политика очистки">
      <div className="checkbox-row" style={{ justifyContent: 'space-between', marginBottom: 10 }}>
        <div>
          <label htmlFor="safe-only">Только безопасный профиль</label>
          <p className="help" style={{ margin: '2px 0 0' }}>Рисковые категории (корзина, shader-кэш) в расписании и по умолчанию не участвуют.</p>
        </div>
        <Switch checked={c.safeProfileOnly} label="Только безопасный профиль" onChange={(v) => void onSettingsChange({ cleanup: { ...c, safeProfileOnly: v } })} />
      </div>
      <div className="checkbox-row" style={{ justifyContent: 'space-between', marginBottom: 10 }}>
        <div>
          <label htmlFor="include-trash">Включать корзину при явной очистке</label>
          <p className="help" style={{ margin: '2px 0 0' }}>Корзина никогда не входит в автоматическую очистку. Только вручную, с отдельным подтверждением.</p>
        </div>
        <Switch checked={c.includeTrash} label="Включать корзину" onChange={(v) => void onSettingsChange({ cleanup: { ...c, includeTrash: v } })} />
      </div>
      <div className="row" style={{ gap: 12, flexWrap: 'wrap' }}>
        <div className="field" style={{ width: 220 }}>
          <label htmlFor="keep-pacman">Оставлять версий pacman</label>
          <input id="keep-pacman" type="number" min={1} max={10} value={c.keepPacmanVersions} onChange={(e) => void onSettingsChange({ cleanup: { ...c, keepPacmanVersions: Math.max(1, Number(e.target.value) || 2) } })} />
        </div>
        <div className="field" style={{ width: 220 }}>
          <label htmlFor="journal-mb">journald, лимит размера, МБ</label>
          <input id="journal-mb" type="number" min={50} max={10000} step={50} value={c.journalMaxSizeMb} onChange={(e) => void onSettingsChange({ cleanup: { ...c, journalMaxSizeMb: Number(e.target.value) || 500 } })} />
        </div>
      </div>
      <p className="metric-sub">«Сессии, пароли, история, загрузки, исходники, виртуальные окружения и установленные приложения» никогда не считаются кэшем.</p>
    </Card>
  );
}

// ================= Сеть =================

function NetworkTab({ settings, onSettingsChange }: Pick<Props, 'settings' | 'onSettingsChange'>) {
  const n = settings.network;
  return (
    <Card title="Сетевой профиль обхода">
      <div className="field" style={{ maxWidth: 220 }}>
        <label htmlFor="bypass-port">Порт локального прокси</label>
        <input id="bypass-port" type="number" min={1024} max={65535} value={n.bypassPort} onChange={(e) => void onSettingsChange({ network: { ...n, bypassPort: Number(e.target.value) || 18080 } })} />
        <p className="help">Только 127.0.0.1 (loopback). Внешний интерфейс не открывается ни при каких настройках.</p>
      </div>
      <p className="metric-sub">Список проверяемых URL редактируется на странице «Зеркала и DNS → Сеть и обход».</p>
    </Card>
  );
}

// ================= Приложение и данные =================

function AboutTab({ settings, onSettingsChange, onToast }: Props) {
  const exportJson = useMemo(() => JSON.stringify(settings, null, 2), [settings]);
  const [confirmReset, setConfirmReset] = useState(false);

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(exportJson);
      onToast('ok', 'JSON настроек скопирован (без секретов)');
    } catch {
      onToast('warn', 'Буфер не дал прочитать — JSON ниже');
    }
  };

  return (
    <Card title="Приложение и данные">
      <dl className="kv" style={{ maxWidth: 520 }}>
        <dt>Версия</dt><dd>0.2.0</dd>
        <dt>Схема настроек</dt><dd>v{settings.schemaVersion}</dd>
        <dt>Конфигурация</dt><dd>~/.config/meshpilot/settings.json</dd>
        <dt>Состояние / логи</dt><dd>~/.local/state/meshpilot/</dd>
        <dt>Данные (история, отчёты)</dt><dd>~/.local/share/meshpilot/</dd>
        <dt>История изменений настроек</dt><dd>~/.local/state/meshpilot/settings-history.jsonl</dd>
      </dl>
      <Divider />
      <div className="row" style={{ gap: 8, flexWrap: 'wrap' }}>
        <button type="button" className="btn" onClick={() => void copy()}>Копировать JSON настроек</button>
        <button type="button" className="btn danger" onClick={() => setConfirmReset(true)}>Сбросить настройки</button>
      </div>
      <p className="metric-sub" style={{ marginTop: 8 }}>Экспорт не содержит ключей API (их вообще нет в схеме настроек). Секреты удаляются только кнопкой «Удалить ключ» в подключениях.</p>

      {confirmReset && (
        <Modal title="Сбросить настройки?" onClose={() => setConfirmReset(false)}
          actions={(<>
            <button type="button" className="btn" onClick={() => setConfirmReset(false)}>Отмена</button>
            <button type="button" className="btn danger" onClick={async () => { setConfirmReset(false); try { await api.settingsReset(); onToast('ok', 'Настройки сброшены к умолчанию'); void (onSettingsChange({})); } catch (e) { onToast('err', e instanceof Error ? e.message : String(e)); } }}>Сбросить</button>
          </>)}>
          <p>Все настройки вернутся к значениям по умолчанию. Ключи API не удаляются (они не в настройках) — их нужно удалить отдельно.</p>
        </Modal>
      )}
    </Card>
  );
}
