"""Нижняя панель MeshPilot: виджет поверх системной панели (громкость, статус, быстрые кнопки)."""
from __future__ import annotations
from PySide6.QtCore import Qt
from PySide6.QtWidgets import QWidget, QHBoxLayout, QPushButton, QLabel

class BottomPanel(QWidget):
    def __init__(self) -> None:
        super().__init__()
        self.setWindowFlags(Qt.WindowStaysOnTopHint | Qt.FramelessWindowHint | Qt.Tool)
        self.setFixedHeight(56)
        layout = QHBoxLayout(self)
        layout.setContentsMargins(12, 4, 12, 4)
        self.status_label = QLabel("MeshPilot: готов")
        layout.addWidget(self.status_label)
        layout.addStretch()
        for text in ["🎤", "🧹", "🛡️", "⚙️"]:
            btn = QPushButton(text)
            btn.setFixedSize(40, 40)
            layout.addWidget(btn)
        self.setStyleSheet("background: rgba(20,20,30,230); border-radius: 12px; color: white;")

    def set_status(self, text: str) -> None:
        self.status_label.setText(text)

    def dock_bottom(self) -> None:
        from PySide6.QtWidgets import QApplication
        screen = QApplication.primaryScreen().geometry()
        self.setFixedWidth(screen.width() - 24)
        self.move(12, screen.height() - 68)
