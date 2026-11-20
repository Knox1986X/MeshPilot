# Окно MeshPilot в одноразовой CachyOS

**Проверено дважды:** в [CachyOS Handheld Live 260628, x86-64 Plasma](https://github.com/Knox1986X/MeshPilot/actions/runs/44818742333/job/136573784969) `meshpilot-desktop` создал видимое окно от `liveuser`. WebKitGTK был в подписанном ISO до загрузки; в госте не устанавливали пакеты. Это тест нативной оболочки в CachyOS, не всех её экранов, не тест портативного устройства или Android.

## Результаты раздельно по профилям

| Профиль и источник | Наблюдение | Закреплённый SHA-256 ISO |
|---|---|---|
| `handheld-260628` · [профиль выпуска](https://github.com/CachyOS/CachyOS-Live-ISO/tree/2b4179563103a030ecc0632cc8446bf4eb960b2d/archiso) | **PASS** на [`98e974b`](https://github.com/Knox1986X/MeshPilot/actions/runs/44817668667/job/136570097754) и повторно на `3aef0e7`. Официальный ISO — 2 664 595 456 байт; исходный `/arch/pkglist.x86_64.txt` содержит `webkit2gtk-4.1 2.52.4-1`. Ядро `linux-cachyos-deckify`; гостевой Plasma-дисплей отвечает, обе библиотеки разрешились в файлы `/usr/lib`. Получен полный протокол ниже | `0567c61f21622e5b4b482a874c41076beeb9bd8c00f046fa40e5c8cfbe34d15f` |
| `260809` · [Desktop Live](https://github.com/CachyOS/wiki/blob/2ef1932f3622b92785dcc05d5d407ccb301c926e/src/content/docs/cachyos_basic/download.mdx) | [CLI, сеанс и неизменность проверены](https://github.com/Knox1986X/MeshPilot/actions/runs/44812307884/job/136551701004). **Не PASS:** загрузчик не нашёл `libjavascriptcoregtk-4.1.so.0` и `libwebkit2gtk-4.1.so.0`. До запуска приложения дело не дошло | `959f6577f45e25ee9fd8c220fd221b08e4ea79412c7315c0f922dd6d86d5e33c` |
| `251129` · [архивный Desktop Live](https://github.com/CachyOS/wiki/blob/9402a5ce4f3b3583adf12b101b44cc7238ee5ded/src/content/docs/cachyos_basic/download.mdx) | **Не проверено в госте.** [Архив не передал ISO целиком за лимит](https://github.com/Knox1986X/MeshPilot/actions/runs/44816310015/job/136565459149). Из первого диапазона 762 503 168 байт получено 100 411 272; `curl` вернул 28. Частичный образ не загружен в QEMU. Автозагрузки этого архива в CI больше нет | `2ab25f72ce8fece890b440cf9e260fc3d57ae4f5fb5308086f670240de02da7b` |

Ни один профиль не служит автоматической заменой другого после ошибки. Не рекомендуем смену редакции или откат пакетов для запуска приложения. [Ubuntu/Xvfb](https://github.com/Knox1986X/MeshPilot/actions/runs/44817678342) и браузерная приёмка — отдельные проверки.

## Как устроен тест

1. `media.sh` принимает только закреплённый профиль. Передача ограничена HTTPS, временем и размером. Явный HTML вместо файла — ошибка доставки, даже при HTTP 200. Источники — зеркала и архив издателя, не произвольный пользовательский URL. Для Handheld дополнительно требуем совпадения пина с файлом `.iso.sha256` дистрибутива. [Отдельная запись контрольной суммы](https://github.com/neramc/linuxhub/blob/0a9ef22ab26dad420939f6c4c2a7d18bc5e6aa17/src/data/releases/cachyos.json) не заменяет эту проверку.
2. `verify.sh` офлайн сверяет SHA-256 байтов и отделённую OpenPGP-подпись ожидаемого ключа `882DCFE48E2051D48E2562ABF3B607488DB35A47`. [Открытый ключ](https://github.com/CachyOS/CachyOS-PKGBUILDS/blob/0669f94df9b4970886b3c3bbee13711aab7db5a8/cachyos-keyring/cachyos.gpg) закреплён также по хешу. Keyring временный. Ошибка хеша, подписи или отпечатка — стоп, без подбора другого хеша. `run.sh` повторяет проверку перед извлечением ядра; это относится и к локально предоставленному носителю.
3. QEMU/KVM работает как обычный пользователь одноразового Ubuntu runner. Ядро и initramfs — из выбранного ISO, испытуемые бинарники — из текущего checkout по lockfiles. У гостя только два read-only CD и рабочий слой в RAM. Нет NIC, пишущего диска, SSH, QGA, общих каталогов и передачи ключей хоста. Зависимости сборки устанавливаются только на runner.
4. Разовая гостевая служба проверяет DMI стенда, виртуализацию, ядро, read-only mountpoints и наличие только loopback. CLI, окно и наблюдатель запускаются как `liveuser` с `no_new_privs` и временным HOME/XDG. Используется **дисплей гостевой Plasma**, X11/XWayland. `LD_LIBRARY_PATH` только у вспомогательного `xdotool`, не у MeshPilot. Sandbox WebKit не отключаем.

**Условие успеха:** по отдельному UART получена ровно последовательность `GUEST → CLI → SESSION → WINDOW → UNCHANGED → PASS` с идентификатором текущего запуска **и** код QEMU равен 0. По отдельности метки означают:

- `GUEST`: прошли проверки среды и носителей.
- `CLI`: `meshpilot status --json` и `meshpilot doctor --json --dry-run` вернули 0. Проверены исходный `os_id`, обнаружение CachyOS и pacman, поля `null`, диагностика файлов GUI и пустой HOME после CLI.
- `SESSION`: от дисплея получены ненулевые размеры. Наличие `DISPLAY` само по себе не доказательство.
- `WINDOW`: наблюдатель нашёл **видимое** окно с заголовком MeshPilot и PID именно запущенного живого процесса. Сеанс, чужое окно или успешный `ldd` не проходят этот критерий.
- `UNCHANGED`: до/после совпали содержимое базы pacman, настройки pacman и зеркал, NetworkManager, networkd/resolved, ссылка и данные `/etc/resolv.conf`, адреса и маршруты IPv4/IPv6. Сравнение выполняется по возможности и после ошибки GUI. Это не утверждение о каждом байте ОС. Содержимое и хеши не покидают гостя.
- `PASS`: гостевой сценарий завершён. Таймаут, неполный отчёт и простое выключение VM — не успех.

## Воспроизведение

Нужен **одноразовый** x86-64 Linux-стенд с KVM, QEMU, GnuPG, xorriso, `xdotool`/libxdo и зависимостями сборки Tauri (см. `.github/workflows/cachyos-vm.yml`, Ubuntu 24.04). Гостю выделяется 5 ГиБ RAM. Запускать QEMU без root, не открывать `/dev/kvm` всем пользователям. Из корня:

```sh
cargo build --locked -p meshpilot-cli
npm --prefix apps/desktop ci
npm --prefix apps/desktop run tauri -- build --debug --no-bundle --ci -- --locked
media_dir="$(mktemp -d)"
profile=handheld-260628
bash tests/cachyos-vm/media.sh "$media_dir" "$profile"
source tests/cachyos-vm/fixtures.sh
select_fixture "$profile"
bash tests/cachyos-vm/run.sh "$media_dir/$name" target/debug/meshpilot apps/desktop/src-tauri/target/debug/meshpilot-desktop "$profile"
```

Для отрицательной проверки Desktop — `profile=260809`. Старый вызов скриптов без профиля по-прежнему означает 260809. Для архивного 251129 `run.sh` принимает только уже имеющуюся пару ISO и соседнюю `.sig`; отдельно проверить пару можно через `bash tests/cachyos-vm/verify.sh ПУТЬ_К_ISO 251129`. Носители не добавляются в Git или Dropbox.

На push изменений приложения/стенда workflow выбирает `handheld-260628`; после появления workflow в основной ветке ручной запуск также позволяет выбрать `260809`. Только документация не скачивает ISO. Локальные тесты правил выбора, проверки и вердикта: `python3 -m unittest discover -s tests/cachyos-vm`. Не запускать `guest.py` как диагностику реальной машины.
