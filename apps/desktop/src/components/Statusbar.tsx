import type { Snapshot } from '../types';

interface Props {
  snapshot: Snapshot | null;
  loading: boolean;
  apiConnected: boolean | null; // null — ещё не проверяли
  demoMode: boolean;
}

export function Statusbar({ snapshot, loading, apiConnected, demoMode }: Props) {
  const dot = apiConnected == null ? 'idle' : apiConnected ? 'ok' : 'err';
  const label = apiConnected == null ? 'Помощник: не проверен' : apiConnected ? 'Помощник: Groq подключён' : 'Помощник: недоступен';
  return (
    <footer className="statusbar" aria-label="Строка состояния">
      <span><span className={`status-dot ${dot}`} aria-hidden="true" />{label}</span>
      <span aria-hidden="true">|</span>
      <span>Backend: {snapshot ? 'в норме' : loading ? 'опрос…' : 'нет данных'}</span>
      {snapshot?.partial && snapshot.partial.length > 0 && (
        <>
          <span aria-hidden="true">|</span>
          <span title={snapshot.partial.join('\n')}>Датчики: частично ({snapshot.partial.length})</span>
        </>
      )}
      <span aria-hidden="true">|</span>
      <span>{demoMode ? 'ДЕМО-РЕЖИМ · данные синтетические' : 'локальные данные'}</span>
      <span style={{ marginLeft: 'auto' }}>MeshPilot · без телеметрии</span>
    </footer>
  );
}
