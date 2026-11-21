// Форматирование значений для интерфейса. Чистые функции — покрыты тестами.

const KIB = 1024;
const MIB = KIB * 1024;
const GIB = MIB * 1024;
const TIB = GIB * 1024;

const BYTE_UNITS: Array<[number, string]> = [
  [TIB, 'ТБ'],
  [GIB, 'ГБ'],
  [MIB, 'МБ'],
  [KIB, 'КБ'],
  [1, 'Б'],
];

/** Человекочитаемый размер; отрицательные/NaN → «—». */
export function formatBytes(bytes: number | null | undefined, digits = 1): string {
  if (bytes == null || !Number.isFinite(bytes) || bytes < 0) return '—';
  for (const [unit, label] of BYTE_UNITS) {
    if (bytes >= unit) return `${(bytes / unit).toFixed(unit === 1 ? 0 : digits)} ${label}`;
  }
  return `${bytes.toFixed(0)} Б`;
}

/** Скорость сети, Б/с. */
export function formatRate(bytesPerSec: number | null | undefined): string {
  if (bytesPerSec == null || !Number.isFinite(bytesPerSec) || bytesPerSec < 0) return '—';
  return `${formatBytes(bytesPerSec, bytesPerSec >= KIB ? 1 : 0)}/с`;
}

export function formatPercent(value: number | null | undefined): string {
  if (value == null || !Number.isFinite(value)) return '—';
  return `${value.toFixed(value >= 10 ? 0 : 1)}%`;
}

/** Аптайм: «3 д 4 ч» / «5 ч 12 мин» / «8 мин». */
export function formatUptime(seconds: number | null | undefined): string {
  if (seconds == null || !Number.isFinite(seconds) || seconds < 0) return '—';
  const s = Math.floor(seconds);
  const days = Math.floor(s / 86400);
  const hours = Math.floor((s % 86400) / 3600);
  const minutes = Math.floor((s % 3600) / 60);
  if (days > 0) return `${days} д ${hours} ч`;
  if (hours > 0) return `${hours} ч ${minutes} мин`;
  return `${minutes} мин`;
}

/** «12.11.2026, 09:41» — локаль ru-RU. */
export function formatDateTime(iso: string | null | undefined): string {
  if (!iso) return '—';
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return '—';
  return d.toLocaleString('ru-RU', {
    day: '2-digit',
    month: '2-digit',
    year: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

export function formatClock(d: Date): string {
  return d.toLocaleTimeString('ru-RU', { hour: '2-digit', minute: '2-digit', second: '2-digit' });
}

/** Моноширинные цифры для таблицы: добивает ведущими пробелами не нужно — UI делает tabular-nums. */
export function clamp(n: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, n));
}

/** Путь-кандидат должен находиться внутри домашнего каталога (защита UI-логики). */
export function looksLikeSafeHomePath(path: string): boolean {
  if (!path) return false;
  const normalized = path.replace(/\\/g, '/');
  if (!normalized.startsWith('/')) return false;
  if (normalized.includes('/../') || normalized.endsWith('/..')) return false;
  return true;
}
