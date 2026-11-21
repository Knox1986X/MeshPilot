"""Утверждения гостевого стенда. Значения procfs и содержимое дерева окна не выводятся."""
from datetime import datetime, timezone
from math import isclose
from pathlib import Path
import re
import time


class Failure(Exception):
    """Короткий код без секретов для последовательного порта."""


def check(condition, code):
    if not condition:
        raise Failure(code)


def verify_stamp(text, started, code):
    check(isinstance(text, str) and re.fullmatch(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z", text), code)
    check(started // 1_000_000 <= int(datetime.fromisoformat(text).timestamp() * 1000 + 0.5)
          <= time.time_ns() // 1_000_000, code)


def proc_total():
    match = re.search(r"^MemTotal:\s*(\d+) kB$", Path("/proc/meminfo").read_text(), re.MULTILINE)
    check(match, "PROC_TOTAL")
    return int(match[1]) * 1024


def verify_cli_ram(ram, started):
    check(isinstance(ram, dict) and ram.get("source") == "/proc/meminfo", "CLI_RAM_SOURCE")
    check(type(ram.get("total_bytes")) is int and ram["total_bytes"] == proc_total() and
          type(ram.get("used_bytes")) is int and 0 <= ram["used_bytes"] <= ram["total_bytes"], "CLI_RAM_BYTES")
    verify_stamp(ram.get("observed_at_utc"), started, "CLI_RAM_TIME")


def verify_ram_card(label, started):
    # Доступное имя собрано aria-labelledby из видимых заголовка, чисел, источника и time.
    # Не сравнивать занятое между двумя чтениями: запуск WebView расходует память.
    label = label.replace("\xa0", "").replace("\u202f", "")  # разделитель групп ru-RU
    quantity = r"([0-9]+(?:,[0-9]{1,2})?) (Б|КиБ|МиБ|ГиБ|ТиБ|ПиБ)"
    match = re.fullmatch(r"Занято RAM\s+" + quantity + r"\s+из\s+" + quantity +
                         r"\s+/proc/meminfo\s+(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}) UTC", label)
    check(match, "GUI_RAM_LABEL")
    units = ["Б", "КиБ", "МиБ", "ГиБ", "ТиБ", "ПиБ"]
    used, unit, total, total_unit, stamp = match.groups()
    total_scale = 1024 ** units.index(total_unit)
    check(abs(float(total.replace(",", ".")) - proc_total() / total_scale) <= 0.005001, "GUI_RAM_TOTAL")
    scale = 1024 ** units.index(unit)
    check(0 <= float(used.replace(",", ".")) <= proc_total() / scale + 0.005001, "GUI_RAM_USED")
    seconds = datetime.fromisoformat(stamp).replace(tzinfo=timezone.utc).timestamp()
    check(started <= seconds <= time.time(), "GUI_RAM_TIME")


def proc_cpu_ticks():
    # Независимые захваты до запуска и после выхода CLI дают внешние границы его интервала.
    with Path("/proc/stat").open("rb") as f:
        fields = f.readline(513).split()
    check(fields[:1] == [b"cpu"] and 9 <= len(fields) <= 11, "PROC_CPU")
    ticks = list(map(int, fields[1:9]))
    idle = ticks[3] + ticks[4]
    return sum(ticks) - idle, idle


def verify_cli_cpu(cpu, started, elapsed_ms, before, after):
    check(isinstance(cpu, dict) and cpu.get("source") == "/proc/stat", "CLI_CPU_SOURCE")
    busy, total, pct, ms = (cpu.get(k) for k in ("busy_ticks", "total_ticks", "usage_percent", "interval_ms"))
    check(type(total) is int and type(busy) is int and 0 <= busy <= total and total > 0, "CLI_CPU_TICKS")
    check(busy <= after[0] - before[0] and total - busy <= after[1] - before[1], "CLI_CPU_BRACKET")
    check(type(pct) in (float, int) and 0 <= pct <= 100 and isclose(pct, 100 * busy / total, abs_tol=1e-9), "CLI_CPU_PERCENT")
    check(type(ms) is int and 500 <= ms <= min(5000, elapsed_ms), "CLI_CPU_INTERVAL")
    verify_stamp(cpu.get("observed_at_utc"), started, "CLI_CPU_TIME")


def verify_cpu_card(label, started, elapsed_ms):
    match = re.fullmatch(r"Загрузка CPU\s+(\d{1,3}(?:,\d)?) % за (\d{1,4}) мс\s+"
                         r"/proc/stat\s+(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}) UTC", label)
    check(match, "GUI_CPU_LABEL")
    pct, interval, stamp = match.groups()
    check(0 <= float(pct.replace(",", ".")) <= 100, "GUI_CPU_PERCENT")
    check(500 <= int(interval) <= min(5000, elapsed_ms), "GUI_CPU_INTERVAL")
    seconds = datetime.fromisoformat(stamp).replace(tzinfo=timezone.utc).timestamp()
    check(started <= seconds <= time.time(), "GUI_CPU_TIME")
