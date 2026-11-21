"""Сравнение задержки двух локальных release-бинарников; не абсолютный порог для CI."""
import argparse
import json
import statistics
import subprocess
import time
from pathlib import Path


def sample(binary):
    start = time.perf_counter_ns()
    output = subprocess.check_output([binary, "status", "--json", "--dry-run"], timeout=10)
    elapsed_ms = (time.perf_counter_ns() - start) / 1_000_000
    status = json.loads(output)
    if status["read_only"] is not True or status["cpu"] is None:
        raise ValueError("Не подтверждён исходный замер CPU")
    return elapsed_ms


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("before", type=Path)
    parser.add_argument("after", type=Path)
    args = parser.parse_args()
    binaries = (args.before.resolve(), args.after.resolve())
    times = ([], [])
    for binary in binaries:
        sample(binary)  # Прогрев не входит в результат.
    for n in range(7):
        for i in ((0, 1) if n % 2 == 0 else (1, 0)):
            times[i].append(sample(binaries[i]))
    for label, ms in zip(("До", "После"), times):
        print(f"{label}: n={len(ms)}, медиана {statistics.median(ms):.2f} мс, min–max {min(ms):.2f}–{max(ms):.2f} мс")
