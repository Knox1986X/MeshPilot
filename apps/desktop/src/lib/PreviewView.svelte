<script lang="ts">
  import { ArrowLeft, LockKeyhole } from "@lucide/svelte";
  import { views, type ViewId } from "./navigation";
  type PreviewId = Exclude<ViewId, "dashboard">;
  let { id }: { id: PreviewId } = $props();
  let view = $derived(views.find((v) => v.id === id)!);
  const previews = {
    cleanup: { step: 4, title: "План очистки", text: "Размеры не рассчитаны. Перед удалением MeshPilot покажет категории, пути, объём и запросит подтверждение.", rows: ["Кэш pacman и Flatpak", "Снапшоты Btrfs и журналы", "Временные файлы и кэш шейдеров"] },
    network: { step: 3, title: "Сетевые профили", text: "DNS и зеркала не измерялись. MeshPilot не менял маршруты или резолвер. Восстановление появится вместе с журналом собственных изменений.", rows: ["Подобрать DNS", "Подобрать зеркала", "Восстановить сетевые настройки", "Управление обходом блокировок"] },
    permissions: { step: 5, title: "Системный доступ", text: "Привилегированный помощник не установлен этим приложением. Пароль не запрашивается, состояние Polkit и защитных служб не проверялось.", rows: ["Polkit", "Firewall", "CVE и TLS", "Аудит входа"] },
    settings: { step: 5, title: "Параметры рабочего места", text: "Сейчас доступны только просмотр и навигация. Автозапуск, уведомления и синхронизация не включены. Конфигурация на диск не записывается.", rows: ["Общее и уведомления", "Трей и экспертный режим", "GitHub и Dropbox"] },
  } as const;
  let spec = $derived(previews[id]);
</script>
<div class="page preview-page"><div class="eyebrow"><i></i> WORKSPACE <b>/</b> PREVIEW</div><h1>{spec.title}</h1>
  <section class="panel preview-card"><div class="panel-heading"><span class="preview-icon"><view.icon size={24}/></span><span class="badge">Этап {spec.step}</span></div>
    <h2>Предварительный экран</h2><p>{spec.text}</p>
    <ul>{#each spec.rows as row}<li><span>{row}</span><small><LockKeyhole size={12}/> недоступно</small></li>{/each}</ul>
    <a class="ghost" href="#/dashboard"><ArrowLeft size={15}/> На Dashboard</a>
  </section>
</div>
