<script lang="ts">
  import { RefreshCw, Server } from "@lucide/svelte";
  import { isTauri } from "@tauri-apps/api/core";
  import { formatUptime, formatUtc, readHint, type Reading, type UptimeObservation, type KernelObservation, type OsVersionObservation } from "./measurements";
  let { uptime, kernel, osVersion, pendingUptime, pendingKernel, pendingOsVersion, refreshUptime, refreshKernel, refreshOsVersion }: {
    uptime: Reading<UptimeObservation>; kernel: Reading<KernelObservation>; osVersion: Reading<OsVersionObservation>;
    pendingUptime: boolean; pendingKernel: boolean; pendingOsVersion: boolean;
    refreshUptime: () => void; refreshKernel: () => void; refreshOsVersion: () => void;
  } = $props();
  // Каждая измеренная строка — свой источник и свой запрос; обновления независимы.
  let uptimeHint = $derived(readHint(uptime.issue, pendingUptime, "/proc/uptime"));
  let kernelHint = $derived(readHint(kernel.issue, pendingKernel, "/proc/version"));
  let osVersionHint = $derived(readHint(osVersion.issue, pendingOsVersion, "/etc/os-release"));
</script>
<section class="panel info-panel" aria-labelledby="info-title">
  <div class="panel-heading"><div><div class="section-label"><Server size={13} /> ОКРУЖЕНИЕ</div><h2 id="info-title">Среда CachyOS</h2></div>
    <span class="badge amber">Local Only</span>
  </div>
  <p class="panel-sub">Параметры узла и бэкапа</p>
  <dl class="info-list">
    <div><dt>Hostname</dt><dd>—</dd></div>
    <div id="os-version-row" aria-busy={pendingOsVersion} aria-labelledby="os-version-label os-version-value os-version-meta">
      <dt id="os-version-label">Версия системы</dt>
      <dd>
        <span id="os-version-value">{#if osVersion.value}{osVersion.value.version}{:else}—{/if}</span>
        <span id="os-version-meta" class="info-meta">{#if osVersion.value}<code>{osVersion.value.source}</code> <time datetime={osVersion.value.observed_at_utc} title={osVersion.value.observed_at_utc}>{formatUtc(osVersion.value.observed_at_utc)}</time>{:else}{osVersionHint}{/if}</span>
      </dd>
      {#if isTauri()}<button class="icon-btn row-refresh" onclick={refreshOsVersion} disabled={pendingOsVersion} aria-label="Обновить версию системы" title="Обновить версию системы"><RefreshCw size={14} /></button>{/if}
    </div>
    <div id="kernel-row" aria-busy={pendingKernel} aria-labelledby="kernel-label kernel-value kernel-meta">
      <dt id="kernel-label">Ядро</dt>
      <dd>
        <span id="kernel-value">{#if kernel.value}{kernel.value.version}{:else}—{/if}</span>
        <span id="kernel-meta" class="info-meta">{#if kernel.value}<code>{kernel.value.source}</code> <time datetime={kernel.value.observed_at_utc} title={kernel.value.observed_at_utc}>{formatUtc(kernel.value.observed_at_utc)}</time>{:else}{kernelHint}{/if}</span>
      </dd>
      {#if isTauri()}<button class="icon-btn row-refresh" onclick={refreshKernel} disabled={pendingKernel} aria-label="Обновить версию ядра" title="Обновить версию ядра"><RefreshCw size={14} /></button>{/if}
    </div>
    <div id="uptime-row" aria-busy={pendingUptime} aria-labelledby="uptime-label uptime-value uptime-meta">
      <dt id="uptime-label">Время работы</dt>
      <dd>
        <span id="uptime-value">{#if uptime.value}{formatUptime(uptime.value.uptime_seconds)}{:else}—{/if}</span>
        <span id="uptime-meta" class="info-meta">{#if uptime.value}<code>{uptime.value.source}</code> <time datetime={uptime.value.observed_at_utc} title={uptime.value.observed_at_utc}>{formatUtc(uptime.value.observed_at_utc)}</time>{:else}{uptimeHint}{/if}</span>
      </dd>
      {#if isTauri()}<button class="icon-btn row-refresh" onclick={refreshUptime} disabled={pendingUptime} aria-label="Обновить время работы" title="Обновить время работы"><RefreshCw size={14} /></button>{/if}
    </div>
    <div><dt>Snapshot backend</dt><dd>—</dd></div>
    <div><dt>Dropbox</dt><dd>не привязан</dd></div>
  </dl>
</section>
