<script lang="ts">
  import { ArrowDown, ArrowUp, Network } from "@lucide/svelte";
</script>
<article class="panel metric net" aria-label="Network I/O">
  <div class="metric-head"><span class="metric-icon"><Network size={20} /></span><h3>Network I/O</h3><span class="no-data">нет данных</span></div>
  <div class="metric-value net-values"><span><ArrowDown size={14} /> — <small>B/s</small></span><span><ArrowUp size={14} /> — <small>B/s</small></span></div>
  <div class="metric-sub">Приём / передача</div>
</article>
