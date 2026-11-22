# Zapret2 в MeshPilot

## Что это
Zapret2 (Flowseal/zapret2) — актуальная ветка zapret для обхода DPI-блокировок в RU.
Поддерживает nftables, стратегии fake/split/multisplit/syndata, YouTube/Discord/Telegram.

## Установка
```bash
meshpilot zapret install --no-dry-run
# или вручную:
git clone https://github.com/Flowseal/zapret2.git /tmp/zapret2
cd /tmp/zapret2 && make && sudo make install
sudo systemctl enable --now zapret2
```

## Стратегии (config/zapret_strategies.yaml)
- `auto` — автоподбор перебором по youtube/discord/google/github
- `fake-split` — классика RU
- `multisplit` — YouTube/Discord
- `syndata` — HTTPS
- `wireguard-bypass` — UDP/WireGuard
- `minimal` — слабые ПК

## Управление
```bash
meshpilot zapret status
meshpilot zapret auto --no-dry-run
meshpilot zapret restart
meshpilot zapret logs
```

## Важно
- Zapret2 требует root (nftables). MeshPilot использует `sudo -n` — настрой `sudoers` или вводи пароль.
- Стратегии обновляются из community lists. MeshPilot умеет `zapret update`.
- Для игр: zapret может добавлять latency — включай только для нужных хостов (hostlist).
