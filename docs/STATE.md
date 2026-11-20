# MeshPilot · контрольная точка

**Дата:** 2026-11-20 UTC. **Этап:** 1, каркас только для чтения. **Ветка:** `arena/9c5d61a6-meshpilot` от `018d537`. **PR:** [Knox1986X/MeshPilot#1](https://github.com/Knox1986X/MeshPilot/pull/1), открыт в `main`, не слит. Релиза нет.

## Доступ и результат

- **Команды выполнены:** `git ls-remote`, `gh repo view`, `git push -u origin arena/9c5d61a6-meshpilot`, `gh pr create`, `gh pr view 1`. Проверены чтение, создание удалённой ветки записью и создание PR. Создание другой локальной ветки — **не проверено**: ветка сессии закреплена. В `main` не пушили.
- **Dropbox Arena:** только `/MeshPilot/`; папка создана, контрольная точка записана и прочитана. Обновления по ожидаемой ревизии, без перезаписи чужих правок. Во время параллельной промежуточной записи был HTTP 429. Полный архив проектных документов `/MeshPilot/design/MeshPilot-stage-1-6fa6b5f.zip` записан отдельно, content hash совпал. Это не OAuth приложения.
- **Закоммичено:** исходный собираемый каркас `81d6b9d`; контракты, проверка HOME/XDG и отказ от root GUI — `6fa6b5f`. Текущее содержимое и границы — в PR, [дерево с пометками](../README.md#дерево-и-границы-этапа).
- **Файлы созданы:** workspace из `meshpilot-core`, `meshpilot-linux`, `meshpilot-cli`, их тесты и `Cargo.lock`; `apps/desktop/src` (Svelte/TypeScript), `src-tauri` (минимальные оконные capabilities, свой lockfile), `tests`, `.github/workflows/ci.yml`. Нативное окно на Linux отказывается запускать WebView с effective UID 0.
- **Проектные артефакты:** [архитектура и IPC](ARCHITECTURE.md), [обе таблицы переходов](STATE-MACHINES.md), [бэкап и две JSON Schema](BACKUP.md), [требования 01–100](ROADMAP.md), [решения](DECISIONS.md). Службы, Polkit, независимый таймер, установка пакетов, шифрование и Android **предложены, не реализованы**.

## Проверки

Из корня, в Debian 12. Локальный Rust stable 1.88.0 и проверенные по индексу crates.io зависимости лежали вне репозитория (`PATH=/tmp/mp-tooling/rust/bin:$PATH`, `CARGO_HOME=/tmp/mp-tooling/cargo-home` для workspace). CI использует закреплённый stable 1.95.0. Команды для обычной среды — в [README](../README.md#проверки-из-корня).

| Команда выполнена | Наблюдение |
|---|---|
| `cargo fmt --all --check`; `cargo clippy --locked --workspace --all-targets -- -D warnings` | Успех |
| `cargo test --workspace --locked` | **8 тестов**: повторы, подмена плана, граница отмены, точность ID, безопасный разбор OS ID, отчёты/коды и первый запуск с пустым HOME/XDG |
| `cargo build --workspace --locked`; `cargo package -p meshpilot-core --locked` | Бинарник собран; Cargo-архив ядра собран и проверен компиляцией. Это не PKGBUILD |
| `target/debug/meshpilot status --json`; `target/debug/meshpilot doctor --json` | `status`: код 0, `debian`, наблюдения `null`. `doctor`: код 3, не CachyOS, pacman и Flatpak не обнаружены |
| `python3 tests/project.py`; `PYTHONPATH=/tmp/mp-tooling/schema-check python3 tests/backup_schema.py` | 3 проверки проекта и 2 проверки структуры схем прошли; `jsonschema==4.25.1`. Шифрование этим не проверено |
| В `apps/desktop`: `npm ci`, `npm run check`, `npm run build` | Проверка типов: 0 ошибок и предупреждений; production-ресурсы собраны, зависимости и шрифты локальны во время работы |
| В `apps/desktop`: `LD_LIBRARY_PATH=/tmp/al2023/lib CHROMIUM_BINARY=/tmp/chromium npm run test:layout` | Chromium 138: 4/2/1 колонки при 1920/1280/768 px, сайдбар 260 px без перекрытия, четыре «нет данных», навигация без внешних запросов |
| `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml --check`; `git diff --check` | Успех |
| `git grep` по идентификаторам приложения, D-Bus и units; `cargo tree -p meshpilot-core --locked --edges normal` | Префикс `io.github.Knox1986X.MeshPilot`, имена `meshpilot-agent.service`, `meshpilot-helper.service`, `meshpilot-*.timer`; у ядра нет Linux/GUI-зависимостей |
| `gh pr checks 1`; `gh run view 44804900872 --json jobs,conclusion` | [Прогон для 81d6b9d](https://github.com/Knox1986X/MeshPilot/actions/runs/44804900872): workspace и desktop прошли, включая нативную сборку и появление окна на Xvfb |

При перепроверке `6fa6b5f` workspace и браузерные проверки прошли; шаг подготовки системных библиотек на новом runner занял более восьми минут. Для CI задан отдельный таймаут 6 минут на этот шаг и ограничение повторов APT. Результаты относятся к указанным SHA. **Актуальное заключение для головы ветки:** [Checks PR №1](https://github.com/Knox1986X/MeshPilot/pull/1/checks), команда `gh pr checks 1`.

## Ограничения и открытые вопросы

- Нативный запуск **локально не проверен**: в образе нет WebKitGTK 4.1 и дисплея. Браузерный предпросмотр не подменяет нативный smoke-тест. На реальной CachyOS и Android **не проверено**; сборка/окно в CI — Ubuntu VM.
- `JobBook` живёт в памяти. Нет долговечного задания после перезапуска, реальных замеров CPU/сети, списка обновлений или гарантий отката. Не тестировали системные изменения, не трогали реальную машину.
- Лицензия ещё не выбрана; `cargo package` предупреждает о метаданных. Публикация отключена. Открытый вопрос для приёмки: поведение оболочки в графической сессии CachyOS.

**Следующий шаг:** после успешных Checks PR провести smoke-тест этого каркаса в одноразовой графической VM CachyOS без изменения сетевых настроек и пакетов.
