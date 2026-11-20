# MeshPilot

Локальная консоль обслуживания CachyOS. **Этап 1: только чтение.** Есть CLI и окно Tauri v2 с Dashboard на Svelte и TypeScript. Нет работающего помощника, изменений DNS, очистки, установки пакетов, фоновых служб или синхронизации. При открытии приложения согласие не предполагается.

`meshpilot status` отвечает, что уже известно. `meshpilot doctor` читает ID системы и проверяет наличие системных утилит по фиксированным путям, **не запускает** их. На Live ISO с `ID=arch` дополнительно сверяются название системы и маркер CachyOS; JSON сохраняет исходный `os_id`. Наличие файла не доказывает работоспособность сервиса. В окне `—` и «нет данных» означают отсутствие измерений; это не нулевая нагрузка, не отсутствие обновлений и не отказ сети. «Быстрая очистка» открывает предварительный экран без удаления. `meshpilot ready` относится к готовности интерфейса.

## Запуск без установки

```sh
cargo run --locked -p meshpilot-cli -- status
cargo run --locked -p meshpilot-cli -- doctor --json --dry-run
cd apps/desktop
npm ci
npm run tauri -- dev
```

Нужны Rust stable (версия для CI в `rust-toolchain.toml`), Node.js 22.22.3 и для нативного окна — GTK 3, WebKitGTK 4.1, компоновщик, pkg-config и графическая сессия Linux. Для просмотра в браузере достаточно `npm ci` и `npm run dev` в `apps/desktop`. Шрифты и иконки локальные. Терминал нужен для разработки, не для действий в будущем GUI.

CLI: `0` — отчёт построен; `2` — ошибка аргументов или неподдерживаемая команда; `3` — `doctor` не подтвердил обязательную предпосылку. Отчёт при коде `3` всё равно доступен. Отсутствие необязательного Flatpak само по себе не ведёт к `3`. `--json` выдаёт схему 1; `--dry-run` допустим, обе команды и без него только читают. `updates apply` и прочие действия следующих этапов не имитируются.

## Дерево и границы этапа

```text
Cargo.toml, Cargo.lock, rust-toolchain.toml
crates/
  meshpilot-core/       сейчас: модели отчёта, Plan, JobBook, чистые переходы
  meshpilot-linux/      сейчас: пассивная диагностика Linux
  meshpilot-cli/        сейчас: бинарник meshpilot и интеграционные тесты
  meshpilot-net/        позже: DNS и проверки доступности (2)
  meshpilot-agent/      позже: агент пользователя, Tokio, IPC, события (2)
  meshpilot-helper/     позже: узкий сервис с Polkit (3)
  meshpilot-sync/       позже: AEAD, ревизии и Dropbox App Folder (6)
apps/
  desktop/              сейчас: Vite, Svelte, TypeScript, lockfile npm
    src/                main.ts, App.svelte, компоненты, тёмная тема
    src-tauri/          независимый манифест приложения и lockfile; не CLI
    tests/              браузерная приёмка собранного интерфейса
  android/              позже: Kotlin и общее Rust-ядро (7)
tests/                  сейчас: инварианты проекта, smoke-тест окна в VM
.github/workflows/      сейчас: проверки workspace и desktop
packaging/              позже: PKGBUILD для meshpilot (6)
docs/                   дорожная карта, контракты, решения, контрольная точка
```

Пути с пометкой «позже» не созданы и не включены в Cargo. `meshpilot-core` ничего не знает о GUI, pacman, systemd, Android или Dropbox. Манифест desktop отделён, чтобы CLI на сервере не требовал GTK и даже разрешения его зависимостей.

## Проверки из корня

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --locked
cargo package -p meshpilot-core --locked
python3 tests/project.py
npm --prefix apps/desktop ci
npm --prefix apps/desktop run check
npm --prefix apps/desktop run build
CHROMIUM_BINARY="$(command -v google-chrome)" npm --prefix apps/desktop run test:layout
cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml --check
```

На Linux VM с нативными библиотеками дополнительно:

```sh
cd apps/desktop
cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings
npm run tauri -- build --debug --no-bundle --ci -- --locked
xvfb-run -a dbus-run-session -- bash ../../tests/native-window.sh src-tauri/target/debug/meshpilot-desktop
```

[Приёмка в CachyOS Live](docs/VM-SMOKE.md) проверяет CLI и нативное окно внутри отдельной VM без NIC и пишущего диска. CI не меняет сеть хоста и не публикует артефакты. Проверка Cargo-пакета ядра — не сборка PKGBUILD и не релиз. Результаты, а не только список команд: [контрольная точка](docs/STATE.md). Полный объём и способы проверки: [требования 01–100](docs/ROADMAP.md).

## Проект следующих этапов

- [Процессы, границы доверия, типы и IPC](docs/ARCHITECTURE.md)
- [Обе таблицы переходов: журнал, падение, повтор или ручное решение](docs/STATE-MACHINES.md)
- [Формат бэкапа Dropbox v1 и восстановление](docs/BACKUP.md)
- [Датированные архитектурные решения](docs/DECISIONS.md)
