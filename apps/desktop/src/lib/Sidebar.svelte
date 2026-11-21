<script lang="ts">
  import { ChevronRight, Cpu, Layers, Route } from "@lucide/svelte";
  import { views, type ViewId } from "./navigation";
  import { version } from "../../package.json";
  let { active }: { active: ViewId } = $props();
</script>
<aside class="sidebar" aria-label="Рабочее пространство">
  <a class="brand" href="#/dashboard" aria-label="MeshPilot · Dashboard">
    <img src="/meshpilot.svg" alt="" width="43" height="43" />
    <div><div class="wordmark">MeshPilot <span class="version">v{version}</span></div><div class="brand-sub">CachyOS</div></div>
  </a>
  <nav aria-label="Экраны MeshPilot">
    <div class="nav-heading">СИСТЕМА <b>·</b> MVP</div>
    {#each views as view}
      <a href={"#/" + view.id} class:active={view.id === active} aria-current={view.id === active ? "page" : undefined}>
        <view.icon size={18} /><span>{view.name}</span>{#if active === view.id}<ChevronRight class="nav-tail" size={14} />{/if}
      </a>
    {/each}
    <div class="nav-heading roadmap-heading">ДОРОЖНАЯ КАРТА</div>
    <button disabled><Route size={18} /><span>Sing-box / WG</span><small>Phase 2</small></button>
    <button disabled><Cpu size={18} /><span>Kernel & Tuning</span><small>Phase 2</small></button>
  </nav>
  <div class="stack-wrap"><div class="stack-card">
    <div class="stack-title"><Layers size={17} /><strong>Zero-Cost Stack</strong><span class="free">Free</span></div>
    <p>GitHub Releases + Dropbox Sync</p><code>~/.Dropbox/MeshPilot</code>
  </div></div>
</aside>
