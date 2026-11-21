<script lang="ts">
  import { Network, RefreshCw } from "@lucide/svelte";
  import { isTauri } from "@tauri-apps/api/core";
  import { formatRate, formatUtc, readHint, type InterfaceIo, type NetworkIoObservation, type ReadIssue } from "./measurements";
  let { sample, selected, issue, pending, refresh, choose }: {
    sample: NetworkIoObservation | null; selected: InterfaceIo | null; issue: ReadIssue | null;
    pending: boolean; refresh: () => void; choose: (name: string) => void;
  } = $props();
</script>
<article class="panel metric snapshot net" aria-labelledby="net-heading net-choice net-value net-meta" aria-busy={pending}>
  <div class="metric-head"><span class="metric-icon"><Network size={20} /></span><h3 id="net-heading">Трафик сети</h3>
    {#if pending}<span class="sample-label">замер…</span>{:else if sample}<span class="sample-label">снимок</span>{:else}<span class="no-data">нет данных</span>{/if}
    {#if isTauri()}<button class="icon-btn refresh-sample" onclick={refresh} disabled={pending} aria-label="Обновить замер сети" title="Обновить замер сети"><RefreshCw size={14} /></button>{/if}
  </div>
  <div id="net-choice" class="net-choice">{#if sample && selected}
    {#if sample.interfaces.length > 1}
      <select aria-label="Сетевой интерфейс" value={selected.name} onchange={(e) => choose(e.currentTarget.value)}>
        {#each sample.interfaces as dev (dev.name)}<option value={dev.name}>{dev.name}</option>{/each}
      </select>
    {:else}<code>{selected.name}</code>{/if}
    {#if selected.name === "lo"}<span>{sample.interfaces.length === 1 ? "только локальная петля" : "локальная петля"}</span>{/if}
  {/if}</div>
  <div id="net-value" class={sample ? "net-reading" : "metric-value"}>
    {#if selected?.rates}<span>Приём <strong>{formatRate(selected.rates.rx_bytes_per_sec)}</strong></span> <span>Передача <strong>{formatRate(selected.rates.tx_bytes_per_sec)}</strong></span>
    {:else if selected}Нет данных за интервал{:else if sample}Нет интерфейсов{:else}—{/if}
  </div>
  <div class:sample-meta={!!sample} class="metric-sub" id="net-meta">
    {#if sample}<span>за {sample.interval_ms} мс</span> <code>{sample.source}</code> <time datetime={sample.observed_at_utc} title={sample.observed_at_utc}>{formatUtc(sample.observed_at_utc)}</time>{:else}{readHint(issue, pending, "/proc/net/dev")}{/if}
  </div>
</article>
