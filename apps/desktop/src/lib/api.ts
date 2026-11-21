// Слой доступа к backend. В Tauri вызывает команды; вне Tauri — явно помеченный
// демо-провайдер (синтетические данные, в интерфейсе горит бейдж «ДЕМО»).
import type {
  BypassStatus,
  BypassTestResult,
  ChatMessage,
  CleanupApplyRequest,
  CleanupApplyResult,
  CleanupReport,
  ConnectionTestResult,
  DnsBenchmarkResult,
  DnsStatus,
  JarvisSendRequest,
  JarvisSendResult,
  MirrorsStatus,
  OpResult,
  PermissionInfo,
  ProviderId,
  ProviderStatus,
  ScheduleInfo,
  Settings,
  Snapshot,
} from '../types';

export const isTauri =
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export const isDemo = !isTauri;

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri) throw new Error(`Команда «${cmd}» доступна только в приложении Tauri`);
  const { invoke: tauriInvoke } = await import('@tauri-apps/api/core');
  return tauriInvoke<T>(cmd, args);
}

// ---------------- демо-данные (только вне Tauri) ----------------

const demoSnapshot = (): Snapshot => ({
  cpuTotalPercent: 34,
  cpuPerCore: [41, 28, 33, 12, 52, 9, 38, 21],
  memoryUsedBytes: 9.6 * 1024 ** 3,
  memoryTotalBytes: 31.2 * 1024 ** 3,
  swapUsedBytes: 0,
  swapTotalBytes: 16 * 1024 ** 3,
  diskUsedBytes: 612 * 1024 ** 3,
  diskTotalBytes: 931 * 1024 ** 3,
  disks: [
    {
      name: 'demo0s1',
      mountPoint: '/',
      fsType: 'btrfs',
      totalBytes: 931 * 1024 ** 3,
      usedBytes: 612 * 1024 ** 3,
      availableBytes: 319 * 1024 ** 3,
      dedupedFrom: ['/home', '/var', '/.snapshots'],
    },
  ],
  networkRxBytesPerSec: 284 * 1024,
  networkTxBytesPerSec: 61 * 1024,
  temperatureCelsius: null,
  uptimeSeconds: 3 * 86400 + 7 * 3600 + 12 * 60,
  hostname: 'demo-cachyos',
  kernel: '6.x-cachyos',
  osName: 'CachyOS (демо)',
  desktop: 'KDE Plasma (Wayland) (демо)',
  updatedAtUtc: new Date().toISOString(),
  partial: ['датчик температуры (демо)'],
});

const demoCleanup = (): CleanupReport => ({
  categories: [
    { id: 'pacman-cache', name: 'Кэш pacman', description: 'Старые версии пакетов в /var/cache/pacman/pkg (останется N последних). Требуются права.', risk: 'safe', defaultSelected: true, requiresPrivilege: true, requiresOptIn: false, paths: ['/var/cache/pacman/pkg'], estimatedBytes: null, available: false, blockedReason: 'демонстрация без backend' },
    { id: 'thumbnails', name: 'Миниатюры', description: '~/.cache/thumbnails — пересоздаются автоматически', risk: 'safe', defaultSelected: true, requiresPrivilege: false, requiresOptIn: false, paths: ['~/.cache/thumbnails'], estimatedBytes: 184 * 1024 ** 2, available: true, blockedReason: null },
    { id: 'browser-cache', name: 'Кэши браузеров', description: 'Firefox / Floorp / Chromium: только cache2 и Service Worker CacheStorage', risk: 'safe', defaultSelected: true, requiresPrivilege: false, requiresOptIn: false, paths: ['~/.cache/mozilla', '~/.mozilla/firefox/*/cache2', '~/.cache/chromium'], estimatedBytes: 2.1 * 1024 ** 3, available: true, blockedReason: null },
    { id: 'messaging-cache', name: 'Кэши мессенджеров', description: 'Telegram, AyuGram, Discord, Vesktop — только кэш, не данные и не история', risk: 'caution', defaultSelected: false, requiresPrivilege: false, requiresOptIn: false, paths: ['~/.local/share/TelegramDesktop/tdata/user_data/cache', '~/.config/discord/Cache'], estimatedBytes: 890 * 1024 ** 2, available: true, blockedReason: null },
    { id: 'dev-caches', name: 'Кэши разработчика', description: 'pip, npm, pnpm, go-build, cargo registry — воспроизводимые', risk: 'safe', defaultSelected: true, requiresPrivilege: false, requiresOptIn: false, paths: ['~/.cache/pip', '~/.npm/_cacache', '~/.cache/go-build'], estimatedBytes: 1.4 * 1024 ** 3, available: true, blockedReason: null },
    { id: 'trash', name: 'Корзина', description: '~/.local/share/Trash — полное удаление, восстановить будет нельзя', risk: 'risky', defaultSelected: false, requiresPrivilege: false, requiresOptIn: true, paths: ['~/.local/share/Trash'], estimatedBytes: 3.2 * 1024 ** 3, available: true, blockedReason: null },
    { id: 'steam-shaders', name: 'Shader-кэш Steam/Mesa', description: 'Пересоздаётся при следующем запуске игр (первый запуск медленнее)', risk: 'caution', defaultSelected: false, requiresPrivilege: false, requiresOptIn: true, paths: ['~/.local/share/Steam/steamapps/shadercache', '~/.cache/mesa_shader_cache'], estimatedBytes: 5.7 * 1024 ** 3, available: true, blockedReason: null },
    { id: 'journal-coredump', name: 'journald / coredumps', description: 'Ограничение возраста и размера журнала; старше N дней удалятся', risk: 'caution', defaultSelected: false, requiresPrivilege: true, requiresOptIn: false, paths: ['/var/log/journal', '/var/lib/systemd/coredump'], estimatedBytes: null, available: false, blockedReason: 'требуется helper (см. «Разрешения»)' },
    { id: 'aur-helpers', name: 'Кэш yay/paru', description: '~/.cache/yay, ~/.cache/paru — скачанные PKGBUILD и пакеты', risk: 'safe', defaultSelected: true, requiresPrivilege: false, requiresOptIn: false, paths: ['~/.cache/yay', '~/.cache/paru'], estimatedBytes: 640 * 1024 ** 2, available: true, blockedReason: null },
  ],
  totalEstimatedBytes: 13.6 * 1024 ** 3,
  runningApps: ['firefox'],
  warnings: ['Firefox запущен — его кэш может быть пропущен или удалён небезопасно.'],
  analyzedAtUtc: new Date().toISOString(),
});

