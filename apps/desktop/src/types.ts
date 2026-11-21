// Общие типы IPC между frontend и Rust-backend.
// Имена полей camelCase — backend сериализует с rename_all = "camelCase".

export interface DiskInfo {
  name: string;        // устройство или ФС (источник statvfs)
  mountPoint: string;  // точка монтирования
  fsType: string | null;
  totalBytes: number;
  usedBytes: number;
  availableBytes: number;
  dedupedFrom: string[]; // другие точки монтирования той же ФС (не учитываются повторно)
}

export interface Snapshot {
  cpuTotalPercent: number | null;
  cpuPerCore: (number | null)[];
  memoryUsedBytes: number | null;
  memoryTotalBytes: number | null;
  swapUsedBytes: number | null;
  swapTotalBytes: number | null;
  diskUsedBytes: number | null;   // сумма по уникальным ФС
  diskTotalBytes: number | null;
  disks: DiskInfo[];
  networkRxBytesPerSec: number | null;
  networkTxBytesPerSec: number | null;
  temperatureCelsius: number | null;
  uptimeSeconds: number | null;
  hostname: string | null;
  kernel: string | null;
  osName: string | null;
  desktop: string | null;
  updatedAtUtc: string;
  partial: string[];   // человекочитаемый список недоступных датчиков
}

// ---- Очистка ----

export type RiskLevel = 'safe' | 'caution' | 'risky';

export interface CleanupCategory {
  id: string;
  name: string;
  description: string;
  risk: RiskLevel;
  defaultSelected: boolean;
  requiresPrivilege: boolean;
  requiresOptIn: boolean; // категория, которую можно включить только явно (корзина, shader cache, flatpak)
  paths: string[];        // обнаруженные пути (для отчёта)
  estimatedBytes: number | null;
  available: boolean;     // false, если цель не найдена на этой машине
  blockedReason: string | null; // например, «приложение запущено»
}

export interface CleanupReport {
  categories: CleanupCategory[];
  totalEstimatedBytes: number;
  runningApps: string[];
  warnings: string[];
  analyzedAtUtc: string;
}

export interface CleanupApplyRequest {
  categoryIds: string[];
  dryRun: boolean;
  confirmRisky: boolean; // оптин-флаг для risky/optIn категорий
}

export interface CleanupApplyResult {
  freedBytes: number;
  results: { categoryId: string; freedBytes: number; ok: boolean; message: string }[];
  report: CleanupReport | null; // повторный анализ после очистки
}

// ---- Зеркала ----

export interface MirrorsStatus {
  osName: string | null;
  isArchLike: boolean;
  detectedTools: string[];      // найденные: cachyos-rate-mirrors, rate-mirrors, reflector
  mirrorlistPath: string | null;
  mirrorlistExists: boolean;
  lastBackup: string | null;
  pacmanDbCount: number | null;
}

export interface OpResult {
  ok: boolean;
  message: string;
  backupPath?: string | null;
}

// ---- DNS ----

export type DnsManager = 'networkmanager' | 'systemd-resolved' | 'resolv-conf' | 'unknown';

export interface DnsStatus {
  manager: DnsManager;
  managerDetail: string;
  currentServers: string[];
  dnssec: string | null;
  dnsOverTls: string | null;
  providerHint: string | null;
  lastBackup: string | null;
  needsPrivilege: boolean;
}

export interface DnsCandidate {
  name: string;
  address: string;
  tlsName: string | null;
  medianMs: number | null;
  okCount: number;
  failCount: number;
  filtered: boolean; // блокирует/фильтрует вредоносные домены (Quad9/AdGuard)
}

export interface DnsBenchmarkResult {
  candidates: DnsCandidate[];
  testedAtUtc: string;
  domains: string[];
  note: string | null;
}

// ---- Обход (byedpi/zapret) ----

export interface BypassStatus {
  ciadpiFound: boolean;
  zapretFound: boolean;
  activeService: boolean;
  port: number;
  checkUrls: string[];
  lastProfile: string | null;
}

export interface BypassStrategyResult {
  strategy: string[];
  label: string;
  passed: number;
  total: number;
  works: boolean;
}

export interface BypassTestResult {
  direct: { url: string; ok: boolean }[];
  strategies: BypassStrategyResult[];
  testedAtUtc: string;
}

// ---- Расписания ----

export interface ScheduleInfo {
  id: string;
  name: string;
  description: string;
  enabled: boolean;
  calendar: string;
  lastRun: string | null;
  lastResult: string | null;
  nextRun: string | null;
  mode: 'report' | 'apply';
  supported: boolean;
  unsupportedReason: string | null;
}

// ---- LLM / провайдеры ----

export type ProviderId = 'groq' | 'openrouter';

export interface ProviderStatus {
  provider: ProviderId;
  hasKey: boolean;
  storage: 'keyring' | 'session' | 'none';
  keyringAvailable: boolean;
}

export interface ConnectionTestResult {
  ok: boolean;
  code:
    | 'ok'
    | 'no-network'
    | 'invalid-key'
    | 'rate-limited'
    | 'model-unavailable'
    | 'server-error'
    | 'timeout'
    | 'unknown';
  message: string;
  latencyMs: number | null;
}

export interface Settings {
  schemaVersion: number;
  assistant: {
    provider: ProviderId;
    model: string;
    backupEnabled: boolean;
    backupProvider: ProviderId | null;
    backupModel: string | null;
    timeoutSeconds: number;
    maxTokens: number;
    temperature: number;
    sirAddressing: boolean;
    userName: string;
  };
  jarvis: {
    saveHistory: boolean;
    historyLimit: number;
    wakeWord: boolean;
    pushToTalkHotkey: string;
  };
  cleanup: {
    safeProfileOnly: boolean;
    includeTrash: boolean;
    keepPacmanVersions: number;
    journalMaxSizeMb: number;
  };
  network: {
    bypassPort: number;
    checkUrls: string[];
  };
  ui: {
    demoMode: boolean;
  };
}

// ---- Джарвис / чат ----

export interface ToolCallInfo {
  tool: string;
  label: string;
  ok: boolean;
  detail: string;
}

export interface ChatMessage {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  createdAtUtc: string;
  toolCalls: ToolCallInfo[];
  error: string | null;
}

export interface JarvisSendRequest {
  messages: { role: 'user' | 'assistant'; content: string }[];
}

export interface JarvisSendResult {
  reply: ChatMessage;
}

export interface PermissionInfo {
  id: string;
  title: string;
  why: string;
  needsPrivilege: boolean;
  status: 'available' | 'missing-tool' | 'not-supported' | 'needs-helper';
  statusDetail: string;
}
