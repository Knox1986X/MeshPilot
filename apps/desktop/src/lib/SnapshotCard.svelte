<script lang="ts">
  import { Cpu, HardDrive, MemoryStick, RefreshCw } from "@lucide/svelte";
  import { isTauri } from "@tauri-apps/api/core";
  import { formatBytes, formatPercent, formatUtc, readHint, type ReadIssue, type CpuObservation, type RamObservation, type RootFsObservation } from "./measurements";
  type Props = { issue: ReadIssue | null; pending: boolean; refresh: () => void } & (
    { kind: "disk"; sample: RootFsObservation | null } | { kind: "ram"; sample: RamObservation | null } | { kind: "cpu"; sample: CpuObservation | null }
  );
  let props: Props = $props();
  const config = {
    disk: { title: "Занято на /", source: "statvfs(/)", icon: HardDrive, refresh: "Обновить снимок ФС /" },
    ram: { title: "Занято RAM", source: "/proc/meminfo", icon: MemoryStick, refresh: "Обновить снимок RAM" },
    cpu: { title: "Загрузка CPU", source: "/proc/stat", icon: Cpu, refresh: "Обновить замер CPU" },
  };
  let metric = $derived(config[props.kind]);
  let hint = $derived(readHint(props.issue, props.pending, metric.source));
  let reading = $derived.by(() => {
    if (!props.sample) return null;
    if (props.kind === "cpu") return { value: formatPercent(props.sample.usage_percent), detail: `за ${props.sample.interval_ms} мс`, fill: props.sample.usage_percent };
    return { value: formatBytes(props.sample.used_bytes), detail: `из ${formatBytes(props.sample.total_bytes)}`, fill: props.sample.used_bytes / props.sample.total_bytes * 100 };
  });
</script>
<article class={"panel metric snapshot " + props.kind} aria-labelledby={`${props.kind}-heading ${props.kind}-value ${props.kind === "disk" ? "disk-available " : ""}${props.kind}-meta`} aria-busy={props.pending}>
  <div class="metric-head">
    <span class="metric-icon"><metric.icon size={20} /></span><h3 id={`${props.kind}-heading`}>{metric.title}</h3>
    {#if props.pending}<span class="sample-label">замер…</span>{:else if reading}<span class="sample-label">снимок</span>{:else}<span class="no-data">нет данных</span>{/if}
    {#if isTauri()}<button class="icon-btn refresh-sample" onclick={props.refresh} disabled={props.pending} aria-label={metric.refresh} title={metric.refresh}><RefreshCw size={14} /></button>{/if}
  </div>
  <div class={reading ? "sample-reading" : "metric-value"} id={`${props.kind}-value`}>
    {#if reading}<strong>{reading.value}</strong> <small>{reading.detail}</small>{:else}—{/if}
  </div>
  {#if props.kind === "disk"}<div id="disk-available" class="sample-available">{#if props.sample}Доступно пользователю: {formatBytes(props.sample.available_bytes)}{/if}</div>{/if}
  <div class:sample-meta={!!props.sample} class="metric-sub" id={`${props.kind}-meta`}>
    {#if props.sample}<code>{props.sample.source}</code> <time datetime={props.sample.observed_at_utc} title={props.sample.observed_at_utc}>{formatUtc(props.sample.observed_at_utc)}</time>{:else}{hint}{/if}
  </div>
  <div class="metric-track" aria-hidden="true">{#if reading}<i style:width={`${reading.fill}%`}></i>{/if}</div>
</article>
