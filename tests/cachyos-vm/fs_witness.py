"""Стенд: независимые чтения os.statvfs и /proc/uptime от того же liveuser во время работы CLI или окна.

Сохраняются только нижняя/верхняя наблюдавшиеся границы, не пути и не история.
Этот процесс не связан с IPC приложения. Вывод остаётся внутри одноразового гостя.
"""
import json
import os
import subprocess
import sys
from pathlib import Path


def root_bytes():
    s = os.statvfs("/")
    total, free, user = (n * s.f_frsize for n in (s.f_blocks, s.f_bfree, s.f_bavail))
    return dict(unit=s.f_frsize, total_bytes=total, used_bytes=total - free,
                available_bytes=user, unavailable_free=free - user)


def parse_uptime(raw):
    """Целые секунды из первого поля «digits(.digits)?»; таблица ровно из двух полей."""
    body = raw[:-1] if raw.endswith(b"\n") else raw
    first, sep, rest = body.partition(b" ")
    if not sep or b" " in rest:
        raise ValueError("uptime table")
    integer, dot, frac = first.partition(b".")
    if not integer or not integer.isascii() or not integer.isdigit():
        raise ValueError("uptime field")
    if dot and not frac.isdigit():
        raise ValueError("uptime fraction")
    value = int(integer)
    if value >= 2**64:
        raise ValueError("uptime range")
    return value


def uptime_seconds():
    return parse_uptime(Path("/proc/uptime").read_bytes())


def witnessed(args):
    low = root_bytes()
    high = low.copy()
    up = uptime_seconds()
    up_lo, up_hi = up, up

    def observe():
        nonlocal up_lo, up_hi
        for key, n in root_bytes().items():
            low[key], high[key] = min(low[key], n), max(high[key], n)
        u = uptime_seconds()
        up_lo, up_hi = min(up_lo, u), max(up_hi, u)

    with subprocess.Popen(args, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE) as p:
        while True:
            try:
                out, err = p.communicate(timeout=0.05)
                break
            except subprocess.TimeoutExpired:
                observe()
        observe()
        return {"low": low, "high": high, "up_lo": up_lo, "up_hi": up_hi,
                "returncode": p.returncode, "out": out, "err": err}


if __name__ == "__main__":
    if os.geteuid() == 0:
        sys.exit("Ожидался обычный пользователь")
    print(json.dumps(witnessed(sys.argv[1:])))
