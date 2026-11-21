# Архитектура MeshPilot

## Слои

```
┌─────────────────────────────────────────────┐
│ React 18 + Vite (веб-view, без фреймворка)  │  apps/desktop/src
│  pages/* · components/* · lib/api.ts        │  типизированный invoke()
└──────────────┬──────────────────────────────┘
               │ Tauri IPC (JSON, camelCase)
┌──────────────▼──────────────────────────────┐
│ Rust / Tauri 2 (без плагинов fs/shell/net)  │  apps/desktop/src-tauri/src
│  commands.rs — тонкие обработчики+валидация │
│  snapshot / cleanup / network / llm / …     │
└─────────────────────────────────────────────┘
```

- **isDemo = !isTauri**: вне Tauri `lib/api.ts` отдаёт явно помеченные синтетические данные;
  в Tauri всегда используется backend. Переключателя «включить демо в Tauri» нет.
- Все команды валидируют входы (enum-поля, диапазоны, схемы для URL/ключей) и возвращают
  `CmdResult<T> = Result<T, AppError>`, где `AppError { code, message }` — без секретов.
- Тяжёлые синхронные операции (`get_snapshot`, анализ/применение очистки, DNS-бенчмарк,
  тест обхода, зеркала) уходят в `spawn_blocking`; окно не блокируется.

## IPC-контракт (28 команд)

Типы — в `src/types.ts` (фронт) и соответствующих `src-tauri/src/*.rs` (бэкенд);
serde с `rename_all = "camelCase"` на обеих сторонах.

| Команда | Вход | Выход | Примечания |
|---|---|---|---|
| get_snapshot | — | Snapshot | CPU/RAM/swap/диски (дедуп по устройству), сеть (дельта от предыдущего опроса), температура hwmon, аптайм, ОС/ядро/Хост, `partial[]` недоступных датчиков |
| cleanup_analyze | — | CleanupReport | 9 категорий; оценка размеров; блокировки по запущенным приложениям; предупреждения |
| cleanup_apply | `{req:{categoryIds, dryRun, confirmRisky}}` | CleanupApplyResult | Повторный анализ после применения; risky/opt-in требуют `confirmRisky`; safe-профиль — настройка `cleanup.safeProfileOnly` |
| mirrors_status | — | MirrorsStatus | os-release, arch-like, инструменты, mirrorlist, листание pacman-базы, последний бэкап |
| mirrors_apply | `{confirm:true}` | OpResult | Бэкап → ранжирование (cachyos-rate-mirrors / rate-mirrors / reflector) → запись через `sudo -n tee`; отказ при <3 серверах |
| dns_status | — | DnsStatus | Определение менеджера (systemd-resolved / NetworkManager / resolv.conf), серверы, DoT/DNSSEC, hint провайдера |
| dns_benchmark | — | DnsBenchmarkResult | Сырой UDP DNS (ручной пакет, проверка QR/rcode/qid), 4 кандидата × 3 домена × 2 попытки; результат кэшируется в state |
| dns_apply | `{confirm:true}` | OpResult | «Лучший из проверенных» из последнего бенчмарка; только systemd-resolved (drop-in + restart) или статический /etc/resolv.conf; бэкап всегда |
| dns_rollback | `{confirm:true}` | OpResult | По последнему бэкапу; для resolved — удалить drop-in и перезапустить |
| bypass_status | — | BypassStatus | ciadpi/tpws по PATH, активность — TCP-connect к 127.0.0.1:<port>, URLs из настроек |
| bypass_test | `{urls: string[]}` | BypassTestResult | `≤10` URL; direct-проверка (blocking reqwest) + до 6 стратегий ciadpi на тестовом loopback-порту; лучшая рабочая стратегия сохраняется в state |
| bypass_enable / bypass_disable | `{confirm:true}` | OpResult | User systemd unit `meshpilot-byedpi.service`; `ExecStart` из PATH ciadpi; только 127.0.0.1 |
| schedules_list | — | ScheduleInfo[] | 4 расписания (report-only): mirrors-daily, dns-daily, net-weekly, cleanup-weekly |
| schedule_set | `{id, enabled}` | OpResult | Создаёт `meshpilot-<id>.{service,timer}` в `~/.config/systemd/user/`, `Persistent=true`, режим report |
| permission_checks | — | PermissionInfo[] | 8 проверок возможностей: paccache, DNS drop-in, mirrorlist, journald, gio/xdg-open, $HOME, ciadpi/tpws, systemd --user |
| llm_provider_status | `{provider}` | ProviderStatus | `keyring`/`session`/`none`, `keyringAvailable` |
| llm_save_key | `{provider, key}` | `{ok, storage, message}` | Валидация (8–512, без пробелов/control); keyring → иначе session-fallback с явным предупреждением |
| llm_delete_key | `{provider}` | `{ok, message}` | Идемпотентно, удаляет из обоих хранилищ |
| llm_test | `{provider, model}` | ConnectionTestResult | GET /models; маппинг 200/401/403/404/429/5xx/таймаут → коды; замер latency; проверка наличия модели |
| llm_list_models | `{provider}` | string[] | GET /models, сортировка, капа 300 |
| jarvis_send | `{req:{messages}}` | JarvisSendResult | Цикл инструментов (≤3 раунда, ≤8 вызовов за раунд); system-prompt с правилами (read-only инструменты, «сэр», имя); резервный провайдер при сбое основного |
| history_load / history_save / history_clear | — / `{messages}` / — | ChatMessage[] / void / `{ok,message}` | `atomic_write` c 0600, битый файл откладывается в `*.broken.json` без падения |
| settings_get / settings_update / settings_reset | — / `{patch}` / — | Settings | Глубокий merge на Value + нормализация (клампы), schemaVersion фиксирован; reset НЕ трогает ключи |

