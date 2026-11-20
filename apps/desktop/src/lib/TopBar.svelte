<script lang="ts">
  import { Minus, Square, X, Settings, ShieldCheck, LockKeyhole } from "@lucide/svelte";
  import { isTauri } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { views, type ViewId } from "./navigation";
  let { active }: { active: ViewId } = $props();
  let view = $derived(views.find((v) => v.id === active)!);
  let windowError = $state("");
  async function control(action: "minimize" | "toggleMaximize" | "close") {
    try { await getCurrentWindow()[action](); }
    catch { windowError = "Не удалось изменить состояние окна"; }
  }
</script>
<header class="topbar" data-tauri-drag-region>
  <div class="title-block" data-tauri-drag-region><div class="screen-title" data-tauri-drag-region>{view.name}</div><div class="screen-sub" data-tauri-drag-region>{view.sub}</div></div>
  <div class="top-tools">
    <span class="read-only"><LockKeyhole size={12} /> Только чтение</span>
    <a class="icon-btn" href="#/permissions" aria-label="Открыть Permissions" title="Разрешения"><ShieldCheck size={17} /></a>
    <a class="icon-btn" href="#/settings" aria-label="Открыть Settings & Tray" title="Настройки"><Settings size={17} /></a>
    {#if isTauri()}
      <div class="window-controls">
        <button class="icon-btn" onclick={() => control("minimize")} aria-label="Свернуть"><Minus size={14} /></button>
        <button class="icon-btn" onclick={() => control("toggleMaximize")} aria-label="Изменить размер"><Square size={12} /></button>
        <button class="icon-btn" onclick={() => control("close")} aria-label="Закрыть"><X size={14} /></button>
      </div>
    {/if}
  </div>
  {#if windowError}<span role="alert" class="window-error">{windowError}</span>{/if}
</header>
