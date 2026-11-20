"""UART принадлежит только этому запуску. Условие успеха — весь протокол, не выход QEMU."""
import re
import sys
from pathlib import Path

EXPECTED = ["GUEST", "CLI", "SESSION", "WINDOW", "UNCHANGED", "PASS"]


def verdict(lines, run_id, qemu_status):
    prefix = f"MESHPILOT/1 {run_id} "
    stages = [s[len(prefix):] for s in lines if s.startswith(prefix)]
    if qemu_status != 0 or stages != EXPECTED:
        # Не передавать произвольный текст гостя в аннотации/команды Actions.
        safe = [s for s in stages if s in EXPECTED or re.fullmatch(r"FAIL [A-Z_]+(?: lib[\w.+-]{1,80}){0,20}", s)]
        raise ValueError(f"QEMU={qemu_status}; подтверждения: {', '.join(safe) or 'нет'}")


if __name__ == "__main__":
    path, run_id, code = sys.argv[1:]
    try:
        with Path(path).open("rb") as f:
            raw = f.read(8193)
        if len(raw) > 8192:
            raise ValueError("Отчёт превышает лимит")
        verdict(raw.decode("ascii", errors="replace").splitlines(), run_id, int(code))
    except (ValueError, OSError) as error:
        print(f"::error file=tests/cachyos-vm/guest.py,title=CachyOS VM::{error}")
        sys.exit(1)
    print("::notice file=tests/cachyos-vm/guest.py,title=CachyOS VM::CachyOS Live: CLI, окно с совпадающим PID и сравнение состояния прошли")
