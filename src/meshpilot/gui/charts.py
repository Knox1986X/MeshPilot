"""Графики MeshPilot (QPainter): телеметрия, донат диска, ядра CPU."""
from __future__ import annotations
import math
from collections import deque

from PySide6.QtCore import Qt, QPointF
from PySide6.QtGui import QPainter, QColor, QPen, QFont
from PySide6.QtWidgets import QWidget

# Цвета как на фото (MeshPrime)
CPU_COLOR = QColor("#8b7cf6")   # фиолетовый
RAM_COLOR = QColor("#34d399")   # зелёный
GRID_COLOR = QColor("#1e293b")
TEXT_COLOR = QColor("#64748b")
DONUT_COLORS = ["#8b7cf6", "#34d399", "#f59e0b", "#38bdf8", "#f472b6", "#a78bfa", "#22c55e", "#fb923c"]

class TelemetryChart(QWidget):
    """Линейный график CPU/RAM за 60 сек (скользящее окно)."""

    def __init__(self, parent=None) -> None:
        super().__init__(parent)
        self.setMinimumHeight(180)
        self.cpu_history: deque[float] = deque(maxlen=60)
        self.ram_history: deque[float] = deque(maxlen=60)
        for _ in range(60):
            self.cpu_history.append(0.0)
            self.ram_history.append(0.0)

    def push(self, cpu: float, ram: float) -> None:
        self.cpu_history.append(cpu)
        self.ram_history.append(ram)
        self.update()

    def _draw_series(self, painter: QPainter, data: deque[float], color: QColor, w: int, h: int, top: int, left: int) -> None:
        if len(data) < 2:
            return
        painter.setPen(QPen(color, 2))
        points = []
        n = len(data)
        for i, v in enumerate(data):
            x = left + (w * i / max(n - 1, 1))
            y = top + h - (h * max(0.0, min(100.0, v)) / 100.0)
            points.append(QPointF(x, y))
        painter.drawPolyline(points)

    def paintEvent(self, event) -> None:  # noqa: N802
        painter = QPainter(self)
        painter.setRenderHint(QPainter.Antialiasing)
        w, h = self.width(), self.height()
        left, right, top, bottom = 36, 10, 10, 24
        plot_w, plot_h = w - left - right, h - top - bottom
        # сетка + подписи Y
        painter.setPen(QPen(GRID_COLOR, 1))
        painter.setFont(QFont("Inter", 8))
        painter.setPen(TEXT_COLOR)
        for pct in [0, 25, 50, 75, 100]:
            y = top + plot_h - (plot_h * pct / 100.0)
            painter.drawLine(left, int(y), w - right, int(y))
            painter.drawText(4, int(y) + 4, f"{pct}%")
        # подписи X (-55s ... 0s)
        for i, sec in enumerate([55, 45, 35, 25, 15, 5, 0]):
            x = left + (plot_w * i / 6.0)
            painter.drawText(int(x) - 10, h - 6, f"-{sec}s" if sec else "0s")
        self._draw_series(painter, self.cpu_history, CPU_COLOR, plot_w, plot_h, top, left)
        self._draw_series(painter, self.ram_history, RAM_COLOR, plot_w, plot_h, top, left)
        painter.end()

class DonutChart(QWidget):
    """Донат распределения диска."""

    def __init__(self, parent=None) -> None:
        super().__init__(parent)
        self.setMinimumSize(160, 160)
        self.values: list[tuple[str, float]] = []  # (label, percent)

    def set_values(self, values: list[tuple[str, float]]) -> None:
        self.values = values
        self.update()

    def paintEvent(self, event) -> None:  # noqa: N802
        painter = QPainter(self)
        painter.setRenderHint(QPainter.Antialiasing)
        w, h = self.width(), self.height()
        cx, cy = w // 2, h // 2
        radius = min(w, h) // 2 - 8
        thickness = 18
        total = sum(v for _, v in self.values) or 1.0
        start_angle = -90.0
        for i, (_label, val) in enumerate(self.values):
            sweep = 360.0 * (val / total)
            color = QColor(DONUT_COLORS[i % len(DONUT_COLORS)])
            painter.setPen(QPen(color, thickness))
            painter.drawArc(cx - radius, cy - radius, radius * 2, radius * 2,
                            int(start_angle * 16), int(sweep * 16))
            start_angle += sweep
        painter.end()

class CoreBars(QWidget):
    """Бары нагрузки по ядрам CPU."""

    def __init__(self, parent=None) -> None:
        super().__init__(parent)
        self.setMinimumHeight(90)
        self.cores: list[float] = []

    def set_cores(self, cores: list[float]) -> None:
        self.cores = cores
        self.update()

    def paintEvent(self, event) -> None:  # noqa: N802
        painter = QPainter(self)
        painter.setRenderHint(QPainter.Antialiasing)
        w, h = self.width(), self.height()
        n = max(len(self.cores), 1)
        cols = 4 if n > 4 else n
        rows = math.ceil(n / cols)
        cell_w = w // cols
        cell_h = h // rows
        painter.setFont(QFont("Inter", 8))
        for i, val in enumerate(self.cores):
            col, row = i % cols, i // cols
            x, y = col * cell_w + 4, row * cell_h + 4
            cw, ch = cell_w - 8, cell_h - 10
            # фон
            painter.setPen(QPen(QColor("#1e293b"), 1))
            painter.setBrush(QColor("#0b1220"))
            painter.drawRoundedRect(x, y, cw, ch, 6, 6)
            # заливка
            fill_w = int(cw * max(0.0, min(100.0, val)) / 100.0)
            painter.setPen(Qt.NoPen)
            painter.setBrush(QColor("#8b7cf6"))
            if fill_w > 0:
                painter.drawRoundedRect(x + 1, y + 1, fill_w, ch - 2, 5, 5)
            # текст
            painter.setPen(QColor("#e2e8f0"))
            painter.drawText(x + 8, y + ch // 2 + 4, f"CPU{i}")
            painter.drawText(x + cw - 36, y + ch // 2 + 4, f"{val:.0f}%")
        painter.end()
