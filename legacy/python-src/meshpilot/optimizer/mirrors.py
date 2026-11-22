"""Поиск и установка лучших зеркал pacman (CachyOS/Arch).

Стратегии:
1. rate-mirrors (быстрый, Rust) — если установлен
2. reflector — классика Arch
3. Собственный бенчмарк: замер latency + download speed для топ-кандидатов
MeshPilot сам выбирает и пишет /etc/pacman.d/mirrorlist (с sudo).
"""
from __future__ import annotations
import asyncio
import logging
import shutil
import subprocess
import time
from dataclasses import dataclass
from pathlib import Path

import httpx

log = logging.getLogger("meshpilot.mirrors")

@dataclass
class Mirror:
    url: str
    latency_ms: float = 0.0
    speed_mbps: float = 0.0
    score: float = 0.0

CACHYOS_MIRRORS = [
    "https://mirror.cachyos.org/",
    "https://mirror1.cachyos.org/",
    "https://mirror2.cachyos.org/",
    "https://cdn-fastly.mirror.cachyos.org/",
    "https://mirror.kumi.systems/",
    "https://mirror.informatik.tu-freiberg.de/",
    "https://mirror.freedif.org/",
    "https://mirror.rackspace.com/",
    "https://mirrors.edge.kernel.org/",
    "https://mirror.netcologne.de/",
    "https://mirror.yandex.ru/",
    "https://mirror.yandex.kz/",
    "https://mirror.tuna.tsinghua.edu.cn/",
    "https://mirror.nju.edu.cn/",
    "https://mirrors.tuna.tsinghua.edu.cn/",
    "https://mirror.0x0.st/",
    "https://mirror.chaotic.cx/",
    "https://repo.chaotic.cx/",
]

async def _ping(url: str, timeout: float = 5.0) -> float:
    """Latency в мс через HEAD/GET на core.db или корневую страницу."""
    test = url.rstrip("/") + "/core/os/x86_64/core.db"
    start = time.time()
    try:
        async with httpx.AsyncClient(timeout=timeout, follow_redirects=True) as c:
            r = await c.head(test)
            if r.status_code >= 400:
                r = await c.get(url, headers={"Range": "bytes=0-1023"})
            return (time.time() - start) * 1000
    except Exception:
        return 9999.0

async def _speed(url: str, size_kb: int = 512, timeout: float = 10.0) -> float:
    """Скорость загрузки тестового файла, Mbps."""
    test = url.rstrip("/") + "/core/os/x86_64/core.db"
    try:
        async with httpx.AsyncClient(timeout=timeout, follow_redirects=True) as c:
            start = time.time()
            async with c.stream("GET", test) as r:
                n = 0
                async for chunk in r.aiter_bytes(65536):
                    n += len(chunk)
                    if n >= size_kb * 1024:
                        break
            elapsed = max(time.time() - start, 0.01)
            return (n * 8 / elapsed) / 1_000_000
    except Exception:
        return 0.0

async def rank_mirrors(candidates: list[str] | None = None, top_n: int = 10) -> list[Mirror]:
    candidates = candidates or CACHYOS_MIRRORS
    async def _one(url: str) -> Mirror:
        lat = await _ping(url)
        spd = await _speed(url) if lat < 9000 else 0.0
        # score: ниже latency и выше speed — лучше
        score = (1000.0 / max(lat, 1.0)) + spd * 2.0
        return Mirror(url=url, latency_ms=lat, speed_mbps=spd, score=score)
    results = await asyncio.gather(*[_one(u) for u in candidates])
    results = [m for m in results if m.latency_ms < 9000]
    results.sort(key=lambda m: m.score, reverse=True)
    return results[:top_n]

def rank_mirrors_sync(top_n: int = 10) -> list[Mirror]:
    return asyncio.run(rank_mirrors(top_n=top_n))

def apply_mirrors(mirrors: list[Mirror], country: str = "Russia,Germany", dry_run: bool = True) -> str:
    """Записывает mirrorlist. dry_run=True — только показывает."""
    lines = [f"Server = {m.url.rstrip('/')}/core/os/x86_64/core.db" for m in mirrors]
    # pacman mirrorlist использует Server = url/$repo/os/$arch — упрощённо для core
    # реальный формат: Server = https://.../$repo/os/$arch
    lines = [f"Server = {m.url.rstrip('/')}/$repo/os/$arch" for m in mirrors]
    content = "# MeshPilot mirrorlist — сгенерировано автоматически\n" + "\n".join(lines) + "\n"
    if dry_run:
        return f"Dry-run: mirrorlist будет:\n{content}"
    target = Path("/etc/pacman.d/mirrorlist")
    backup = Path("/etc/pacman.d/mirrorlist.bak-meshpilot")
    if target.exists():
        shutil.copy(target, backup)
    # запись через sudo tee
    proc = subprocess.run(["sudo", "tee", str(target)], input=content, text=True, capture_output=True)
    return f"Mirrorlist записан ({len(mirrors)} зеркал). Backup: {backup}\n{proc.stdout}{proc.stderr}"

def auto_mirrors(top_n: int = 10, dry_run: bool = True) -> str:
    """Полный цикл: rate-mirrors -> reflector -> собственный бенчмарк."""
    # 1) rate-mirrors
    if shutil.which("rate-mirrors"):
        proc = subprocess.run(
            ["rate-mirrors", "--allow-root", "--save", "/tmp/meshpilot-mirrors",
             "--max-delay", "21600", "arch", "cachyos"],
            capture_output=True, text=True, timeout=120,
        )
        if proc.returncode == 0 and Path("/tmp/meshpilot-mirrors").exists():
            content = Path("/tmp/meshpilot-mirrors").read_text()
            if dry_run:
                return f"rate-mirrors (dry-run):\n{content[:2000]}"
            subprocess.run(["sudo", "cp", "/tmp/meshpilot-mirrors", "/etc/pacman.d/mirrorlist"], check=False)
            return f"rate-mirrors применён:\n{content[:2000]}"
    # 2) reflector
    if shutil.which("reflector"):
        proc = subprocess.run(
            ["reflector", "--country", "Russia,Germany,France,Netherlands", "--protocol", "https",
             "--latest", "20", "--sort", "rate", "--save", "/tmp/meshpilot-mirrors"],
            capture_output=True, text=True, timeout=120,
        )
        if proc.returncode == 0 and Path("/tmp/meshpilot-mirrors").exists():
            content = Path("/tmp/meshpilot-mirrors").read_text()
            if dry_run:
                return f"reflector (dry-run):\n{content[:2000]}"
            subprocess.run(["sudo", "cp", "/tmp/meshpilot-mirrors", "/etc/pacman.d/mirrorlist"], check=False)
            return f"reflector применён:\n{content[:2000]}"
    # 3) собственный бенчмарк
    mirrors = rank_mirrors_sync(top_n=top_n)
    return apply_mirrors(mirrors, dry_run=dry_run)
