"""Внешняя пара чтений procfs. Не создаёт тестовый трафик, не публикует имён и счётчиков."""
from datetime import datetime, timezone
from math import isclose
from pathlib import Path
import re
import time

from checks import check, verify_stamp


def net_ticks():
    with Path("/proc/net/dev").open("rb") as f:
        text = f.read(65537)
    check(len(text) <= 65536 and text.endswith(b"\n"), "PROC_NET")
    result = {}
    for row in text.splitlines()[2:]:
        name, fields = row.split(b":", 1)
        parts = fields.split()
        name = name.strip().decode("ascii")
        check(len(parts) == 16 and name not in result, "PROC_NET")
        result[name] = int(parts[0]), int(parts[8])
    return result


def verify_cli_net(sample, started, elapsed_ms, before, after):
    check(isinstance(sample, dict) and sample.get("source") == "/proc/net/dev", "CLI_NET_SOURCE")
    ms, devices = sample.get("interval_ms"), sample.get("interfaces")
    check(type(ms) is int and 500 <= ms <= min(5000, elapsed_ms), "CLI_NET_INTERVAL")
    verify_stamp(sample.get("observed_at_utc"), started, "CLI_NET_TIME")
    # На стенде набор устройств стабилен. Не соглашаемся на пустой/частичный список вместо него.
    check(isinstance(devices, list) and all(isinstance(d, dict) for d in devices) and
          [d.get("name") for d in devices] == sorted(after) and before.keys() == after.keys(), "CLI_NET_INTERFACES")
    for dev in devices:
        name, rates = dev["name"], dev.get("rates")
        check(isinstance(rates, dict), "CLI_NET_RATES")
        for i, direction in enumerate(("rx", "tx")):
            delta, rate = rates.get(direction+"_delta_bytes"), rates.get(direction+"_bytes_per_sec")
            check(type(delta) is int and 0 <= delta <= min(2**53-1, after[name][i]-before[name][i]), "CLI_NET_DELTA")
            check(type(rate) in (int, float) and isclose(rate, 1000*delta/ms, rel_tol=1e-12), "CLI_NET_RATE")


def verify_shared_window(cpu, net):
    # После проверки обеих меток времени: замеры в status должны перекрываться,
    # а не добавлять сетевую паузу после CPU. Не ставим предел времени запуска процесса.
    ends = [round(datetime.fromisoformat(s["observed_at_utc"]).timestamp() * 1000) for s in (cpu, net)]
    starts = [e - s["interval_ms"] for e, s in zip(ends, (cpu, net))]
    check(min(ends) - max(starts) >= 450, "CLI_ACTIVITY_SERIAL")


def verify_loopback_card(label, started, elapsed_ms, before, after):
    # У гостя нет NIC. Проверяется настоящий loopback, не скорость доступа в интернет.
    check(set(before) == set(after) == {"lo"}, "GUI_NET_DEVICES")
    quantity = r"(\d+(?:,\d{1,2})?) (Б|КиБ|МиБ|ГиБ|ТиБ|ПиБ)/с"
    label = label.replace("\xa0", "").replace("\u202f", "")
    m = re.fullmatch(r"Трафик сети\s+lo\s+только локальная петля\s+Приём\s+" + quantity +
                     r"\s+Передача\s+" + quantity + r"\s+за (\d{1,4}) мс\s+/proc/net/dev\s+"
                     r"(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}) UTC", label)
    check(m, "GUI_NET_LABEL")
    ms = int(m[5]); check(500 <= ms <= min(5000, elapsed_ms), "GUI_NET_INTERVAL")
    for i, (value, unit) in enumerate([(m[1], m[2]), (m[3], m[4])]):
        n = float(value.replace(",", "."))
        scale = 1024 ** ("Б", "КиБ", "МиБ", "ГиБ", "ТиБ", "ПиБ").index(unit)
        delta = after["lo"][i] - before["lo"][i]
        check(delta >= 0 and (scale == 1 or n >= 1) and n <= 1024 and
              n <= (1000*delta/ms)/scale + 0.005001, "GUI_NET_RATE")
    seconds = datetime.fromisoformat(m[6]).replace(tzinfo=timezone.utc).timestamp()
    check(started <= seconds <= time.time(), "GUI_NET_TIME")
