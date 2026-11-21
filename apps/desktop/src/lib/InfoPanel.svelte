<script lang="ts">
  import { RefreshCw, Server } from "@lucide/svelte";
  import { isTauri } from "@tauri-apps/api/core";
  import { formatUptime, formatUtc, readHint, type ReadIssue, type Reading, type UptimeObservation } from "./measurements";
  let { uptime, pending, refresh }: {
    uptime: Reading<UptimeObservation>; pending: boolean; refresh: () => void;
  } = $props();
  let hint = $derived(readHint(uptime.issue, pending, "/proc/uptime"));
</script>
<section class="panel info-panel" aria-labelledby="info-title">
  <div class="panel-heading"><div><div class="section-label"><Server size={13} /> ОКРУЖЕНИЕ</div><h2 id="info-title">Среда CachyOS</h2></div>
    <span class="panel-tools"><span class="badge amber">Local Only</span>
      {#if isTauri()}<button class="icon-btn" onclick={refresh} disabled={pending} aria-label="Обновить время работы" title="Обновить время работы"><RefreshCw size={14} /></button>{/if}
    </span>
  </div>
  <p class="panel-sub">Параметры узла и бэкапа</p>
  <dl class="info-list">
    <div><dt>Hostname</dt><dd>—</dd></div>
    <div aria-labelledby="uptime-label uptime-value uptime-meta">
      <dt id="uptime-label">Время работы</dt>
      <dd>
        <span id="uptime-value">{#if uptime.value}{formatUptime(uptime.value.uptime_seconds)}{:else}—{/if}</span>
        <span id="uptime-meta" class="info-meta">{#if uptime.value}<code>{uptime.value.source}</code> <time datetime={uptime.value.observed_at_utc} title={uptime.value.observed_at_utc}>{formatUtc(uptime.value.observed_at_utc)}</time>{:else}{hint}{/if}</span>
      </dd>
    </div>
    <div><dt>Snapshot backend</dt><dd>—</dd></div>
    <div><dt>Dropbox</dt><dd>не привязан</dd></div>
  </dl>
</section>
