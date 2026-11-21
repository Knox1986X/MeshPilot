import { useEffect, useState } from 'react';
import type { PermissionInfo } from '../types';
import { api } from '../lib/api';
import { Badge, Card, Spinner, type BadgeKind } from '../components/ui';

const STATUS: Record<PermissionInfo['status'], { kind: BadgeKind; label: string }> = {
  'available': { kind: 'safe', label: 'доступно' },
  'missing-tool': { kind: 'caution', label: 'нет инструмента' },
  'needs-helper': { kind: 'muted', label: 'нужен helper' },
  'not-supported': { kind: 'risky', label: 'не поддерживается' },
};

export function Permissions() {
  const [items, setItems] = useState<PermissionInfo[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    api.permissionChecks()
      .then((v) => { if (!cancelled) setItems(v); })
      .catch((e) => { if (!cancelled) setError(e instanceof Error ? e.message : String(e)); });
    return () => { cancelled = true; };
  }, []);

  return (
    <div>
      <h1 className="section-title">Разрешения</h1>
      <p className="section-sub">Что именно требует повышения прав, зачем и как это отключить. Универсального root у приложения и у модели нет.</p>

      <Card title="Модель привилегий">
        <div className="note" style={{ marginBottom: 10 }}>
          Графический интерфейс работает от обычного пользователя. Системные действия выполняет будущий узкий helper
          (<span className="mono">meshpilot-helper</span>) через <b>polkit</b> с действиями вида
          <span className="mono"> io.github.Knox1986X.MeshPilot.&lt;action&gt;</span>: параметры и пути проверяются на привилегированной стороне,
          helper действует по закрытому allowlist, не исполняет произвольные команды и не верит UID, переданным в теле запроса.
        </div>
        <ul className="metric-sub" style={{ margin: 0, paddingLeft: 18, lineHeight: 1.9 }}>
          <li>Helper <b>не установлен</b> в этой сборке — привилегированные операции честно недоступны (см. таблицу).</li>
          <li>Процесс с вашим UID (и сама модель) не получает root-прав через MeshPilot.</li>
          <li>Отзыв прав: удалить helper, удалить polkit-правило — все привилегированные действия сразу пропадут, остальное продолжит работать.</li>
        </ul>
      </Card>

      <Card title="Состояние на этой машине" hint="проверяется backend">
        {!items && !error && <Spinner label="Проверяем инструменты и права…" />}
        {error && <div className="note err" role="alert">{error}</div>}
        {items && (
          <table className="table">
            <thead>
              <tr><th>Действие</th><th>Зачем</th><th>Права</th><th>Состояние</th></tr>
            </thead>
            <tbody>
              {items.map((p) => (
                <tr key={p.id}>
                  <td style={{ fontWeight: 600 }}>{p.title}</td>
                  <td style={{ color: 'var(--text-dim)', maxWidth: 420 }}>{p.why}</td>
                  <td>{p.needsPrivilege ? <Badge kind="caution">требуются</Badge> : <Badge kind="safe">нет</Badge>}</td>
                  <td>
                    <Badge kind={STATUS[p.status].kind}>{STATUS[p.status].label}</Badge>
                    <div style={{ fontSize: 10.5, color: 'var(--text-faint)', marginTop: 3, maxWidth: 300 }}>{p.statusDetail}</div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </Card>

      <Card title="Что никогда не делается">
        <ul className="metric-sub" style={{ margin: 0, paddingLeft: 18, lineHeight: 1.9 }}>
          <li>Не исполняется сгенерированный моделью shell. Чёрный список — не защита; есть только закрытый типизированный набор инструментов.</li>
          <li>Никаких универсальных pkexec/sudo-обёрток, беспарольного sudo и записи в произвольные пути.</li>
          <li>Текст с сайтов, файлов и вывода команд считается данными: он не может разрешить новые действия.</li>
          <li>Ключ API хранится в Secret Service/KWallet; в файлы конфигурации, журнал и frontend-сборку не попадает.</li>
        </ul>
      </Card>
    </div>
  );
}
