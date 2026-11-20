<script lang="ts">
  import { onMount } from "svelte";
  import { resolveView, views } from "./lib/navigation";
  import Sidebar from "./lib/Sidebar.svelte";
  import TopBar from "./lib/TopBar.svelte";
  import Dashboard from "./lib/Dashboard.svelte";
  import PreviewView from "./lib/PreviewView.svelte";
  let active = $state(resolveView(window.location.hash));
  let main: HTMLElement;
  onMount(() => {
    function navigate() { active = resolveView(window.location.hash); main.scrollTop = 0; main.focus(); }
    window.addEventListener("hashchange", navigate);
    return () => window.removeEventListener("hashchange", navigate);
  });
</script>
<svelte:head><title>MeshPilot · {views.find((v) => v.id === active)?.name}</title></svelte:head>
<div class="shell"><a class="skip" href="#main" onclick={(e) => { e.preventDefault(); main.focus(); }}>К содержимому</a>
  <Sidebar {active} /><TopBar {active} />
  <main bind:this={main} id="main" tabindex="-1">{#key active}{#if active === "dashboard"}<Dashboard />{:else}<PreviewView id={active} />{/if}{/key}</main>
  <footer class="statusbar"><span><i></i>meshpilot ready <b>·</b> zero-cost mode</span><span>cpu — <b>·</b> ram — <b>·</b> ↓ — B/s ↑ — B/s</span></footer>
</div>