## Хранилища на диске

Всё — XDG, без root:

| Путь | Содержимое | Режим |
|---|---|---|
| `~/.config/meshpilot/settings.json` | настройки (без секретов) | 0600, atomic rename |
| `~/.local/state/meshpilot/history.json` | история чата (только если включено) | 0600, atomic rename |
| `~/.local/state/meshpilot/dns-benchmark.json` | последний бенчмарк DNS | 0600 |
| `~/.local/state/meshpilot/bypass-profile.json` | лучшая стратегия обхода | 0600 |
| `~/.local/state/meshpilot/backups/` | `mirrorlist-*`, `dns-*.json` | атомарные копии |
| `~/.local/state/meshpilot/reports/` | отчёты расписаний | текст |
| `~/.local/state/meshpilot/schedules/<id>.json` | `lastRunUtc`, `lastResult` | 0600 |
| `~/.config/systemd/user/meshpilot-*.{service,timer}` | юниты расписаний и byedpi | user-only |

Секреты (API-ключи) — **только** Secret Service (`keyring`, service `meshpilot`, account `llm-<provider>`),
иначе — `HashMap` в памяти процесса.

## Очистка: модель безопасности

1. Каталог разрешённых категорий (`cleanup::CATALOG`) — ничего вне его невозможно запросить.
2. Все прямые удаления — только внутри канонизированного `$HOME`; симлинки удаляются как
   симлинки, цели не раскрываются; корзина очищается только под `Trash/{files,info,expunged}`.
3. Системные пути (pacman-кэш, journald, coredumps, mirrorlist, DNS drop-in) — **только** через
   `sudo -n` (никогда не интерактивно, чтобы приложение не зависало), иначе категория помечается
   `needs-helper` и не выполняется.
4. `safeProfileOnly` (по умолчанию включён) пропускает всё, кроме `risk=safe`.
   Категории `requiresOptIn`/`risky` требуют флага `confirmRisky` от UI с галочкой.
5. Блокировки по запущенным приложениям (браузеры/мессенджеры по `/proc/*/comm`).
6. После применения — обязательный повторный анализ, UI показывает фактический остаток.
7. `dryRun` — полный проход без единого `remove_*`.

## Джарвис: цикл инструментов

```
system prompt (правила: read-only инструменты, «только через UI подтверждаются изменения»)
→ POST /chat/completions (Groq | OpenRouter, failover на резервный при ошибке)
→ tool_calls? → валидация по allowlist (open_section: enum; остальные без аргументов)
→ execute (cleanup_analyze / get_snapshot / dns_status / mirrors_status) → tool message
→ ≤3 раундов → финальный ответ
```

Инструменты `open_section` исполняется клиентом (навигация). Остальные — read-only.
Destructive-инструмент (cleanup_apply, dns_apply …) намеренно **не выдаётся** модели:
модель может предложить, пользователь решает в интерфейсе.

## Расписания

`systemd --user` таймеры → oneshot-сервис → запуск **того же бинарника** с `--run-schedule <id>`
до подъёма окна: `main()` видит аргумент, выполняет отчёт (`report_mirrors|dns|net|cleanup`)
и выходит. Режим всегда `report`: файлы ничего не удаляются и не меняют. `Persistent=true` —
пропущенный запуск выполнится при следующем включении. Отчёт: `~/.local/state/meshpilot/reports/`.
Этап 3 может добавить `apply`-режим для строго безопасных категорий за отдельной опцией.

## Демо-режим (честность UI)

Вне Tauri весь API — демо: бейдж «ДЕМО» в сайдбаре, баннер в чате, кнопки в деструктивных
разделах возвращают «Демо-режим: …» без эффекта; применения возвращают `freedBytes=0`.
Внутри Tauri демо данных нет; недоступные датчики показываются как `partial[]`
(например, отсутствие hwmon — честно: «датчик температуры»).

## Тесты и гейты

- Фронт: `tsc --noEmit`, `vitest` (форматтеры), `vite build` — все зелёные (2026-11-21).
- Rust: юнит-тесты парсеров (`/proc/stat`, `/proc/meminfo`, `/proc/net/dev`, `/proc/mounts`,
  os-release, DNS-пакет, медианы, merge настроек, каталог очистки, инварианты dry-run/opt-in).
- CI (`.github/workflows/ci.yml`): frontend job + desktop job (fmt → clippy `-D warnings` →
  cargo test → `tauri build --debug --no-bundle` → нативное окно под `xvfb`/`dbus-run-session`).
  Локально Rust не собирался — **первый настоящий гейт для Rust — этот CI** и запуск на CachyOS.
