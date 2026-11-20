<script lang="ts">
  import { Activity, Radio } from "@lucide/svelte";
  const times = Array.from({ length: 16 }, (_, i) => i === 15 ? "0s" : `−${59 - i * 4}s`);
</script>
<section class="panel chart-panel" aria-labelledby="telemetry-title">
  <div class="panel-heading"><div><div class="section-label"><Activity size={13} /> ТЕЛЕМЕТРИЯ</div><h2 id="telemetry-title">CPU & RAM Telemetry <span>(60s)</span></h2></div>
    <div class="legend-inline"><span class="legend-cpu">CPU %</span><span class="legend-ram">RAM %</span></div>
  </div>
  <div class="plot" role="img" aria-label="Пустой график CPU и RAM за последние 60 секунд: нет данных">
    <div class="y-axis" aria-hidden="true">{#each [100, 75, 50, 25, 0] as n}<span>{n}%</span>{/each}</div>
    <div class="plot-area"><div class="gridlines" aria-hidden="true">{#each Array(5) as _}<i></i>{/each}</div>
      <div class="plot-message"><span class="signal-icon"><Radio size={17} /></span><strong>Ожидание телеметрии</strong><small>Источник метрик не подключён</small></div>
    </div>
    <div class="x-axis" aria-hidden="true">{#each times as t}<span>{t}</span>{/each}</div>
  </div>
  <div class="chart-foot"><span class="tiny-dot"></span> Скользящее окно 60 секунд <b>·</b> шаг 1 сек <span class="foot-source">Tauri Events</span></div>
</section>
