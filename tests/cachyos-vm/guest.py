"""Работает только на Live ISO в выделенной VM, запущенной run.sh. Не устанавливает пакеты."""
import hashlib
import json
import os
import platform
import pwd
import re
import stat
import subprocess
import tempfile
import time
from pathlib import Path

from checks import (Failure, check, proc_cpu_ticks, verify_cli_cpu, verify_cpu_card,
                    verify_cli_ram, verify_ram_card, verify_cli_root_fs, verify_root_fs_card,
                    verify_cli_uptime, verify_uptime_card, verify_cli_kernel, verify_kernel_card,
                    verify_cli_os_version, verify_os_version_card)

from net_checks import net_ticks, verify_cli_net, verify_loopback_card, verify_shared_window

MEDIA = Path("/run/meshpilot-smoke")


def command(args, **kwargs):
    return subprocess.run(args, capture_output=True, text=True, timeout=kwargs.pop("timeout", 12),
                          check=True, **kwargs).stdout


def readonly_cd(path):
    info = command(["findmnt", "-n", "-o", "FSTYPE,OPTIONS", "--mountpoint", path]).split()
    return len(info) == 2 and info[0] == "iso9660" and "ro" in info[1].split(",")


def guard():
    check(os.geteuid() == 0 and Path("/sys/class/dmi/id/product_name").read_text().strip() == "MeshPilot-smoke", "VM_GUARD")
    check(command(["systemd-detect-virt", "--vm"]).strip() in {"kvm", "qemu"}, "VM_GUARD")


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


