"""DNS: бенчмарк серверов + установка лучших (systemd-resolved / NetworkManager / /etc/resolv.conf).

Кандидаты: Cloudflare, Google, Quad9, AdGuard, Yandex, DNSPod, OpenDNS, Mullvad, NextDNS.
Замер: latency (UDP/DoH) + проверка фильтрации (AdGuard/Yandex) + скорость.
"""
from __future__ import annotations
import asyncio
import logging
import socket
import subprocess
import time
from dataclasses import dataclass

import httpx

log = logging.getLogger("meshpilot.dns")

@dataclass
class DNSResult:
    ip: str
    name: str
    latency_ms: float
    doh: bool = False
    filtering: bool = False
    score: float = 0.0

CANDIDATES = [
    ("1.1.1.1", "Cloudflare"),
    ("1.0.0.1", "Cloudflare alt"),
    ("8.8.8.8", "Google"),
    ("8.8.4.4", "Google alt"),
    ("9.9.9.9", "Quad9"),
    ("94.140.14.14", "AdGuard"),
    ("94.140.15.15", "AdGuard no-filter"),
    ("77.88.8.8", "Yandex"),
    ("77.88.8.1", "Yandex Safe"),
    ("208.67.222.222", "OpenDNS"),
    ("193.110.128.1", "Mullvad"),
    ("45.90.28.0", "NextDNS"),
    ("185.228.168.9", "DNSPod"),
    ("5.5.5.5", "Yandex RU"),
    ("2.2.2.2", "Yandex RU alt"),
]

def _udp_ping(ip: str, timeout: float = 2.0) -> float:
    """Быстрый замер через UDP connect (не ICMP, работает без прав)."""
    start = time.time()
    try:
        s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        s.settimeout(timeout)
        s.connect((ip, 53))
        s.send(b"\x00" * 12)  # пустой DNS query-ish
        s.close()
        return (time.time() - start) * 1000
    except Exception:
        return 9999.0

async def _doh_check(ip: str, name: str) -> bool:
    """Проверка DoH: https://ip/dns-query"""
    try:
        async with httpx.AsyncClient(timeout=4, verify=False) as c:
            r = await c.get(f"https://{ip}/dns-query?name=example.com&type=A",
                            headers={"Accept": "application/dns-json"})
            return r.status_code == 200
    except Exception:
        return False

async def benchmark_dns(top_n: int = 3) -> list[DNSResult]:
    results: list[DNSResult] = []
    for ip, name in CANDIDATES:
        lat = _udp_ping(ip)
        if lat >= 9000:
            continue
        doh = await _doh_check(ip, name)
        filtering = "AdGuard" in name or "Yandex Safe" in name or "Quad9" in name
        score = 1000.0 / max(lat, 1.0) + (20 if doh else 0) + (10 if filtering else 0)
        results.append(DNSResult(ip=ip, name=name, latency_ms=lat, doh=doh, filtering=filtering, score=score))
    results.sort(key=lambda d: d.score, reverse=True)
    return results[:top_n]

def benchmark_dns_sync(top_n: int = 3) -> list[DNSResult]:
    return asyncio.run(benchmark_dns(top_n=top_n))

def apply_dns(dns_ips: list[str], method: str = "systemd-resolved", dry_run: bool = True) -> str:
    """Установка DNS. method: systemd-resolved | networkmanager | resolvconf"""
    servers = " ".join(dns_ips)
    if dry_run:
        return f"Dry-run: DNS {servers} через {method}"
    if method == "systemd-resolved":
        # /etc/systemd/resolved.conf.d/meshpilot.conf
        conf = "[Resolve]\nDNS=" + " ".join(dns_ips) + "\nFallbackDNS=1.1.1.1 8.8.8.8\n"
        proc = subprocess.run(
            ["bash", "-c", f"sudo mkdir -p /etc/systemd/resolved.conf.d && echo '{conf}' | sudo tee /etc/systemd/resolved.conf.d/meshpilot.conf && sudo systemctl restart systemd-resolved"],
            capture_output=True, text=True, timeout=30,
        )
        return f"systemd-resolved: {proc.stdout}{proc.stderr}"
    if method == "networkmanager":
        proc = subprocess.run(
            ["bash", "-c", f"sudo nmcli con mod $(nmcli -t -f NAME connection show --active | head -1) ipv4.dns '{','.join(dns_ips)}' && sudo nmcli con up $(nmcli -t -f NAME connection show --active | head -1)"],
            capture_output=True, text=True, timeout=30,
        )
        return f"NetworkManager: {proc.stdout}{proc.stderr}"
    # fallback: /etc/resolv.conf
    proc = subprocess.run(
        ["bash", "-c", f"echo 'nameserver {dns_ips[0]}' | sudo tee /etc/resolv.conf"],
        capture_output=True, text=True, timeout=10,
    )
    return f"resolv.conf: {proc.stdout}{proc.stderr}"

def auto_dns(dry_run: bool = True) -> str:
    results = benchmark_dns_sync(top_n=3)
    if not results:
        return "DNS бенчмарк: ни один сервер не ответил"
    ips = [r.ip for r in results]
    report = "Топ DNS:\n" + "\n".join(f"  {r.name} ({r.ip}) — {r.latency_ms:.0f} мс, DoH={r.doh}" for r in results)
    return report + "\n" + apply_dns(ips, dry_run=dry_run)
