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


def verify_cli_root_fs(sample, started, low, high):
    check(isinstance(sample, dict) and sample.get("source") == "statvfs(/)", "CLI_FS_SOURCE")
    check(low["unit"] == high["unit"] > 0 and low["total_bytes"] == high["total_bytes"] > 0, "FS_WITNESS")
    for key in ("total_bytes", "used_bytes", "available_bytes"):
        n = sample.get(key)
        check(type(n) is int and n >= 0 and n % low["unit"] == 0 and low[key] <= n <= high[key], "CLI_FS_BYTES")
    # Если часть свободных блоков недоступна, ни used, ни available не должны их включать.
    other = sample["total_bytes"] - sample["used_bytes"] - sample["available_bytes"]
    check(0 <= low["unavailable_free"] <= other <= high["unavailable_free"], "CLI_FS_ACCOUNTING")
    verify_stamp(sample.get("observed_at_utc"), started, "CLI_FS_TIME")


UPTIME_UNITS = {"д": 86400, "ч": 3600, "м": 60, "с": 1}


def parse_uptime_value(text):
    # «3 ч 18 м 51 с» — пары число/единица; показ всегда заканчивается секундами.
    tokens = text.split()
    check(len(tokens) >= 2 and len(tokens) % 2 == 0 and tokens[-1] == "с", "GUI_UPTIME_VALUE")
    total = 0
    for i in range(0, len(tokens), 2):
        number, unit = tokens[i], tokens[i + 1]
        check(number.isdigit() and unit in UPTIME_UNITS, "GUI_UPTIME_VALUE")
        total += int(number) * UPTIME_UNITS[unit]
    return total


def verify_cli_uptime(observation, started, low, high):
    check(isinstance(observation, dict) and observation.get("source") == "/proc/uptime", "CLI_UPTIME_SOURCE")
    seconds = observation.get("uptime_seconds")
    check(type(seconds) is int and seconds >= 0 and low <= seconds <= high, "CLI_UPTIME_BRACKET")
    verify_stamp(observation.get("observed_at_utc"), started, "CLI_UPTIME_TIME")


def verify_uptime_card(label, started, low, high):
    # Доступное имя строки панели «Среда» из видимых dt, значения, источника и time.
    label = label.replace("\xa0", "").replace("\u202f", "")
    match = re.fullmatch(r"Время работы\s+(.+)\s+/proc/uptime\s+"
                         r"(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}) UTC", label)
    check(match, "GUI_UPTIME_LABEL")
    seconds = parse_uptime_value(match.group(1))
    check(low <= seconds <= high, "GUI_UPTIME_BRACKET")
    stamp = datetime.fromisoformat(match.group(2)).replace(tzinfo=timezone.utc).timestamp()
    check(started <= stamp <= time.time(), "GUI_UPTIME_TIME")


def proc_kernel():
    # Независимое чтение второго слова /proc/version, без uname и shell.
    raw = Path("/proc/version").read_bytes()
    body = raw[:-1] if raw.endswith(b"\n") else raw
    check(b"\n" not in body, "PROC_KERNEL")
    fields = body.split()
    check(len(fields) >= 3 and fields[0] == b"Linux" and fields[1] == b"version", "PROC_KERNEL")
    version = fields[2]
    check(version and len(version) <= 64 and version.isascii()
          and all(0x21 <= b <= 0x7E for b in version), "PROC_KERNEL")
    return version.decode("ascii")


def verify_cli_kernel(observation, started, low, high):
    # Версия ядра постоянна за загрузку: границы наблюдений совпадают.
    check(isinstance(observation, dict) and observation.get("source") == "/proc/version", "CLI_KERNEL_SOURCE")
    version = observation.get("version")
    check(isinstance(version, str) and 1 <= len(version) <= 64
          and all(0x21 <= ord(c) <= 0x7E for c in version) and low <= version <= high, "CLI_KERNEL_VERSION")
    verify_stamp(observation.get("observed_at_utc"), started, "CLI_KERNEL_TIME")


def verify_kernel_card(label, started, low, high):
    # Доступное имя строки «Ядро» панели «Среда» из видимых dt, значения, источника и time.
    label = label.replace("\xa0", "").replace("\u202f", "")
    match = re.fullmatch(r"Ядро\s+([!-~]{1,64})\s+/proc/version\s+"
                         r"(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}) UTC", label)
    check(match, "GUI_KERNEL_LABEL")
    check(low <= match.group(1) <= high, "GUI_KERNEL_VERSION")
    stamp = datetime.fromisoformat(match.group(2)).replace(tzinfo=timezone.utc).timestamp()
    check(started <= stamp <= time.time(), "GUI_KERNEL_TIME")


def verify_root_fs_card(label, started, low, high):
    quantity = r"([0-9]+(?:,[0-9]{1,2})?) (Б|КиБ|МиБ|ГиБ|ТиБ|ПиБ)"
    label = label.replace("\xa0", "").replace("\u202f", "")
    match = re.fullmatch(r"Занято на /\s+" + quantity + r"\s+из\s+" + quantity +
                         r"\s+Доступно пользователю:\s+" + quantity +
                         r"\s+statvfs\(/\)\s+(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}) UTC", label)
    check(match, "GUI_FS_LABEL")
    fields = match.groups()
    check(low["total_bytes"] == high["total_bytes"] > 0, "FS_WITNESS")
    for i, key in enumerate(("used_bytes", "total_bytes", "available_bytes")):
        value, unit = fields[2*i:2*i + 2]
        scale = 1024 ** ("Б", "КиБ", "МиБ", "ГиБ", "ТиБ", "ПиБ").index(unit)
        n = float(value.replace(",", "."))
        check((unit == "Б" and n.is_integer() and n < 1024) or (unit != "Б" and 1 <= n <= 1024), "GUI_FS_UNIT")
        # Двоичные единицы с округлением до сотых. Не сравнивать с более ранним CLI.
        check(low[key] / scale - 0.005001 <= n <= high[key] / scale + 0.005001, "GUI_FS_BYTES")
    seconds = datetime.fromisoformat(fields[-1]).replace(tzinfo=timezone.utc).timestamp()
    check(started <= seconds <= time.time(), "GUI_FS_TIME")
