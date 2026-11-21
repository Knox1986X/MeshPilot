<script lang="ts">import { Cpu } from "@lucide/svelte";</script>
<section class="panel cores-panel" aria-labelledby="cores-title">
  <div class="panel-heading"><div><div class="section-label"><Cpu size={13} /> ПРОЦЕССОР</div><h2 id="cores-title">Нагрузка по ядрам CPU</h2></div><span class="badge">нет данных</span></div>
  <p class="panel-sub">Показания по отдельным CPU не собираются</p>
  <div class="core-grid" aria-hidden="true">{#each Array(4) as _}<div class="core-socket"><div><span>CPU —</span><b>—</b></div><i></i></div>{/each}</div>
  <p class="card-note"><i class="tiny-dot"></i> Число логических CPU не определено</p>
</section>
