"""Стенд: независимые чтения os.statvfs от того же liveuser во время работы CLI или окна.

Сохраняются только нижняя/верхняя наблюдавшиеся границы, не пути и не история.
Этот процесс не связан с IPC приложения. Вывод остаётся внутри одноразового гостя.
"""
import json
import os
import subprocess
import sys


def root_bytes():
    s = os.statvfs("/")
    total, free, user = (n * s.f_frsize for n in (s.f_blocks, s.f_bfree, s.f_bavail))
    return dict(unit=s.f_frsize, total_bytes=total, used_bytes=total - free,
                available_bytes=user, unavailable_free=free - user)


def witnessed(args):
    low = root_bytes()
    high = low.copy()

    def observe():
        for key, n in root_bytes().items():
            low[key], high[key] = min(low[key], n), max(high[key], n)

    with subprocess.Popen(args, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE) as p:
        while True:
            try:
                out, err = p.communicate(timeout=0.05)
                break
            except subprocess.TimeoutExpired:
                observe()
        observe()
        return {"low": low, "high": high, "returncode": p.returncode, "out": out, "err": err}


if __name__ == "__main__":
    if os.geteuid() == 0:
        sys.exit("Ожидался обычный пользователь")
    print(json.dumps(witnessed(sys.argv[1:])))
