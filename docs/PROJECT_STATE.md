# Состояние проекта MeshPilot

_Обновлено: 2026-11-21 (ветка `arena/070c1999-meshpilot` от `main` @ `018d537`)._

## Этап

Этап 2 (переосмысление UI): React + Tauri 2, без Svelte, без UI-фреймворков. Предыдущий
прототип (PR #1, `arena/9c5d61a6-meshpilot`) не мёржится; общая инфраструктура CI переиспользована.

## Что готово и проверено

- **Фронтенд** (`apps/desktop/src`): все 6 разделов + shell (hotkeys Ctrl+1..6, опрос 1 с,
  visibility-aware; тосты; демо-режим с честными пометками).
  - `npx tsc --noEmit` — чисто (exit 0).
  - `npx vitest run` — 13/13 зелёные.
  - `npm run build` — успешно (dist/, ~220 KB JS).
- **Rust/Tauri бэкенд** (`apps/desktop/src-tauri`): 28 команд IPC; парсеры /proc и /sys;
  очистка (9 категорий, safe-profile, opt-in, бэкапы, sudo -n); зеркала; DNS (UDP-бенчмарк,
  resolved/resolv.conf apply+rollback); byedpi (тесты стратегий, user unit); расписания
  (user timers, report-only); LLM (Groq/OpenRouter, failover, read-only инструменты);
  history/settings (0600, atomic rename, битый файл → .broken.json); permissions-диагностика.
- **CI** (`.github/workflows/ci.yml`): frontend (npm ci → check → test → build) и desktop
  (fmt → clippy -D warnings → cargo test → tauri build --debug --no-bundle → окно под xvfb).
- README / ARCHITECTURE / SECURITY — написаны.

## Что НЕ проверено (честно)

- **Сборка Rust в sandbox'е не выполнялась**: нет тулчейна (rustup/cargo отсутствуют),
  apt заблокирован, static.rust-lang.org недоступен. Все `.rs`/TOML/conf написаны «вслепую»
  и проверены только чтением. Вероятность мелких ошибок компиляции/clippy — средняя.
  Первый реальный гейт: CI на GitHub (ubuntu-24.04, rust 1.95.0) и запуск на CachyOS.
- `Cargo.lock` отсутствует: crates.io недоступен на рабочей машине. CI не использует `--locked`.
  **Действие:** после первой сборки на CachyOS закоммитить `Cargo.lock`.
- Нативное окно запускалось только теоретически (скрипт CI есть, сам запуск — в CI).
- Реальная машина (CachyOS, Wayland, KDE, hwmon, NetworkManager/systemd-resolved конфигурация
  автора) не проверялась: поведение `dns_apply`, `paccache`, keyring зависит от окружения.
- Иконка создана генератором; артефакты android/ios из `tauri icon` удалены.
- Лицензия: `LICENSE` не добавлен (планируется MIT в первом релизе).

## Матрица проверки

| Гейт | Статус |
|---|---|
| tsc --noEmit | ✅ 2026-11-21 |
| vitest (13) | ✅ 2026-11-21 |
| vite build | ✅ 2026-11-21 |
| cargo fmt/clippy/test | ⏳ CI |
| tauri build (debug, no-bundle) | ⏳ CI |
| окно под xvfb (native-window.sh) | ⏳ CI |
| запуск на CachyOS (KDE/Wayland) | ⏳ пользователь |
| реальная очистка/DNS/зеркала | ⏳ пользователь, с бэкапами и confirm |

## Дальше (этап 2.1 / 3)

1. Запушить ветку, открыть PR, дождаться CI; починить всё, что найдёт clippy/окно.
2. На CachyOS: `npm ci && npm run tauri dev` — ручной smoke (все разделы, демо-режим, ключ Groq).
3. Закоммитить `Cargo.lock` (и включить `--locked` в CI).
4. Замер реального UI (Wayland fractional scaling), хоткеи.
5. Этап 3: polkit-helper (узкий allowlist действий), apply-режим расписаний
   (только safe-категории), Tauri updater + подпись, релиз `v0.2.0`.
6. Вернуться к CachyOS VM smoke-стенду из PR #1 и подключить окно к нему.

## Окружение автора

- Рабочая станция: CachyOS (Arch), KDE Plasma + Wayland, pacman, Chromium-family +
  Firefox-family, Telegram/AyuGram, Discord/Vesktop, Steam — см. каталог очистки.
- Разработка: Debian 12 sandbox (нет Rust, нет GPU, только статический фронтенд-гейт).
