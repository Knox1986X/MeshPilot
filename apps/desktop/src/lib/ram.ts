import { invoke, isTauri } from "@tauri-apps/api/core";

/** Тот же объект, что status --json → ram. Не вычисляем нагрузку из данных браузера. */
export interface RamObservation {
  used_bytes: number;
  total_bytes: number;
  source: "/proc/meminfo";
  observed_at_utc: string;
}

function valid(value: unknown): value is RamObservation {
  if (!value || typeof value !== "object") return false;
  const r = value as Partial<RamObservation>;
  if (r.source !== "/proc/meminfo" || typeof r.observed_at_utc !== "string"
      || !/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/.test(r.observed_at_utc)) return false;
  const ms = Date.parse(r.observed_at_utc);
  return Number.isFinite(ms) && new Date(ms).toISOString() === r.observed_at_utc
    && typeof r.used_bytes === "number" && Number.isSafeInteger(r.used_bytes) && r.used_bytes >= 0
    && typeof r.total_bytes === "number" && Number.isSafeInteger(r.total_bytes) && r.total_bytes > 0
    && r.used_bytes <= r.total_bytes;
}

export async function readRam(): Promise<RamObservation | null> {
  if (!isTauri()) return null;
  try {
    const result = await invoke<unknown>("read_ram");
    return valid(result) ? result : null;
  } catch { return null; }
}

const units = ["Б", "КиБ", "МиБ", "ГиБ", "ТиБ", "ПиБ"] as const;
const decimal = new Intl.NumberFormat("ru-RU", { maximumFractionDigits: 2 });
export function formatBytes(bytes: number): string {
  let n = bytes, i = 0;
  while (n >= 1024 && i < units.length - 1) { n /= 1024; i++; }
  return `${decimal.format(n)} ${units[i]}`;
}
