<script lang="ts">import { Server } from "@lucide/svelte";</script>
<section class="panel info-panel" aria-labelledby="info-title">
  <div class="panel-heading"><div><div class="section-label"><Server size={13} /> ОКРУЖЕНИЕ</div><h2 id="info-title">Среда CachyOS</h2></div><span class="badge amber">Local Only</span></div>
  <p class="panel-sub">Параметры узла и бэкапа</p>
  <dl class="info-list">{#each [["Hostname", "—"], ["Uptime", "—"], ["Snapshot backend", "—"], ["Dropbox", "не привязан"]] as [k,v]}<div><dt>{k}</dt><dd>{v}</dd></div>{/each}</dl>
</section>
