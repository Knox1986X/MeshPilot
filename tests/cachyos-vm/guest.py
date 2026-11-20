"""Работает только на Live ISO в выделенной VM, запущенной run.sh. Не устанавливает пакеты."""
import hashlib
import json
import os
import pwd
import re
import stat
import subprocess
import tempfile
import time
from pathlib import Path

MEDIA = Path("/run/meshpilot-smoke")


class Failure(Exception):
    """Короткий код без секретов для последовательного порта."""


def check(condition, code):
    if not condition:
        raise Failure(code)


def command(args, **kwargs):
    return subprocess.run(args, capture_output=True, text=True, timeout=kwargs.pop("timeout", 12),
                          check=True, **kwargs).stdout


def readonly_cd(path):
    info = command(["findmnt", "-n", "-o", "FSTYPE,OPTIONS", "--mountpoint", path]).split()
    return len(info) == 2 and info[0] == "iso9660" and "ro" in info[1].split(",")


def guard():
    check(os.geteuid() == 0 and Path("/sys/class/dmi/id/product_name").read_text().strip() == "MeshPilot-smoke", "VM_GUARD")
    check(command(["systemd-detect-virt", "--vm"]).strip() in {"kvm", "qemu"}, "VM_GUARD")
    check(readonly_cd(MEDIA) and readonly_cd("/run/archiso/bootmnt"), "MEDIA_GUARD")
    check("cachyos" in os.uname().release and set(os.listdir("/sys/class/net")) == {"lo"}, "GUEST_ENV")


def system_state():
    """Только сравнение. Содержимое, адреса и хеши не покидают гостя."""
    h = hashlib.sha256()

    def add(blob):
        h.update(len(blob).to_bytes(8, "big")); h.update(blob)

    def visit(p):
        add(bytes(p))
        try:
            mode = p.lstat().st_mode
        except FileNotFoundError:
            add(b"missing"); return
        add(str(mode).encode())
        if stat.S_ISLNK(mode):
            add(os.fsencode(os.readlink(p)))
        elif stat.S_ISDIR(mode):
            for child in sorted(p.iterdir()):
                visit(child)
        elif stat.S_ISREG(mode):
            with p.open("rb") as source:
                add(hashlib.file_digest(source, "sha256").digest())
        else:
            raise Failure("UNEXPECTED_FILE")

    for path in ("/var/lib/pacman", "/etc/pacman.conf", "/etc/resolv.conf", "/etc/NetworkManager",
                 "/etc/systemd/network", "/etc/systemd/resolved.conf", "/etc/systemd/resolved.conf.d"):
        visit(Path(path))
    for path in sorted(Path("/etc/pacman.d").glob("*mirrorlist*")):
        visit(path)
    add(Path("/etc/resolv.conf").read_bytes())  # И ссылка, и фактическое содержимое.
    for family in ["-4", "-6"]:
        add(command(["ip", "-j", family, "route", "show", "table", "all"]).encode())
    add(command(["ip", "-brief", "address"]).encode())
    return h.digest()


