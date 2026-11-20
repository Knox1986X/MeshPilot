<script lang="ts">
  import { ArrowDown, ArrowUp, Cpu, HardDrive, Network } from "@lucide/svelte";
  let { kind }: { kind: "cpu" | "disk" | "net" } = $props();
  const metrics = {
    cpu: { name: "CPU Load", icon: Cpu, unit: "%", detail: "Логические потоки: —" },
    disk: { name: "Disk Storage", icon: HardDrive, unit: "GB", detail: "Разделы: —" },
    net: { name: "Network I/O", icon: Network, unit: "B/s", detail: "Приём / передача" },
  } as const;
  let metric = $derived(metrics[kind]);
</script>
<article class={"panel metric " + kind} aria-label={metric.name}>
  <div class="metric-head"><span class="metric-icon"><metric.icon size={20} /></span><h3>{metric.name}</h3><span class="no-data">нет данных</span></div>
  {#if kind === "net"}
    <div class="metric-value net-values"><span><ArrowDown size={14} /> — <small>B/s</small></span><span><ArrowUp size={14} /> — <small>B/s</small></span></div>
  {:else}<div class="metric-value">— <small>{metric.unit}</small></div>{/if}
  <div class="metric-sub">{metric.detail}</div>
  {#if kind !== "net"}<div class="metric-track" aria-hidden="true"></div>{/if}
</article>
