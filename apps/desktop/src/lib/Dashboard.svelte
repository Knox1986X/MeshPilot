<script lang="ts">
  import { ArrowUpRight, Box, CircleHelp, ListChecks, Shield, Zap } from "@lucide/svelte";
  import MetricCard from "./MetricCard.svelte";
  import TelemetryChart from "./TelemetryChart.svelte";
  import DiskDonut from "./DiskDonut.svelte";
  import CpuCores from "./CpuCores.svelte";
  import InfoPanel from "./InfoPanel.svelte";
  const metrics = ["cpu", "ram", "disk", "net"] as const;
</script>
<div class="page dashboard">
  <div class="intro"><div><div class="eyebrow"><i></i> SYSTEM OVERVIEW <b>/</b> 01</div><h1>Обзор системы</h1></div>
    <a class="primary" href="#/cleanup" title="Открыть предварительный экран без запуска очистки"><Zap size={16} /> Быстрая очистка</a></div>
  <div class="pills" aria-label="Статусы системы"><span class="pill"><i></i>CachyOS · не проверено</span><span class="pill blue"><i></i>Kernel: —</span><span class="pill"><i></i>Snapshot: —</span><span class="pill"><i></i>Relay: —</span></div>
  <div class="section-divider"><h2>Системные метрики</h2><i></i><span>04 КАНАЛА</span></div>
  <div class="metrics-grid">{#each metrics as kind}<MetricCard {kind} />{/each}</div>
  <div class="analysis-grid"><TelemetryChart /><DiskDonut /><CpuCores /><InfoPanel /></div>
  <section class="summary panel" aria-labelledby="maintenance-title"><div class="summary-heading"><div class="section-label"><ListChecks size={14} /> ОБСЛУЖИВАНИЕ</div><h2 id="maintenance-title">Перед началом работы</h2></div>
    <div class="summary-items">
      <a href="#/network"><CircleHelp size={16}/><span><b>Интернет и DNS</b><small>Не проверено. Профиль не выбран.</small></span><ArrowUpRight size={14}/></a>
      <div><Box size={16}/><span><b>Обновления программ</b><small>Источники не опрошены.</small></span></div>
      <div><Shield size={16}/><span><b>Безопасность системы</b><small>Исправления не проверены.</small></span></div>
      <div><ListChecks size={16}/><span><b>Обновить всё</b><small>Установка отключена. Изменений нет.</small></span></div>
    </div>
  </section>
</div>
