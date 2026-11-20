"""Один архивный ISO, только HTTPS. Результат ещё обязан пройти SHA-256 и gpgv в media.sh."""
import re
import shutil
import subprocess
import sys
import tempfile
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

URL = "https://downloads.sourceforge.net/project/cachyos-arch/gui-installer/desktop/251129/cachyos-desktop-linux-251129.iso"
LIMIT = 5 * 1024**3


class NoRange(Exception):
    pass


def ranges(total):
    if not 1024**2 <= total <= LIMIT:
        raise ValueError("Недопустимый размер ISO")
    return [(total*i//4, total*(i+1)//4 - 1) for i in range(4)]


def verify_range(reply, size, first, last, total):
    m = re.fullmatch(r"206 bytes ([0-9]{1,10})-([0-9]{1,10})/([0-9]{1,10})", reply.strip(), re.I)
    if not m:
        raise ValueError("Нет однозначного ответа 206 с Content-Range")
    left, right, length = map(int, m.groups())
    if (left, right) != (first, last) or size != last - first + 1 or not right < length <= LIMIT:
        raise ValueError("Получен другой диапазон или размер")
    if total is not None and total != length:
        raise ValueError("Размер изменился между запросами")
    return length


def part(path, first, last, total):
    seconds = 25 if total is None else 720
    result = subprocess.run(["curl", "-q", "--fail", "--silent", "--show-error", "--location",
                             "--proto", "=https", "--proto-redir", "=https", "--connect-timeout", "12",
                             "--max-time", str(seconds), "--speed-limit", "32768", "--speed-time", "45",
                             "--range", f"{first}-{last}", "--max-filesize", str(last-first+1), "-o", path,
                             "-w", "%{http_code} %header{content-range}", URL],
                            stdout=subprocess.PIPE, text=True, timeout=seconds+8)
    size = path.stat().st_size if path.exists() else 0
    if total is None and result.stdout.startswith("200 "):
        raise NoRange()
    if result.returncode:
        raise ValueError(f"Диапазон {first}–{last}: curl={result.returncode}, получено {size} байт")
    return verify_range(result.stdout, size, first, last, total)


def download(dest):
    with tempfile.TemporaryDirectory(dir=dest) as tmp:
        work = Path(tmp)
        total = part(work / "probe", 0, 0, None)
        spans = ranges(total)
        print(f"::notice file=tests/cachyos-vm/archive.py::Архив сообщил размер {total} байт, четыре диапазона", flush=True)
        files = [work / str(i) for i in range(len(spans))]
        with ThreadPoolExecutor(max_workers=4) as pool:
            jobs = [pool.submit(part, path, lo, hi, total) for path, (lo, hi) in zip(files, spans)]
            for job in jobs:
                job.result()
        with files[0].open("ab") as target:
            for path in files[1:]:
                with path.open("rb") as source:
                    shutil.copyfileobj(source, target, length=1024**2)
                path.unlink()
        if files[0].stat().st_size != total:
            raise ValueError("Неполная сборка диапазонов")
        # Не перезаписывать существующую фикстуру. Оба пути на одной файловой системе.
        (dest / "media.iso").hardlink_to(files[0])


if __name__ == "__main__":
    try:
        download(Path(sys.argv[1]))
    except NoRange:
        print("Архив не поддержал Range; допустима обычная ограниченная передача")
        sys.exit(75)
    except (ValueError, OSError, subprocess.SubprocessError) as e:
        print(f"::error file=tests/cachyos-vm/archive.py::Загрузка архива не завершена: {e}")
        sys.exit(1)
