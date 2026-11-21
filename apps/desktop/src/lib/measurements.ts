import { invoke, isTauri } from "@tauri-apps/api/core";

export type Metric = "cpu" | "ram";

/** Те же объекты, что status --json → cpu и ram. Браузер не является источником метрик. */
export interface CpuObservation {
  usage_percent: number;
  busy_ticks: number;
  total_ticks: number;
  interval_ms: number;
  source: "/proc/stat";
  observed_at_utc: string;
}
export interface RamObservation {
  used_bytes: number;
  total_bytes: number;
  source: "/proc/meminfo";
  observed_at_utc: string;
}

function utcStamp(s: unknown): s is string {
  if (typeof s !== "string" || !/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/.test(s)) return false;
  const ms = Date.parse(s);
  return Number.isFinite(ms) && new Date(ms).toISOString() === s;
}
function whole(n: unknown): n is number { return typeof n === "number" && Number.isSafeInteger(n) && n >= 0; }

function validRam(value: unknown): value is RamObservation {
  if (!value || typeof value !== "object") return false;
  const r = value as Partial<RamObservation>;
  return r.source === "/proc/meminfo" && utcStamp(r.observed_at_utc) && whole(r.used_bytes)
    && whole(r.total_bytes) && r.total_bytes > 0 && r.used_bytes <= r.total_bytes;
}
function validCpu(value: unknown): value is CpuObservation {
  if (!value || typeof value !== "object") return false;
  const c = value as Partial<CpuObservation>;
  return c.source === "/proc/stat" && utcStamp(c.observed_at_utc)
    && whole(c.interval_ms) && c.interval_ms >= 500 && c.interval_ms <= 5000
    && whole(c.total_ticks) && c.total_ticks > 0 && whole(c.busy_ticks) && c.busy_ticks <= c.total_ticks
    && typeof c.usage_percent === "number" && c.usage_percent >= 0 && c.usage_percent <= 100
    && Math.abs(c.usage_percent - 100 * (c.busy_ticks / c.total_ticks)) < 1e-9;
}
export type ReadIssue = "browser" | "unavailable" | "invalid" | "ipc";
export type Reading<T> = { value: T; issue: null } | { value: null; issue: ReadIssue };
async function read<T>(cmd: "read_ram" | "read_cpu", valid: (value: unknown) => value is T): Promise<Reading<T>> {
  if (!isTauri()) return { value: null, issue: "browser" };
  try {
    const result = await invoke<unknown>(cmd);
    if (result === null) return { value: null, issue: "unavailable" };
    return valid(result) ? { value: result, issue: null } : { value: null, issue: "invalid" };
  } catch { return { value: null, issue: "ipc" }; }
}
export const readCpu = () => read("read_cpu", validCpu);
export const readRam = () => read("read_ram", validRam);

const units = ["Б", "КиБ", "МиБ", "ГиБ", "ТиБ", "ПиБ"] as const;
const decimal = new Intl.NumberFormat("ru-RU", { maximumFractionDigits: 2 });
export function formatBytes(bytes: number): string {
  let n = bytes, i = 0;
  while (n >= 1024 && i < units.length - 1) { n /= 1024; i++; }
  return `${decimal.format(n)} ${units[i]}`;
}
const tenth = new Intl.NumberFormat("ru-RU", { maximumFractionDigits: 1 });
export function formatPercent(p: number): string { return `${tenth.format(p)} %`; }
