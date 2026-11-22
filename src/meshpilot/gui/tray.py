"""Системный трей MeshPilot."""
from __future__ import annotations
from PySide6.QtWidgets import QSystemTrayIcon, QMenu, QApplication
from PySide6.QtGui import QIcon

def create_tray(app: QApplication, on_show=None, on_quit=None) -> QSystemTrayIcon:
    tray = QSystemTrayIcon()
    tray.setToolTip("MeshPilot")
    # иконка из PNG fallback (если есть), иначе пустая — без варнинга
    from pathlib import Path
    icon_path = Path("assets/models/meshpilot.png")
    if icon_path.exists():
        tray.setIcon(QIcon(str(icon_path)))
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
