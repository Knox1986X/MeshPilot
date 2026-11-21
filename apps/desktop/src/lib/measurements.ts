import { invoke, isTauri } from "@tauri-apps/api/core";

export type Metric = "cpu" | "ram" | "disk" | "net";

/** Те же объекты, что status --json → cpu, ram, root_fs и network_io. Браузер не является источником метрик. */
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

export interface RootFsObservation {
  used_bytes: number;
  total_bytes: number;
  available_bytes: number;
  source: "statvfs(/)";
  observed_at_utc: string;
}

export interface TransferRates {
  rx_delta_bytes: number;
  tx_delta_bytes: number;
  rx_bytes_per_sec: number;
  tx_bytes_per_sec: number;
}
export interface InterfaceIo { name: string; rates: TransferRates | null }
export interface NetworkIoObservation {
  interfaces: InterfaceIo[];
  interval_ms: number;
  source: "/proc/net/dev";
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
function validRootFs(value: unknown): value is RootFsObservation {
  if (!value || typeof value !== "object") return false;
  const s = value as Partial<RootFsObservation>;
  return s.source === "statvfs(/)" && utcStamp(s.observed_at_utc)
    && whole(s.total_bytes) && s.total_bytes > 0 && whole(s.used_bytes) && s.used_bytes <= s.total_bytes
    && whole(s.available_bytes) && s.available_bytes <= s.total_bytes - s.used_bytes;
}
function interval(ms: unknown): ms is number { return whole(ms) && ms >= 500 && ms <= 5000; }
function validNetworkIo(value: unknown): value is NetworkIoObservation {
  if (!value || typeof value !== "object") return false;
  const n = value as Partial<NetworkIoObservation>;
  if (n.source !== "/proc/net/dev" || !utcStamp(n.observed_at_utc) || !interval(n.interval_ms)
    || !Array.isArray(n.interfaces) || n.interfaces.length > 256) return false;
  const names = new Set<string>(), perSec = 1000 / n.interval_ms;
  const matches = (rate: unknown, delta: unknown) => {
    if (!whole(delta) || typeof rate !== "number" || !Number.isFinite(rate) || rate < 0) return false;
    const expected = delta * perSec;
    return Math.abs(rate - expected) <= Number.EPSILON * expected * 2;
  };
  for (const dev of n.interfaces) {
    if (!dev || typeof dev.name !== "string" || dev.name.length < 1 || dev.name.length > 15
      || /[^!-~]|[/:]/.test(dev.name) || dev.name === "." || dev.name === ".." || names.has(dev.name)) return false;
    names.add(dev.name);
    const r = dev.rates;
    if (r !== null && (!r || !matches(r.rx_bytes_per_sec, r.rx_delta_bytes) || !matches(r.tx_bytes_per_sec, r.tx_delta_bytes))) return false;
  }
  return true;
}
function validCpu(value: unknown): value is CpuObservation {
  if (!value || typeof value !== "object") return false;
  const c = value as Partial<CpuObservation>;
  return c.source === "/proc/stat" && utcStamp(c.observed_at_utc)
    && interval(c.interval_ms)
    && whole(c.total_ticks) && c.total_ticks > 0 && whole(c.busy_ticks) && c.busy_ticks <= c.total_ticks
    && typeof c.usage_percent === "number" && c.usage_percent >= 0 && c.usage_percent <= 100
    && Math.abs(c.usage_percent - 100 * (c.busy_ticks / c.total_ticks)) < 1e-9;
}
export type ReadIssue = "browser" | "unavailable" | "invalid" | "ipc";
export type Reading<T> = { value: T; issue: null } | { value: null; issue: ReadIssue };
async function read<T>(cmd: "read_ram" | "read_cpu" | "read_root_fs" | "read_network_io", valid: (value: unknown) => value is T): Promise<Reading<T>> {
  if (!isTauri()) return { value: null, issue: "browser" };
  try {
    const result = await invoke<unknown>(cmd);
    if (result === null) return { value: null, issue: "unavailable" };
    return valid(result) ? { value: result, issue: null } : { value: null, issue: "invalid" };
  } catch { return { value: null, issue: "ipc" }; }
}
export const readCpu = () => read("read_cpu", validCpu);
export const readRootFs = () => read("read_root_fs", validRootFs);
export const readNetworkIo = () => read("read_network_io", validNetworkIo);
export const readRam = () => read("read_ram", validRam);

const issues = { browser: "В браузере источник недоступен", ipc: "Канал недоступен", invalid: "Некорректный ответ", unavailable: "Нет достоверного замера" };
export function readHint(issue: ReadIssue | null, pending: boolean, source: string) {
  return !isTauri() ? issues.browser : `${pending ? "Чтение" : issues[issue ?? "unavailable"]}: ${source}`;
}
export function formatRate(n: number) { return `${formatBytes(n)}/с`; }

// UTC из проверенного ответа, не текущее время WebView.
export function formatUtc(stamp: string): string { return `${stamp.slice(0, 10)} ${stamp.slice(11, 19)} UTC`; }

const units = ["Б", "КиБ", "МиБ", "ГиБ", "ТиБ", "ПиБ"] as const;
const decimal = new Intl.NumberFormat("ru-RU", { maximumFractionDigits: 2 });
export function formatBytes(bytes: number): string {
  let n = bytes, i = 0;
  while (n >= 1024 && i < units.length - 1) { n /= 1024; i++; }
  return `${decimal.format(n)} ${units[i]}`;
}
const tenth = new Intl.NumberFormat("ru-RU", { maximumFractionDigits: 1 });
export function formatPercent(p: number): string { return `${tenth.format(p)} %`; }
