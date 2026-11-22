"""Системный трей MeshPilot."""
from __future__ import annotations
from PySide6.QtWidgets import QSystemTrayIcon, QMenu, QApplication
from PySide6.QtGui import QIcon

def create_tray(app: QApplication, on_show=None, on_quit=None) -> QSystemTrayIcon:
    tray = QSystemTrayIcon()
    tray.setToolTip("MeshPilot")
    menu = QMenu()
    show_action = menu.addAction("Показать")
    quit_action = menu.addAction("Выход")
    if on_show:
        show_action.triggered.connect(on_show)
    if on_quit:
        quit_action.triggered.connect(on_quit)
    else:
        quit_action.triggered.connect(app.quit)
    tray.setContextMenu(menu)
    tray.show()
    return tray
