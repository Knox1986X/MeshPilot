<script lang="ts">
  import { onMount } from "svelte";
  import { resolveView, views } from "./lib/navigation";
  import Sidebar from "./lib/Sidebar.svelte";
  import TopBar from "./lib/TopBar.svelte";
  import Dashboard from "./lib/Dashboard.svelte";
  import PreviewView from "./lib/PreviewView.svelte";
  import { readCpu, readRam, formatBytes, formatPercent, type Metric, type Reading, type CpuObservation, type RamObservation } from "./lib/measurements";
  let cpu = $state<Reading<CpuObservation>>({ value: null, issue: "unavailable" });
  let ram = $state<Reading<RamObservation>>({ value: null, issue: "unavailable" });
  let pending = $state({ ram: false, cpu: false });
  async function refresh(kind: Metric) {
    if (pending[kind]) return;
    pending[kind] = true;
    try {
      if (kind === "cpu") cpu = await readCpu();
      else ram = await readRam();
    } finally { pending[kind] = false; }
  }
  let active = $state(resolveView(window.location.hash));
  let main: HTMLElement;
  onMount(() => {
    void refresh("ram");
    void refresh("cpu");
    function navigate() { active = resolveView(window.location.hash); main.scrollTop = 0; main.focus(); }
    window.addEventListener("hashchange", navigate);
    return () => window.removeEventListener("hashchange", navigate);
  });
</script>
<svelte:head><title>MeshPilot · {views.find((v) => v.id === active)?.name}</title></svelte:head>
<div class="shell"><a class="skip" href="#main" onclick={(e) => { e.preventDefault(); main.focus(); }}>К содержимому</a>
  <Sidebar {active} /><TopBar {active} />
  <main bind:this={main} id="main" tabindex="-1">{#key active}{#if active === "dashboard"}<Dashboard {cpu} {ram} {pending} {refresh} />{:else}<PreviewView id={active} />{/if}{/key}</main>
  <footer class="statusbar"><span><i></i>meshpilot ready <b>·</b> только чтение</span><span>cpu {cpu.value ? formatPercent(cpu.value.usage_percent) : "—"} <b>·</b> ram {ram.value ? formatBytes(ram.value.used_bytes) : "—"} <b>·</b> ↓ — B/s ↑ — B/s</span></footer>
</div>
