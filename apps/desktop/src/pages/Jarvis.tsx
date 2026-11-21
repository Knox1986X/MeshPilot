import { useCallback, useEffect, useRef, useState } from 'react';
import type { ChatMessage, Settings, ToolCallInfo } from '../types';
import { api } from '../lib/api';
import { formatDateTime } from '../lib/format';
import { Card, Switch, Modal } from '../components/ui';
import type { SectionId } from '../components/Sidebar';

type AssistantState = 'idle' | 'thinking' | 'executing' | 'answering';

interface Props {
  settings: Settings;
  onOpenSettings: () => void;
  onNavigate: (s: SectionId) => void;
  onToast: (kind: 'ok' | 'err' | 'warn', text: string) => void;
  onApiStateChange: (connected: boolean | null) => void;
  onSettingsChange: (patch: Partial<Settings>) => Promise<void>;
}

const STATE_LABEL: Record<AssistantState, string> = {
  idle: 'готов',
  thinking: 'думаю…',
  executing: 'выполняю инструмент…',
  answering: 'отвечаю…',
};

export function Jarvis({ settings, onOpenSettings, onNavigate, onToast, onApiStateChange, onSettingsChange }: Props) {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [input, setInput] = useState('');
  const [state, setState] = useState<AssistantState>('idle');
  const [confirmClear, setConfirmClear] = useState(false);
  const listRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let cancelled = false;
    api.historyLoad().then((h) => {
      if (!cancelled) setMessages(h);
    }).catch(() => undefined);
    return () => { cancelled = true; };
  }, []);

  useEffect(() => {
    listRef.current?.scrollTo({ top: listRef.current.scrollHeight });
  }, [messages]);

  const sending = state !== 'idle';

  const send = useCallback(async () => {
    const text = input.trim();
    if (!text || sending) return;
    const userMsg: ChatMessage = {
      id: `u-${Date.now()}`,
      role: 'user',
      content: text,
      createdAtUtc: new Date().toISOString(),
      toolCalls: [],
      error: null,
    };
    const next = [...messages, userMsg];
    setMessages(next);
    setInput('');
    setState('thinking');
    try {
      const history = next
        .filter((m) => !m.error)
        .slice(-20)
        .map((m) => ({ role: m.role, content: m.content }));
      const result = await api.jarvisSend({ messages: history });
      const reply = result.reply;
      const after = [...next, reply];
      setMessages(after);
      setState(reply.toolCalls.length > 0 ? 'executing' : 'answering');
      // Навигация по инструменту open_section (если модель его вызвала)
      for (const call of reply.toolCalls) {
        if (call.tool === 'open_section' && ['overview', 'cleanup', 'network', 'permissions', 'settings', 'jarvis'].includes(call.detail)) {
          onNavigate(call.detail as SectionId);
        }
      }
      onApiStateChange(true);
      setState('idle');
      if (settings.jarvis.saveHistory) {
        const limit = Math.max(1, settings.jarvis.historyLimit);
        void api.historySave(after.slice(-limit)).catch(() => undefined);
      }
    } catch (e) {
      const message = e instanceof Error ? e.message : String(e);
      const failMsg: ChatMessage = {
        id: `e-${Date.now()}`,
        role: 'assistant',
        content: '',
        createdAtUtc: new Date().toISOString(),
        toolCalls: [],
        error: message,
      };
      setMessages([...next, failMsg]);
      setState('idle');
      onApiStateChange(false);
    }
  }, [input, messages, onApiStateChange, onNavigate, sending, settings.jarvis.historyLimit, settings.jarvis.saveHistory]);

  const onKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      void send();
    }
  };

  const clearHistory = async () => {
    setConfirmClear(false);
    setMessages([]);
    try {
      await api.historyClear();
      onToast('ok', 'История чата удалена с диска');
    } catch {
      onToast('warn', 'История очищена только в окне (backend недоступен)');
    }
  };

  const historyCount = messages.length;

  return (
    <div>
      <h1 className="section-title">Джарвис</h1>
      <p className="section-sub">Текстовый чат с облачной моделью · инструменты только из закрытого набора · системные действия — только через подтверждающие диалоги в интерфейсе.</p>

      <div className="chat-layout">
        {/* Чат */}
        <Card className="chat-main" title="Джарвис" hint={settings.assistant.model}
          action={<button type="button" className="btn small ghost" onClick={onOpenSettings}>Подключение →</button>}>
          <div className="chat-messages" ref={listRef} aria-live="polite">
            {messages.length === 0 && (
              <div className="empty-state">
                <p style={{ marginBottom: 6 }}>Джарвис на связи. Спросите, например:</p>
                <p className="mono" style={{ color: 'var(--text-dim)' }}>«Состояние системы»<br />«Открой терминал»<br />«Сколько места на диске?»</p>
              </div>
            )}
            {messages.map((m) => (
              <div key={m.id} className={m.error ? 'msg error' : `msg ${m.role}`}>
                {m.content}
                {m.error && <div>Ошибка: {m.error}</div>}
                {m.toolCalls.length > 0 && (
                  <div aria-label="Вызванные инструменты">
                    {m.toolCalls.map((c: ToolCallInfo, i: number) => (
                      <span key={i} className={`tool-chip ${c.ok ? 'ok' : 'fail'}`} title={c.detail}>
                        {c.ok ? '✓' : '✗'} {c.label}
                      </span>
                    ))}
                  </div>
                )}
                <div className="msg-meta">{formatDateTime(m.createdAtUtc)}</div>
              </div>
            ))}
          </div>

          <div className="assistant-state">
            {sending && <span className="pulse" aria-hidden="true" />}
            <span>{STATE_LABEL[state]}</span>
            <span style={{ marginLeft: 'auto', color: 'var(--text-faint)' }}>Enter — отправить · Shift+Enter — перенос</span>
          </div>

          <div className="chat-input-row">
            <button type="button" className="btn" title="Голосовой ввод появится после интеграции локального распознавания (faster-whisper sidecar). В этой сборке недоступен."
              aria-label="Push-to-talk (недоступно в этой сборке)"
              onClick={() => onToast('warn', 'Голосовой ввод в этой сборке не подключён: нужен локальный распознаватель (faster-whisper) или явное включение облачного. См. PROJECT_STATE.md')}>
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                <rect x="9" y="2" width="6" height="12" rx="3" /><path d="M5 11a7 7 0 0 0 14 0" /><path d="M12 18v3" />
              </svg>
            </button>
            <textarea
              value={input}
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={onKeyDown}
              placeholder="Спросите Джарвиса…"
              aria-label="Сообщение для Джарвиса"
              disabled={sending}
            />
            <button type="button" className="btn primary" onClick={() => void send()} disabled={sending || !input.trim()}>
              {sending ? '…' : 'Отправить'}
            </button>
          </div>
        </Card>

        {/* Правая колонка */}
        <aside className="chat-side" style={{ display: 'flex', flexDirection: 'column', gap: 12 }} aria-label="Персона и история">
          <Card title="Персона">
            <div className="checkbox-row" style={{ justifyContent: 'space-between' }}>
              <label htmlFor="sir-switch">Обращение «сэр»</label>
              <Switch id="sir-switch" checked={settings.assistant.sirAddressing} label="Обращение сэр" onChange={(v) => void onSettingsChange({ assistant: { ...settings.assistant, sirAddressing: v } })} />
            </div>
            <p className="note" style={{ marginTop: 8 }}>Краткие полезные ответы, лёгкая ирония. Личность не мешает точности. Изменить можно в «Настройки → Помощник».</p>
          </Card>

          <Card title="Голосовой режим">
            <p className="note">Push-to-talk и wake word «Джарвис» не подключены в этой сборке: на Wayland глобальный перехват клавиш ограничен, а локальное распознавание требует sidecar (faster-whisper + Silero TTS, лицензия MIT). Включение — только явное, с отдельным индикатором состояния.</p>
            <div className="checkbox-row" style={{ justifyContent: 'space-between', marginTop: 8, opacity: 0.55 }}>
              <label htmlFor="wake-switch">Wake word «Джарвис»</label>
              <span className="switch" role="switch" aria-checked="false" aria-disabled="true" aria-label="Wake word (недоступно)" />
            </div>
          </Card>

          <Card title="Инструменты (закрытый набор)">
            <ul style={{ margin: 0, paddingLeft: 16, color: 'var(--text-dim)', fontSize: 11.5, lineHeight: 1.9 }}>
              <li><span className="mono">get_system_summary</span> — сводка ПК</li>
              <li><span className="mono">analyze_cleanup</span> — анализ очистки (dry-run)</li>
              <li><span className="mono">open_application</span> — открыть разрешённое</li>
              <li><span className="mono">set_volume</span>, <span className="mono">media_control</span></li>
              <li><span className="mono">create_timer</span></li>
              <li><span className="mono">open_section</span> — перейти к разделу</li>
            </ul>
            <p className="note warn" style={{ marginTop: 8 }}>Удаление данных, сеть, зеркала, DNS и выключение ПК Джарвис выполнять не может — только кнопочные действия с подтверждением.</p>
          </Card>

          <Card title="История" hint={`${historyCount} сообщений`}>
            <p className="metric-sub">Хранится локально в <span className="mono">~/.local/share/meshpilot/history.json</span>. Аудио не сохраняется.</p>
            <div className="row" style={{ gap: 8, marginTop: 8 }}>
              <button type="button" className="btn small danger" onClick={() => setConfirmClear(true)} disabled={historyCount === 0}>Удалить историю</button>
              <button type="button" className="btn small ghost" onClick={onOpenSettings}>Лимиты →</button>
            </div>
          </Card>
        </aside>
      </div>

      {confirmClear && (
        <Modal title="Удалить историю чата?" onClose={() => setConfirmClear(false)}
          actions={(<>
            <button type="button" className="btn" onClick={() => setConfirmClear(false)}>Отмена</button>
            <button type="button" className="btn danger" onClick={() => void clearHistory()}>Удалить</button>
          </>)}>
          <p>Будут удалены все сообщения, сохранённые на диске. Отменить нельзя.</p>
        </Modal>
      )}
    </div>
  );
}