const demoSettings = (): Settings => JSON.parse(JSON.stringify({
  schemaVersion: 1,
  assistant: {
    provider: 'groq',
    model: 'llama-3.3-70b-versatile',
    backupEnabled: false,
    backupProvider: null,
    backupModel: null,
    timeoutSeconds: 30,
    maxTokens: 1024,
    temperature: 0.4,
    sirAddressing: true,
    userName: '',
  },
  jarvis: { saveHistory: true, historyLimit: 200, wakeWord: false, pushToTalkHotkey: 'Ctrl+Space' },
  cleanup: { safeProfileOnly: true, includeTrash: false, keepPacmanVersions: 2, journalMaxSizeMb: 500 },
  network: { bypassPort: 18080, checkUrls: ['https://www.youtube.com/generate_204', 'https://discord.com/robots.txt'] },
  ui: { demoMode: true },
})) as Settings;

let demoSettingsCache = demoSettings();

const demoJarvisReply: ChatMessage = {
  id: 'demo-reply',
  role: 'assistant',
  content:
    'Это демо-режим: backend Tauri не подключён, поэтому я не могу реально отвечать. ' +
    'Запустите приложение через `npm run tauri dev` на CachyOS и сохраните ключ Groq в «Настройки → Помощник → Подключения».',
  createdAtUtc: new Date().toISOString(),
  toolCalls: [],
  error: null,
};

// ---------------- API ----------------

