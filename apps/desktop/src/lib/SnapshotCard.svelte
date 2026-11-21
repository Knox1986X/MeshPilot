<script lang="ts">
  import { Cpu, MemoryStick, RefreshCw } from "@lucide/svelte";
  import { isTauri } from "@tauri-apps/api/core";
  import { formatBytes, formatPercent, type CpuObservation, type RamObservation } from "./measurements";
  type Props = { pending: boolean; refresh: () => void } & (
    { kind: "ram"; sample: RamObservation | null } | { kind: "cpu"; sample: CpuObservation | null }
  );
  let props: Props = $props();
  const config = {
    ram: { title: "Занято RAM", source: "/proc/meminfo", icon: MemoryStick, refresh: "Обновить снимок RAM" },
    cpu: { title: "Загрузка CPU", source: "/proc/stat", icon: Cpu, refresh: "Обновить замер CPU" },
  };
  let metric = $derived(config[props.kind]);
  let reading = $derived.by(() => {
    if (!props.sample) return null;
    if (props.kind === "cpu") return { value: formatPercent(props.sample.usage_percent), detail: `за ${props.sample.interval_ms} мс`, fill: props.sample.usage_percent };
    return { value: formatBytes(props.sample.used_bytes), detail: `из ${formatBytes(props.sample.total_bytes)}`, fill: props.sample.used_bytes / props.sample.total_bytes * 100 };
  });
</script>
<article class={"panel metric snapshot " + props.kind} aria-labelledby={`${props.kind}-heading ${props.kind}-value ${props.kind}-meta`} aria-busy={props.pending}>
  <div class="metric-head">
    <span class="metric-icon"><metric.icon size={20} /></span><h3 id={`${props.kind}-heading`}>{metric.title}</h3>
    {#if props.pending}<span class="sample-label">замер…</span>{:else if reading}<span class="sample-label">снимок</span>{:else}<span class="no-data">нет данных</span>{/if}
    {#if isTauri()}<button class="icon-btn refresh-sample" onclick={props.refresh} disabled={props.pending} aria-label={metric.refresh} title={metric.refresh}><RefreshCw size={14} /></button>{/if}
  </div>
  {#if reading && props.sample}
    <div class="sample-reading" id={`${props.kind}-value`}><strong>{reading.value}</strong> <small>{reading.detail}</small></div>
    <div class="sample-meta metric-sub" id={`${props.kind}-meta`}><code>{props.sample.source}</code> <time datetime={props.sample.observed_at_utc} title={props.sample.observed_at_utc}>{props.sample.observed_at_utc.slice(0, 10)} {props.sample.observed_at_utc.slice(11, 19)} UTC</time></div>
    <div class="metric-track" aria-hidden="true"><i style:width={`${reading.fill}%`}></i></div>
  {:else}
    <div class="metric-value" id={`${props.kind}-value`}>—</div>
    <div class="metric-sub" id={`${props.kind}-meta`}>{isTauri() ? `Источник: ${metric.source}` : "В браузере источник недоступен"}</div>
    <div class="metric-track" aria-hidden="true"></div>
  {/if}
</article>
