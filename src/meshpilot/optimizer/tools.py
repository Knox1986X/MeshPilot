"""Инструменты оптимизатора для LLM-агента (function calling).

Каждый tool: (name, OpenAI-function-schema, async fn)
Агент сам решает, когда чистить, какие зеркала ставить и т.д. по голосовой команде.
"""
from __future__ import annotations
from typing import Any

from meshpilot.optimizer import cleaner, dns, health, mirrors, system, zapret

def _schema(name: str, desc: str, props: dict | None = None) -> dict:
    return {
        "name": name,
        "description": desc,
        "parameters": {
            "type": "object",
            "properties": props or {},
            "required": [],
        },
    }

async def t_clean_all(dry_run: bool = True) -> str:
    results = cleaner.clean_all(dry_run=dry_run)
    lines = [f"{r.name}: {r.details}" for r in results]
    return "\n".join(lines)

async def t_clean_cache(dry_run: bool = True) -> str:
    r = cleaner.clean_pacman_cache(dry_run=dry_run)
    return f"{r.name}: {r.details}"

async def t_remove_orphans(dry_run: bool = True) -> str:
    r = cleaner.remove_orphans(dry_run=dry_run)
    return f"{r.name}: {r.details}"

async def t_mirrors(dry_run: bool = True) -> str:
    return mirrors.auto_mirrors(dry_run=dry_run)

async def t_dns(dry_run: bool = True) -> str:
    return dns.auto_dns(dry_run=dry_run)

async def t_zapret_status() -> str:
    s = zapret.get_status()
    return f"Zapret2: installed={s.installed} running={s.running} strategy={s.strategy} version={s.version}"

async def t_zapret_install(dry_run: bool = True) -> str:
    return zapret.install(dry_run=dry_run)

async def t_zapret_strategy(strategy: str = "auto", dry_run: bool = True) -> str:
    if strategy == "auto":
        return zapret.auto_strategy(dry_run=dry_run)
    return zapret.set_strategy(strategy, dry_run=dry_run)

async def t_zapret_toggle(action: str = "restart") -> str:
    if action == "start":
        return zapret.start()
    if action == "stop":
        return zapret.stop()
    return zapret.restart()

async def t_health() -> str:
    return health.health_report()

async def t_system_report() -> str:
    return system.system_report()

async def t_gaming(enabled: bool = True, dry_run: bool = True) -> str:
    return system.gaming_mode(enabled=enabled, dry_run=dry_run)

ALL_TOOLS: list[tuple[str, dict, Any]] = [
    ("clean_all", _schema("clean_all", "Полная чистка системы CachyOS: кэш, orphan, journal, tmp, браузеры, flatpak, docker", {"dry_run": {"type": "boolean"}}), t_clean_all),
    ("clean_cache", _schema("clean_cache", "Чистка кэша pacman (paccache)", {"dry_run": {"type": "boolean"}}), t_clean_cache),
    ("remove_orphans", _schema("remove_orphans", "Удаление orphan-пакетов pacman", {"dry_run": {"type": "boolean"}}), t_remove_orphans),
    ("optimize_mirrors", _schema("optimize_mirrors", "Подбор и установка лучших зеркал pacman (rate-mirrors/reflector/бенчмарк)", {"dry_run": {"type": "boolean"}}), t_mirrors),
    ("optimize_dns", _schema("optimize_dns", "Бенчмарк DNS и установка лучших (Cloudflare/Google/AdGuard/Yandex)", {"dry_run": {"type": "boolean"}}), t_dns),
    ("zapret_status", _schema("zapret_status", "Статус Zapret2: установлен, запущен, стратегия"), t_zapret_status),
    ("zapret_install", _schema("zapret_install", "Установка Zapret2 из git", {"dry_run": {"type": "boolean"}}), t_zapret_install),
    ("zapret_strategy", _schema("zapret_strategy", "Установка стратегии Zapret2 или автоподбор", {"strategy": {"type": "string"}, "dry_run": {"type": "boolean"}}), t_zapret_strategy),
    ("zapret_toggle", _schema("zapret_toggle", "Запуск/остановка/перезапуск Zapret2", {"action": {"type": "string"}}), t_zapret_toggle),
    ("system_health", _schema("system_health", "Проверка здоровья системы: диск, RAM, процессы, проблемы"), t_health),
    ("system_report", _schema("system_report", "Отчёт о системе: OS, CPU, RAM, диск"), t_system_report),
    ("gaming_mode", _schema("gaming_mode", "Игровой режим: CPU performance + gamemode", {"enabled": {"type": "boolean"}, "dry_run": {"type": "boolean"}}), t_gaming),
]
