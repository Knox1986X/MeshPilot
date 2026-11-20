<script lang="ts">
  import { MemoryStick, RefreshCw } from "@lucide/svelte";
  import { isTauri } from "@tauri-apps/api/core";
  import { formatBytes, type RamObservation } from "./ram";
  let { ram, pending, refresh }: { ram: RamObservation | null; pending: boolean; refresh: () => void } = $props();
</script>
<article class="panel metric ram" aria-labelledby="ram-heading ram-value ram-meta" aria-busy={pending}>
  <div class="metric-head">
    <span class="metric-icon"><MemoryStick size={20} /></span><h3 id="ram-heading">Занято RAM</h3>
    {#if ram}<span class="sample-label">снимок</span>{:else}<span class="no-data">{pending ? "замер…" : "нет данных"}</span>{/if}
    {#if isTauri()}<button class="icon-btn refresh-ram" onclick={refresh} disabled={pending} aria-label="Обновить снимок RAM" title="Обновить снимок RAM"><RefreshCw size={14} /></button>{/if}
  </div>
  {#if ram}
    <div class="ram-reading" id="ram-value"><strong>{formatBytes(ram.used_bytes)}</strong> <small>из {formatBytes(ram.total_bytes)}</small></div>
    <div class="ram-meta metric-sub" id="ram-meta"><code>{ram.source}</code> <time datetime={ram.observed_at_utc} title={ram.observed_at_utc}>{ram.observed_at_utc.slice(0, 10)} {ram.observed_at_utc.slice(11, 19)} UTC</time></div>
    <div class="metric-track" aria-hidden="true"><i style:width={`${ram.used_bytes / ram.total_bytes * 100}%`}></i></div>
  {:else}
    <div class="metric-value" id="ram-value">—</div>
    <div class="metric-sub" id="ram-meta">{isTauri() ? "Источник: /proc/meminfo" : "В браузере источник недоступен"}</div>
    <div class="metric-track" aria-hidden="true"></div>
  {/if}
</article>
