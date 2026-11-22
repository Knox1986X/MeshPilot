"""Dashboard MeshPilot (как MeshPrime на фото): живые метрики CachyOS."""
from __future__ import annotations
import platform
import socket
import time
from pathlib import Path

from PySide6.QtCore import Qt, QTimer
from PySide6.QtWidgets import (
    QWidget, QVBoxLayout, QHBoxLayout, QFrame, QLabel, QPushButton, QProgressBar,
)

from meshpilot.gui.charts import TelemetryChart, DonutChart, CoreBars

def _card(title: str, subtitle: str, value: str, extra: str = "", pct: float = 0.0, color: str = "#8b7cf6") -> QFrame:
    card = QFrame()
    card.setObjectName("card")
    lay = QVBoxLayout(card)
    lay.setContentsMargins(14, 10, 14, 10)
    lay.setSpacing(4)
    top = QHBoxLayout()
    t = QLabel(title)
    t.setObjectName("subtitle")
    top.addWidget(t)
    top.addStretch()
    if extra:
        chip = QLabel(extra)
        chip.setObjectName("chip")
        top.addWidget(chip)
    lay.addLayout(top)
    sub = QLabel(subtitle)
    sub.setObjectName("subtitle")
    lay.addWidget(sub)
    v = QLabel(value)
    v.setObjectName("stat_value")
    lay.addWidget(v)
    bar = QProgressBar()
    bar.setRange(0, 100)
    bar.setValue(int(pct))
    bar.setFixedHeight(6)
    bar.setTextVisible(False)
    lay.addWidget(bar)
    card._bar = bar  # type: ignore[attr-defined]
    card._value = v  # type: ignore[attr-defined]
    return card

