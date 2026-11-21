<script lang="ts">
  import { Disc3 } from "@lucide/svelte";
  import { formatBytes, formatPercent, formatUtc, type RootFsObservation } from "./measurements";
  let { sample }: { sample: RootFsObservation | null } = $props();
  let used = $derived(sample ? 100 * (sample.used_bytes / sample.total_bytes) : null);
  let available = $derived(sample ? 100 * (sample.available_bytes / sample.total_bytes) : null);
  let rows = $derived([
    { label: "Занято", bytes: sample?.used_bytes, kind: "used" },
    { label: "Доступно пользователю", bytes: sample?.available_bytes, kind: "available" },
    { label: "Свободно, но недоступно", bytes: sample ? sample.total_bytes - sample.used_bytes - sample.available_bytes : undefined, kind: "unavailable" },
    { label: "Всего", bytes: sample?.total_bytes, kind: "total" },
  ]);
</script>
<section class="panel donut-panel" aria-labelledby="disk-title">
  <div class="panel-heading"><div><div class="section-label"><Disc3 size={13} /> ХРАНИЛИЩЕ</div><h2 id="disk-title">Корневая файловая система</h2></div><span class="badge">/</span></div>
  <div class="disk-body">
    <div class="donut" aria-hidden="true">
      <svg viewBox="0 0 200 200">
        <circle class="donut-edge" cx="100" cy="100" r="94"/>
        <circle class="donut-band" class:measured={sample !== null} cx="100" cy="100" r="72.5"/>
        {#if used !== null && available !== null}
          <g transform="rotate(-90 100 100)">
            <circle class="donut-used" cx="100" cy="100" r="72.5" pathLength="100" stroke-dasharray={`${used} 100`}/>
            <circle class="donut-available" cx="100" cy="100" r="72.5" pathLength="100" stroke-dasharray={`${available} 100`} stroke-dashoffset={-used}/>
          </g>
        {:else}<circle class="donut-rule" cx="100" cy="100" r="86"/>{/if}
        <circle class="donut-edge" cx="100" cy="100" r="51"/>
      </svg>
      <div class="donut-value">{used === null ? "—" : formatPercent(used)}<span>ЗАНЯТО</span></div>
    </div>
    <dl class="disk-legend">{#each rows as row}<div><dt><i class={row.kind}></i>{row.label}</dt><dd title={row.bytes === undefined ? undefined : `${row.bytes} байт`}>{row.bytes === undefined ? "—" : formatBytes(row.bytes)}</dd></div>{/each}</dl>
  </div>
  <div class="chart-foot"><span class="tiny-dot"></span>{#if sample}<code>{sample.source}</code> · <time datetime={sample.observed_at_utc}>{formatUtc(sample.observed_at_utc)}</time>{:else}Нет данных о файловой системе /{/if}</div>
</section>
