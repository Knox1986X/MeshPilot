<script lang="ts">
  import { ArrowUpRight, Box, CircleHelp, ListChecks, Shield, Zap } from "@lucide/svelte";
  import NetworkCard from "./NetworkCard.svelte";
  import TelemetryChart from "./TelemetryChart.svelte";
  import DiskDonut from "./DiskDonut.svelte";
  import CpuCores from "./CpuCores.svelte";
  import InfoPanel from "./InfoPanel.svelte";
  import SnapshotCard from "./SnapshotCard.svelte";
  import type { Metric, Reading, CpuObservation, RamObservation, RootFsObservation, NetworkIoObservation, InterfaceIo, UptimeObservation, KernelObservation } from "./measurements";
  let { cpu, ram, disk, net, uptime, kernel, selected, choose, pending, refresh }: {
    cpu: Reading<CpuObservation>; ram: Reading<RamObservation>; disk: Reading<RootFsObservation>;
    net: Reading<NetworkIoObservation>; uptime: Reading<UptimeObservation>; kernel: Reading<KernelObservation>;
    selected: InterfaceIo | null;
    choose: (name: string) => void;
    pending: Record<Metric, boolean>; refresh: (kind: Metric) => void
  } = $props();
</script>
<div class="page dashboard">
  <div class="intro"><div><div class="eyebrow"><i></i> SYSTEM OVERVIEW <b>/</b> 01</div><h1>Обзор системы</h1></div>
    <a class="primary" href="#/cleanup" title="Открыть предварительный экран без запуска очистки"><Zap size={16} /> Быстрая очистка</a></div>
  <div class="pills" aria-label="Статусы системы"><span class="pill"><i></i>CachyOS · не проверено</span><span class="pill blue" title="/proc/version"><i></i>Kernel: {kernel.value?.version ?? "—"}</span><span class="pill"><i></i>Snapshot: —</span><span class="pill"><i></i>Relay: —</span></div>
  <div class="section-divider"><h2>Системные метрики</h2><i></i><span>04 КАНАЛА</span></div>
  <div class="metrics-grid">
    <SnapshotCard kind="cpu" sample={cpu.value} issue={cpu.issue} pending={pending.cpu} refresh={() => refresh("cpu")} />
    <SnapshotCard kind="ram" sample={ram.value} issue={ram.issue} pending={pending.ram} refresh={() => refresh("ram")} />
    <SnapshotCard kind="disk" sample={disk.value} issue={disk.issue} pending={pending.disk} refresh={() => refresh("disk")} />
    <NetworkCard sample={net.value} issue={net.issue} {selected} {choose} pending={pending.net} refresh={() => refresh("net")} />
  </div>
  <div class="analysis-grid"><TelemetryChart /><DiskDonut sample={disk.value} /><CpuCores /><InfoPanel {uptime} {kernel} pendingUptime={pending.uptime} pendingKernel={pending.kernel} refreshUptime={() => refresh("uptime")} refreshKernel={() => refresh("kernel")} /></div>
  <section class="summary panel" aria-labelledby="maintenance-title"><div class="summary-heading"><div class="section-label"><ListChecks size={14} /> ОБСЛУЖИВАНИЕ</div><h2 id="maintenance-title">Перед началом работы</h2></div>
    <div class="summary-items">
      <a href="#/network"><CircleHelp size={16}/><span><b>Интернет и DNS</b><small>Не проверено. Профиль не выбран.</small></span><ArrowUpRight size={14}/></a>
      <div><Box size={16}/><span><b>Обновления программ</b><small>Источники не опрошены.</small></span></div>
      <div><Shield size={16}/><span><b>Безопасность системы</b><small>Исправления не проверены.</small></span></div>
      <div><ListChecks size={16}/><span><b>Обновить всё</b><small>Установка отключена. Изменений нет.</small></span></div>
    </div>
  </section>
</div>