class DashboardPage(QWidget):
    def __init__(self, parent=None) -> None:
        super().__init__(parent)
        self._boot_time = time.time()
        self._build()
        self._timer = QTimer(self)
        self._timer.timeout.connect(self._refresh)
        self._timer.start(1000)
        self._refresh()

    def _build(self) -> None:
        layout = QVBoxLayout(self)
        layout.setContentsMargins(20, 14, 20, 14)
        layout.setSpacing(12)

        # --- шапка ---
        header = QHBoxLayout()
        title = QLabel("Dashboard")
        title.setObjectName("title")
        header.addWidget(title)
        header.addStretch()
        self.pill_online = QLabel("●  CachyOS online")
        self.pill_online.setObjectName("pill_green")
        header.addWidget(self.pill_online)
        self.pill_kernel = QLabel("Kernel: —")
        self.pill_kernel.setObjectName("pill")
        header.addWidget(self.pill_kernel)
        self.pill_relay = QLabel("relay · — ms")
        self.pill_relay.setObjectName("pill_purple")
        header.addWidget(self.pill_relay)
        layout.addLayout(header)
        sub = QLabel("Живые метрики CachyOS в реальном времени (окно 60 сек)")
        sub.setObjectName("subtitle")
        layout.addWidget(sub)

        # --- статус-пилюли + кнопка ---
        pills = QHBoxLayout()
        self.pill_os = QLabel("●  CachyOS online")
        self.pill_os.setObjectName("pill_green")
        pills.addWidget(self.pill_os)
        self.pill_snap = QLabel("●  Snapshot: btrfs ready")
        self.pill_snap.setObjectName("pill_green")
        pills.addWidget(self.pill_snap)
        pills.addStretch()
        quick = QPushButton("✨  Быстрая очистка")
        quick.setObjectName("primary")
        pills.addWidget(quick)
        layout.addLayout(pills)

        # --- 4 карточки ---
        cards = QHBoxLayout()
        self.card_cpu = _card("CPU Load", "6 потоков", "—", "45°C", 0)
        self.card_ram = _card("RAM Memory", "Swap —", "—", "", 0)
        self.card_disk = _card("Disk Storage", "10 разделов", "—", "", 0)
        self.card_net = _card("Network I/O", "Realtime 1s", "0 B/s", "Live", 0)
        for c in [self.card_cpu, self.card_ram, self.card_disk, self.card_net]:
            cards.addWidget(c)
        layout.addLayout(cards)

        # --- график + донат ---
        mid = QHBoxLayout()
        chart_card = QFrame()
        chart_card.setObjectName("card")
        chart_lay = QVBoxLayout(chart_card)
        chart_lay.setContentsMargins(14, 10, 14, 10)
        ch_title = QHBoxLayout()
        ct = QLabel("CPU & RAM Telemetry (60s)")
        ct.setStyleSheet("font-weight: 700; font-size: 14px;")
        ch_title.addWidget(ct)
        ch_title.addStretch()
        cpu_chip = QLabel("CPU %")
        cpu_chip.setObjectName("chip_purple")
        ch_title.addWidget(cpu_chip)
        ram_chip = QLabel("RAM %")
        ram_chip.setObjectName("chip_green")
        ch_title.addWidget(ram_chip)
        chart_lay.addLayout(ch_title)
        cs = QLabel("Скользящее окно 60 секунд · шаг 1 сек")
        cs.setObjectName("subtitle")
        chart_lay.addWidget(cs)
        self.chart = TelemetryChart()
        chart_lay.addWidget(self.chart, 1)
        mid.addWidget(chart_card, 2)

        donut_card = QFrame()
        donut_card.setObjectName("card")
        donut_lay = QVBoxLayout(donut_card)
        donut_lay.setContentsMargins(14, 10, 14, 10)
        dt = QHBoxLayout()
        dtitle = QLabel("Распределение диска")
        dtitle.setStyleSheet("font-weight: 700; font-size: 14px;")
        dt.addWidget(dtitle)
        dt.addStretch()
        dt.addWidget(QLabel("10 томов"))
        donut_lay.addLayout(dt)
        ds = QLabel("Занятое место по mountpoints")
        ds.setObjectName("subtitle")
        donut_lay.addWidget(ds)
        self.donut = DonutChart()
        donut_lay.addWidget(self.donut, 1)
        self.donut_legend = QLabel("")
        self.donut_legend.setObjectName("subtitle")
        donut_lay.addWidget(self.donut_legend)
        mid.addWidget(donut_card, 1)
        layout.addLayout(mid, 1)

        # --- ядра + окружение ---
        bottom = QHBoxLayout()
        cores_card = QFrame()
        cores_card.setObjectName("card")
        cores_lay = QVBoxLayout(cores_card)
        cores_lay.setContentsMargins(14, 10, 14, 10)
        ct2 = QLabel("Нагрузка по ядрам CPU")
        ct2.setStyleSheet("font-weight: 700; font-size: 14px;")
        cores_lay.addWidget(ct2)
        cs2 = QLabel("Потоковая загрузка в реальном времени")
        cs2.setObjectName("subtitle")
        cores_lay.addWidget(cs2)
        self.cores = CoreBars()
        cores_lay.addWidget(self.cores, 1)
        bottom.addWidget(cores_card, 2)

        env_card = QFrame()
        env_card.setObjectName("card")
        env_lay = QVBoxLayout(env_card)
        env_lay.setContentsMargins(14, 10, 14, 10)
        et = QHBoxLayout()
        etitle = QLabel("Среда CachyOS")
        etitle.setStyleSheet("font-weight: 700; font-size: 14px;")
        et.addWidget(etitle)
        et.addStretch()
        local = QLabel("Local Only")
        local.setObjectName("chip_yellow")
        et.addWidget(local)
        env_lay.addLayout(et)
        es = QLabel("Параметры узла и бэкапа")
        es.setObjectName("subtitle")
        env_lay.addWidget(es)
        self.env_labels = {}
        for key in ["Hostname", "Uptime", "Snapshot backend", "Dropbox"]:
            row = QHBoxLayout()
            k = QLabel(key)
            k.setObjectName("subtitle")
            row.addWidget(k)
            row.addStretch()
            v = QLabel("—")
            row.addWidget(v)
            env_lay.addLayout(row)
            self.env_labels[key] = v
        bottom.addWidget(env_card, 1)
        layout.addLayout(bottom, 1)

    def _refresh(self) -> None:
        try:
            import psutil
            vm = psutil.virtual_memory()
            disk = psutil.disk_usage("/")
            cpu = psutil.cpu_percent(interval=0.1)
            cores = psutil.cpu_percent(percpu=True, interval=0.1)
            # карточки
            self.card_cpu._value.setText(f"{cpu:.1f}%")  # type: ignore[attr-defined]
            self.card_cpu._bar.setValue(int(cpu))  # type: ignore[attr-defined]
            self.card_ram._value.setText(f"{vm.used/1e9:.1f} GB / {vm.total/1e9:.1f} GB")  # type: ignore[attr-defined]
            self.card_ram._bar.setValue(int(vm.percent))  # type: ignore[attr-defined]
            self.card_disk._value.setText(f"{disk.used/1e9:.1f} GB / {disk.total/1e9:.1f} GB")  # type: ignore[attr-defined]
            self.card_disk._bar.setValue(int(disk.percent))  # type: ignore[attr-defined]
            # сеть
            try:
                io = psutil.net_io_counters()
                self.card_net._value.setText(f"{io.bytes_recv/1e6:.1f} MB RX")  # type: ignore[attr-defined]
            except Exception:
                pass
            # график
            self.chart.push(cpu, vm.percent)
            # ядра
            self.cores.set_cores(list(cores))
            # донат: разделы
            try:
                parts = psutil.disk_partitions()
                vals = []
                for p in parts[:6]:
                    try:
                        u = psutil.disk_usage(p.mountpoint)
                        vals.append((p.mountpoint, u.percent))
                    except Exception:
                        continue
                if vals:
                    self.donut.set_values(vals)
                    self.donut_legend.setText("  •  ".join(f"{m} {v:.0f}%" for m, v in vals[:4]))
            except Exception:
                pass
            # окружение
            self.env_labels["Hostname"].setText(socket.gethostname())
            up = int(time.time() - self._boot_time)
            self.env_labels["Uptime"].setText(f"{up//3600}d {(up%3600)//60}h {up%60}m")
            self.env_labels["Snapshot backend"].setText("btrfs" if Path("/.snapshots").exists() or Path("/snapshots").exists() else "—")
            self.env_labels["Dropbox"].setText("не примонтирован")
            # kernel
            try:
                rel = platform.release()
                self.pill_kernel.setText(f"Kernel: {rel}")
            except Exception:
                pass
        except Exception:
            pass