def run_checks(emit, home):
    account = pwd.getpwnam("liveuser")
    check(account.pw_uid > 0, "LIVE_UID")
    os.chown(home, account.pw_uid, account.pw_gid)
    runtime = f"/run/user/{account.pw_uid}"
    env = {"HOME": home, "USER": "liveuser", "LOGNAME": "liveuser", "PATH": "/usr/bin", "LC_ALL": "C",
           "XDG_RUNTIME_DIR": runtime, "DBUS_SESSION_BUS_ADDRESS": f"unix:path={runtime}/bus"}
    for key, suffix in [("CONFIG", "config"), ("DATA", "data"), ("CACHE", "cache"), ("STATE", "state")]:
        env[f"XDG_{key}_HOME"] = f"{home}/{suffix}"

    def user(args, **kwargs):
        return command(["setpriv", f"--reuid={account.pw_uid}", f"--regid={account.pw_gid}", "--init-groups", "--no-new-privs", "--", *args], env=env, **kwargs)

    # Только неизменившиеся данные реального образа. Не подменять os-release в тесте.
    raw_id = __import__("platform").freedesktop_os_release()["ID"]
    status = json.loads(user([MEDIA / "meshpilot", "status", "--json"]))
    check(status["read_only"] is True and status["os_id"] == raw_id and status["cachyos"] is True, "CLI_PLATFORM")
    check(all(status[k] is None for k in ("online", "dns", "updatable_apps", "security_fixes", "last_success_utc")), "CLI_OBSERVATIONS")
    doctor = json.loads(user([MEDIA / "meshpilot", "doctor", "--json", "--dry-run"]))
    checks = {c["id"]: c["finding"] for c in doctor["checks"]}
    check(doctor["read_only"] is True and checks["os"] == checks["pacman"] == "detected", "CLI_DOCTOR")
    check(not any(Path(home).iterdir()), "CLI_XDG_WRITE")
    emit("CLI")

    linked = user(["ldd", MEDIA / "meshpilot-desktop"])
    missing = re.findall(r"^\s*(lib[\w.+-]+) => not found$", linked, re.MULTILINE)
    if missing:
        raise Failure("GUI_RUNTIME " + " ".join(sorted(set(missing))[:20]))
    # Используем DISPLAY именно гостевого сеанса Plasma, не передаём окружение хоста.
    deadline = time.monotonic() + 100
    while time.monotonic() < deadline:
        try:
            values = dict(line.split("=", 1) for line in user(["systemctl", "--user", "show-environment"]).splitlines() if "=" in line)
            if re.fullmatch(r":\d+(?:\.\d+)?", values.get("DISPLAY", "")):
                env["DISPLAY"] = values["DISPLAY"]
                env["XAUTHORITY"] = values.get("XAUTHORITY", account.pw_dir + "/.Xauthority")
                break
        except subprocess.CalledProcessError:
            pass
        time.sleep(1)
    else:
        raise Failure("NO_GRAPHICAL_SESSION")
    env.update(GDK_BACKEND="x11", WEBKIT_DISABLE_DMABUF_RENDERER="1", LIBGL_ALWAYS_SOFTWARE="1")
    try:
        user([MEDIA / "native-window.sh", MEDIA / "meshpilot-desktop", MEDIA / "window-observer"], timeout=55)
    except subprocess.SubprocessError as err:
        raise Failure("GUI_WINDOW") from err
    emit("WINDOW")


def main():
    # Никаких эффектов на хосте при ошибочном ручном запуске.
    guard()
    run_id = (MEDIA / "run-id").read_text().strip()
    check(re.fullmatch(r"[0-9a-f]{32}", run_id), "RUN_ID")
    with open("/dev/ttyS1", "w", buffering=1, encoding="ascii") as serial:
        def emit(event):
            print(f"MESHPILOT/1 {run_id} {event}", file=serial, flush=True)
        emit("GUEST")
        try:
            # После штатной загрузки, не сравнивать с состоянием в середине инициализации ОС.
            deadline = time.monotonic() + 150
            while subprocess.run(["systemctl", "is-active", "--quiet", "multi-user.target"], timeout=5).returncode:
                check(time.monotonic() < deadline, "BOOT_NOT_READY"); time.sleep(1)
            baseline = system_state()
            try:
                with tempfile.TemporaryDirectory(prefix="meshpilot-", dir="/run") as home:
                    run_checks(emit, home)
            finally:
                check(system_state() == baseline, "SYSTEM_CHANGED")
                emit("UNCHANGED")
        except Failure as err:
            emit("FAIL " + str(err)); return 1
        except (OSError, KeyError, ValueError, subprocess.SubprocessError) as err:
            emit("FAIL " + type(err).__name__.upper()); return 1
        emit("PASS"); return 0


if __name__ == "__main__":
    raise SystemExit(main())