export const api = {
  isDemo,

  async getSnapshot(): Promise<Snapshot> {
    if (isDemo) return { ...demoSnapshot(), updatedAtUtc: new Date().toISOString() };
    return invoke<Snapshot>('get_snapshot');
  },

  async cleanupAnalyze(): Promise<CleanupReport> {
    if (isDemo) return { ...demoCleanup(), analyzedAtUtc: new Date().toISOString() };
    return invoke<CleanupReport>('cleanup_analyze');
  },

  async cleanupApply(req: CleanupApplyRequest): Promise<CleanupApplyResult> {
    if (isDemo) {
      return { freedBytes: 0, results: req.categoryIds.map((id) => ({ categoryId: id, freedBytes: 0, ok: false, message: 'Демо-режим: ничего не удалялось' })), report: null };
    }
    return invoke<CleanupApplyResult>('cleanup_apply', { req });
  },

  async mirrorsStatus(): Promise<MirrorsStatus> {
    if (isDemo) {
      return { osName: 'CachyOS (демо)', isArchLike: true, detectedTools: ['rate-mirrors'], mirrorlistPath: '/etc/pacman.d/mirrorlist', mirrorlistExists: true, lastBackup: null, pacmanDbCount: 1523 };
    }
    return invoke<MirrorsStatus>('mirrors_status');
  },

  async mirrorsApply(): Promise<OpResult> {
    if (isDemo) return { ok: false, message: 'Демо-режим: зеркала не менялись' };
    return invoke<OpResult>('mirrors_apply', { confirm: true });
  },

  async dnsStatus(): Promise<DnsStatus> {
    if (isDemo) {
      return { manager: 'systemd-resolved', managerDetail: 'демо', currentServers: ['1.1.1.1', '9.9.9.9'], dnssec: 'allow-downgrade', dnsOverTls: 'opportunistic', providerHint: 'Cloudflare / Quad9 (демо)', lastBackup: null, needsPrivilege: true };
    }
    return invoke<DnsStatus>('dns_status');
  },

  async dnsBenchmark(): Promise<DnsBenchmarkResult> {
    if (isDemo) {
      return {
        candidates: [
          { name: 'Cloudflare', address: '1.1.1.1', tlsName: 'cloudflare-dns.com', medianMs: 8.4, okCount: 9, failCount: 0, filtered: false },
          { name: 'Quad9', address: '9.9.9.9', tlsName: 'dns.quad9.net', medianMs: 11.2, okCount: 9, failCount: 0, filtered: true },
          { name: 'Google', address: '8.8.8.8', tlsName: 'dns.google', medianMs: 12.8, okCount: 9, failCount: 0, filtered: false },
          { name: 'AdGuard', address: '94.140.14.14', tlsName: 'dns.adguard-dns.com', medianMs: 15.1, okCount: 8, failCount: 1, filtered: true },
        ],
        testedAtUtc: new Date().toISOString(),
        domains: ['google.com', 'wikipedia.org', 'github.com'],
        note: 'Демо-данные: backend недоступен',
      };
    }
    return invoke<DnsBenchmarkResult>('dns_benchmark');
  },

  async dnsApply(): Promise<OpResult> {
    if (isDemo) return { ok: false, message: 'Демо-режим: DNS не менялись' };
    return invoke<OpResult>('dns_apply', { confirm: true });
  },

  async dnsRollback(): Promise<OpResult> {
    if (isDemo) return { ok: false, message: 'Демо-режим: откат не выполнялся' };
    return invoke<OpResult>('dns_rollback', { confirm: true });
  },

  async bypassStatus(): Promise<BypassStatus> {
    if (isDemo) {
      return { ciadpiFound: false, zapretFound: false, activeService: false, port: 18080, checkUrls: ['https://www.youtube.com/generate_204'], lastProfile: null };
    }
    return invoke<BypassStatus>('bypass_status');
  },

  async bypassTest(urls: string[]): Promise<BypassTestResult> {
    if (isDemo) {
      return { direct: urls.map((url) => ({ url, ok: false })), strategies: [], testedAtUtc: new Date().toISOString() };
    }
    return invoke<BypassTestResult>('bypass_test', { urls });
  },

  async bypassEnable(): Promise<OpResult> {
    if (isDemo) return { ok: false, message: 'Демо-режим: прокси не включался' };
    return invoke<OpResult>('bypass_enable', { confirm: true });
  },

  async bypassDisable(): Promise<OpResult> {
    if (isDemo) return { ok: false, message: 'Демо-режим: прокси не выключался' };
    return invoke<OpResult>('bypass_disable', { confirm: true });
  },

  async schedulesList(): Promise<ScheduleInfo[]> {
    if (isDemo) {
      return [
        { id: 'mirrors-daily', name: 'Проверка зеркал', description: 'Ежедневно: ранжирование кандидатов, отчёт или применение по настройке', enabled: false, calendar: 'ежедневно 05:00', lastRun: null, lastResult: null, nextRun: null, mode: 'report', supported: true, unsupportedReason: null },
        { id: 'dns-daily', name: 'Проверка DNS', description: 'Ежедневно: бенчмарк и отчёт; применение только при улучшении выше порога', enabled: false, calendar: 'ежедневно 05:15', lastRun: null, lastResult: null, nextRun: null, mode: 'report', supported: true, unsupportedReason: null },
        { id: 'net-weekly', name: 'Сетевой профиль', description: 'Еженедельно: проверка доступности и стратегии обхода', enabled: false, calendar: 'вс 05:20', lastRun: null, lastResult: null, nextRun: null, mode: 'report', supported: true, unsupportedReason: null },
        { id: 'cleanup-weekly', name: 'Безопасная очистка', description: 'Еженедельно: очистка safe-профиля (кэши, без корзины и рискованных категорий)', enabled: false, calendar: 'вс 06:00', lastRun: null, lastResult: null, nextRun: null, mode: 'report', supported: true, unsupportedReason: null },
      ];
    }
    return invoke<ScheduleInfo[]>('schedules_list');
  },

  async scheduleSet(id: string, enabled: boolean): Promise<OpResult> {
    if (isDemo) return { ok: false, message: 'Демо-режим: таймеры не менялись' };
    return invoke<OpResult>('schedule_set', { id, enabled });
  },

  async permissionChecks(): Promise<PermissionInfo[]> {
    if (isDemo) {
      return [
        { id: 'paccache', title: 'Обновление кэша pacman', why: 'paccache требует прав для уменьшения /var/cache/pacman/pkg', needsPrivilege: true, status: 'missing-tool', statusDetail: 'демо: paccache не обнаружен' },
        { id: 'resolved-write', title: 'Применение DNS (systemd-resolved)', why: 'Запись drop-in в /etc/systemd/resolved.conf.d требует root + restart сервиса', needsPrivilege: true, status: 'needs-helper', statusDetail: 'привилегированный helper не установлен (этап «Разрешения»)' },
        { id: 'mirrorlist', title: 'Применение зеркал', why: 'Замена /etc/pacman.d/mirrorlist требует root; выполняется через pacman-хелперы', needsPrivilege: true, status: 'needs-helper', statusDetail: 'привилегированный helper не установлен (этап «Разрешения»)' },
        { id: 'open-apps', title: 'Открытие приложений Джарвисом', why: 'Запуск разрешённых приложений из allowlist, без shell', needsPrivilege: false, status: 'available', statusDetail: 'работает без повышения прав' },
        { id: 'user-caches', title: 'Очистка пользовательских кэшей', why: 'Файлы в $HOME, права владельца', needsPrivilege: false, status: 'available', statusDetail: 'работает без повышения прав' },
      ];
    }
    return invoke<PermissionInfo[]>('permission_checks');
  },

  // ---- LLM / ключи ----
  async providerStatus(provider: ProviderId): Promise<ProviderStatus> {
    if (isDemo) return { provider, hasKey: false, storage: 'none', keyringAvailable: false };
    return invoke<ProviderStatus>('llm_provider_status', { provider });
  },

  async saveKey(provider: ProviderId, key: string): Promise<{ ok: boolean; storage: 'keyring' | 'session'; message: string }> {
    if (isDemo) return { ok: false, storage: 'session', message: 'Демо-режим: ключ не сохранён' };
    return invoke('llm_save_key', { provider, key });
  },

  async deleteKey(provider: ProviderId): Promise<{ ok: boolean; message: string }> {
    if (isDemo) return { ok: false, message: 'Демо-режим: ключ не удалён' };
    return invoke('llm_delete_key', { provider });
  },

  async testConnection(provider: ProviderId, model: string): Promise<ConnectionTestResult> {
    if (isDemo) return { ok: false, code: 'no-network', message: 'Демо-режим: сеть backend недоступна', latencyMs: null };
    return invoke<ConnectionTestResult>('llm_test', { provider, model });
  },

  async listModels(provider: ProviderId): Promise<string[]> {
    if (isDemo) {
      return provider === 'groq'
        ? ['llama-3.3-70b-versatile', 'llama-3.1-8b-instant', 'openai/gpt-oss-120b', 'openai/gpt-oss-20b', 'qwen/qwen3-32b', 'meta-llama/llama-4-scout-17b-16e-instruct']
        : ['openai/gpt-4o-mini', 'anthropic/claude-3.5-haiku'];
    }
    return invoke<string[]>('llm_list_models', { provider });
  },

  // ---- Джарвис ----
  async jarvisSend(request: JarvisSendRequest): Promise<JarvisSendResult> {
    if (isDemo) {
      return { reply: { ...demoJarvisReply, createdAtUtc: new Date().toISOString() } };
    }
    return invoke<JarvisSendResult>('jarvis_send', { req: request });
  },

  async historyLoad(): Promise<ChatMessage[]> {
    if (isDemo) return [];
    return invoke<ChatMessage[]>('history_load');
  },

  async historySave(messages: ChatMessage[]): Promise<void> {
    if (isDemo) return;
    return invoke<void>('history_save', { messages });
  },

  async historyClear(): Promise<{ ok: boolean; message: string }> {
    if (isDemo) return { ok: true, message: 'Демо-режим: история и так пуста' };
    return invoke('history_clear');
  },

  // ---- Настройки ----
  async settingsGet(): Promise<Settings> {
    if (isDemo) return JSON.parse(JSON.stringify(demoSettingsCache)) as Settings;
    return invoke<Settings>('settings_get');
  },

  async settingsUpdate(patch: Partial<Settings>): Promise<Settings> {
    if (isDemo) {
      demoSettingsCache = { ...demoSettingsCache, ...patch } as Settings;
      return JSON.parse(JSON.stringify(demoSettingsCache)) as Settings;
    }
    return invoke<Settings>('settings_update', { patch });
  },

  async settingsReset(): Promise<Settings> {
    if (isDemo) {
      demoSettingsCache = demoSettings();
      return JSON.parse(JSON.stringify(demoSettingsCache)) as Settings;
    }
    return invoke<Settings>('settings_reset');
  },
};

export type Api = typeof api;