def display_ready(user, env, account):
    # Не считать наличия переменной DISPLAY доказательством работы графического сеанса.
    deadline = time.monotonic() + 90
    while time.monotonic() < deadline:
        try:
            lines = user(["systemctl", "--user", "show-environment"], timeout=3).splitlines()
            values = dict(line.split("=", 1) for line in lines if "=" in line)
            if re.fullmatch(r":\d+(?:\.\d+)?", values.get("DISPLAY", "")):
                env["DISPLAY"] = values["DISPLAY"]
                env["XAUTHORITY"] = values.get("XAUTHORITY", account.pw_dir + "/.Xauthority")
                geometry = user([MEDIA / "window-observer", "getdisplaygeometry"], timeout=3)
                if re.fullmatch(r"[1-9]\d* [1-9]\d*", geometry.strip()):
                    return True
        except subprocess.SubprocessError:
            pass
        time.sleep(1)
    return False


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

    def observed(args, timeout):
        # Не терять код выхода и диагностику самого испытуемого процесса. Произвольный stdout окна не публикуем.
        record = json.loads(user(["/usr/bin/python3", "-I", "-B", MEDIA / "fs_witness.py", *args], timeout=timeout))
        result = subprocess.CompletedProcess(args, record["returncode"], record["out"], record["err"])
        return (result, record["low"], record["high"],
                record["up_lo"], record["up_hi"], record["kr_lo"], record["kr_hi"],
                record["ov_lo"], record["ov_hi"])

    # Только неизменившиеся данные реального образа. Не подменять os-release в тесте.
    raw_id = platform.freedesktop_os_release()["ID"]
    net_before = net_ticks()
    before = proc_cpu_ticks()
    mono = time.monotonic_ns()
    started = time.time_ns()
    cli, fs_low, fs_high, up_lo, up_hi, kr_lo, kr_hi, ov_lo, ov_hi = observed([MEDIA / "meshpilot", "status", "--json"], timeout=12)
    cli.check_returncode()
    status = json.loads(cli.stdout)
    elapsed_ms = (time.monotonic_ns() - mono) // 1_000_000
    after = proc_cpu_ticks()
    verify_cli_cpu(status.get("cpu"), started, elapsed_ms, before, after)
    verify_cli_net(status.get("network_io"), started, elapsed_ms, net_before, net_ticks())
    verify_shared_window(status["cpu"], status["network_io"])
    check(status["read_only"] is True and status["os_id"] == raw_id and status["cachyos"] is True, "CLI_PLATFORM")
    check(all(status[k] is None for k in ("online", "dns", "updatable_apps", "security_fixes", "last_success_utc")), "CLI_OBSERVATIONS")
    verify_cli_ram(status.get("ram"), started)
    verify_cli_root_fs(status.get("root_fs"), started, fs_low, fs_high)
    verify_cli_uptime(status.get("uptime"), started, up_lo, up_hi)
    verify_cli_kernel(status.get("kernel"), started, kr_lo, kr_hi)
    verify_cli_os_version(status.get("os_version"), started, ov_lo, ov_hi)
    doctor = json.loads(user([MEDIA / "meshpilot", "doctor", "--json", "--dry-run"]))
    checks = {c["id"]: c["finding"] for c in doctor["checks"]}
    check(doctor["read_only"] is True and checks["os"] == checks["pacman"] == "detected", "CLI_DOCTOR")
    for component, soname in [("webkitgtk", "libwebkit2gtk-4.1.so.0"), ("javascriptcoregtk", "libjavascriptcoregtk-4.1.so.0")]:
        row = next((c for c in doctor["checks"] if c["id"] == f"desktop-{component}"), None)
        expected = "detected" if (Path("/usr/lib") / soname).is_file() else "missing"
        check(row is not None and row["required"] is False and row["finding"] == expected, "CLI_GUI_DIAGNOSTIC")
    check(not any(Path(home).iterdir()), "CLI_XDG_WRITE")
    emit("CLI")

    linked = user(["ldd", MEDIA / "meshpilot-desktop"])
    missing = re.findall(r"^\s*(lib[\w.+-]+) => not found$", linked, re.MULTILINE)
    session = display_ready(user, env, account)
    if session:
        emit("SESSION")
    if missing:
        raise Failure("GUI_RUNTIME " + " ".join(sorted(set(missing))[:20]))
    # Не засчитывать библиотеку со второго CD или из нестандартного каталога.
    for soname in ("libwebkit2gtk-4.1.so.0", "libjavascriptcoregtk-4.1.so.0"):
        match = re.search(r"^\s*" + re.escape(soname) + r" => (/\S+) \(", linked, re.MULTILINE)
        check(match and os.path.samefile(match[1], Path("/usr/lib") / soname), "GUI_RUNTIME_PATH")
    check(session, "NO_GRAPHICAL_SESSION")
    env.update(GDK_BACKEND="x11", WEBKIT_DISABLE_DMABUF_RENDERER="1", LIBGL_ALWAYS_SOFTWARE="1")
    net_before = net_ticks()
    started = int(time.time())
    mono = time.monotonic_ns()
    observer_error = None
    try:
        window, wfs_low, wfs_high, wup_lo, wup_hi, wkr_lo, wkr_hi, wov_lo, wov_hi = observed([MEDIA / "native-window.sh", MEDIA / "meshpilot-desktop", MEDIA / "window-observer", MEDIA / "metric-name"], timeout=270)
        window.check_returncode()
        output = window.stdout
    except subprocess.CalledProcessError as err:
        if err.returncode not in (42, 43, 44, 45, 46, 47, 48):
            raise Failure("GUI_WINDOW") from err
        codes = {"GUI_METRIC_" + suffix for suffix in ("BUS", "APP", "TREE", "EMPTY", "IPC", "INVALID", "UNAVAILABLE", "PENDING", "PARTIAL")}
        reason = next((line for line in err.stderr.splitlines() if line in codes), "GUI_METRIC_OBSERVER")
        observer_error = ({42: "RAM", 43: "CPU", 44: "FS", 45: "NET", 46: "UPTIME", 47: "KERNEL", 48: "OS_VERSION"}[err.returncode], reason)
        output = err.stdout
    except subprocess.SubprocessError as err:
        raise Failure("GUI_WINDOW") from err
    emit("WINDOW")

    def label(metric, heading):
        if observer_error and observer_error[0] == metric:
            raise Failure(observer_error[1].replace("METRIC", metric))
        lines = [line for line in output.splitlines() if line.startswith(heading)]
        check(len(lines) == 1, f"GUI_{metric}_LABEL")
        return lines[0]

    verify_ram_card(label("RAM", "Занято RAM"), started)
    emit("RAM_CARD")
    verify_cpu_card(label("CPU", "Загрузка CPU"), started, (time.monotonic_ns() - mono) // 1_000_000)
    emit("CPU_CARD")
    verify_root_fs_card(label("FS", "Занято на /"), started, wfs_low, wfs_high)
    emit("ROOT_FS_CARD")
    verify_loopback_card(label("NET", "Трафик сети"), started, (time.monotonic_ns() - mono) // 1_000_000, net_before, net_ticks())
    emit("NET_IO_CARD")
    verify_uptime_card(label("UPTIME", "Время работы"), started, wup_lo, wup_hi)
    emit("UPTIME_CARD")
    verify_kernel_card(label("KERNEL", "Ядро"), started, wkr_lo, wkr_hi)
    emit("KERNEL_CARD")
    verify_os_version_card(label("OS_VERSION", "Версия системы"), started, wov_lo, wov_hi)
    emit("OS_VERSION_CARD")


def main():
    # Никаких эффектов на хосте при ошибочном ручном запуске.
    guard()
    run_id = (MEDIA / "run-id").read_text().strip()
    check(re.fullmatch(r"[0-9a-f]{32}", run_id), "RUN_ID")
    with open("/dev/ttyS1", "w", buffering=1, encoding="ascii") as serial:
        def emit(event):
            print(f"MESHPILOT/1 {run_id} {event}", file=serial, flush=True)
        try:
            check(readonly_cd(MEDIA) and readonly_cd("/run/archiso/bootmnt"), "MEDIA_GUARD")
            check("cachyos" in os.uname().release and set(os.listdir("/sys/class/net")) == {"lo"}, "GUEST_ENV")
            emit("GUEST")
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
