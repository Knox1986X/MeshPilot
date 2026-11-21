"""Стенд: независимые чтения os.statvfs, /proc/uptime и /proc/version от того же liveuser во время работы CLI или окна.

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


def parse_kernel(raw):
    """Версия — второе слово единственной строки «Linux version <версия> ...»."""
    body = raw[:-1] if raw.endswith(b"\n") else raw
    if b"\n" in body:
        raise ValueError("kernel table")
    fields = body.split()
    if len(fields) < 3 or fields[0] != b"Linux" or fields[1] != b"version":
        raise ValueError("kernel head")
    version = fields[2]
    if not version or len(version) > 64 or not version.isascii() \
            or not all(0x21 <= b <= 0x7E for b in version):
        raise ValueError("kernel version")
    return version.decode("ascii")


def kernel_version():
    return parse_kernel(Path("/proc/version").read_bytes())


def parse_os_version(raw):
    """Поле VERSION= os-release: ровно одна строка, дословное значение.

    Правила совпадают с адаптером приложения: необязательная пара кавычек,
    печатные ASCII без кавычек и краевых пробелов, длина 1–64.
    """
    if len(raw) > 65536:
        raise ValueError("os version size")
    text = raw.decode("utf-8")
    values = []
    for line in text.split("\n"):
        if line.endswith("\r"):
            line = line[:-1]
        key, sep, value = line.partition("=")
        if sep and key == "VERSION":
            values.append(value)
    if len(values) != 1:
        raise ValueError("os version field")
    raw_value = values[0]
    quote = raw_value[0] if raw_value[:1] in {'"', "'"} else None
    if quote:
        if len(raw_value) < 2 or not raw_value.endswith(quote):
            raise ValueError("os version quotes")
        value = raw_value[1:-1]
    else:
        value = raw_value
    if not 1 <= len(value) <= 64:
        raise ValueError("os version length")
    if any(b < 0x20 or b > 0x7E for b in value.encode("ascii")):
        raise ValueError("os version charset")
    if '"' in value or "'" in value:
        raise ValueError("os version quote")
    if value.startswith(" ") or value.endswith(" "):
        raise ValueError("os version pad")
    return value


def os_version():
    """Поле VERSION или None — отсутствия/некорректность — честный исход, не сбой."""
    try:
        return parse_os_version(Path("/etc/os-release").read_bytes())
    except (OSError, ValueError, UnicodeDecodeError):
        return None


def witnessed(args):
    low = root_bytes()
    high = low.copy()
    up = uptime_seconds()
    up_lo, up_hi = up, up
    kr = kernel_version()
    kr_lo, kr_hi = kr, kr
    ov = os_version()

    def observe():
        nonlocal up_lo, up_hi, kr_lo, kr_hi
        for key, n in root_bytes().items():
            low[key], high[key] = min(low[key], n), max(high[key], n)
        u = uptime_seconds()
        up_lo, up_hi = min(up_lo, u), max(up_hi, u)
        k = kernel_version()
        kr_lo, kr_hi = min(kr_lo, k), max(kr_hi, k)
        # Версия за загрузку неизменна: смена значения — сбой стенда.
        if ov is not None and os_version() != ov:
            raise ValueError("os version changed")

    with subprocess.Popen(args, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE) as p:
        while True:
            try:
                out, err = p.communicate(timeout=0.05)
                break
            except subprocess.TimeoutExpired:
                observe()
        observe()
        return {"low": low, "high": high, "up_lo": up_lo, "up_hi": up_hi,
                "kr_lo": kr_lo, "kr_hi": kr_hi, "ov": ov,
                "returncode": p.returncode, "out": out, "err": err}


if __name__ == "__main__":
    if os.geteuid() == 0:
        sys.exit("Ожидался обычный пользователь")
    print(json.dumps(witnessed(sys.argv[1:])))
