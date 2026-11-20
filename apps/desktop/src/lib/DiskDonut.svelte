<script lang="ts">import { Disc3 } from "@lucide/svelte";</script>
<section class="panel donut-panel" aria-labelledby="disk-title">
  <div class="panel-heading"><div><div class="section-label"><Disc3 size={13} /> ХРАНИЛИЩЕ</div><h2 id="disk-title">Распределение диска</h2></div><span class="badge">— томов</span></div>
  <div class="disk-body">
    <div class="donut" role="img" aria-label="Диаграмма без данных о томах">
      <svg viewBox="0 0 200 200" aria-hidden="true"><circle class="donut-edge" cx="100" cy="100" r="94"/><circle class="donut-band" cx="100" cy="100" r="72.5"/><circle class="donut-rule" cx="100" cy="100" r="86"/><circle class="donut-edge" cx="100" cy="100" r="51"/></svg>
      <div class="donut-value">—<span>ЗАНЯТО</span></div>
    </div>
    <dl class="disk-legend">{#each ["Занято", "Свободно", "Всего"] as key}<div><dt><i></i>{key}</dt><dd>— <small>GB</small></dd></div>{/each}</dl>
  </div>
  <div class="chart-foot"><span class="tiny-dot"></span> Занятое место по mountpoints</div>
</section>
