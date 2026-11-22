"""CLI MeshPilot: meshpilot chat|voice|optimize|mirrors|dns|zapret|health|gui"""
from __future__ import annotations
import asyncio

import typer
from rich.console import Console
from rich.panel import Panel

from meshpilot.logger import setup_logger

app = typer.Typer(name="meshpilot", help="MeshPilot — голосовой ассистент + оптимизатор CachyOS")
console = Console()

@app.command()
def chat(text: str = typer.Argument("", help="Текст запроса (пусто = интерактивно)")):
    """Текстовый чат с агентом (Groq/GigaChat/Hermes)."""
    setup_logger()
    from meshpilot.core.agent import build_default_agent
    agent = build_default_agent()
    if text:
        console.print(Panel(asyncio.run(agent.run(text)), title="MeshPilot"))
    else:
        console.print("MeshPilot чат. /exit для выхода.")
        while True:
            try:
                q = input("Вы: ").strip()
            except (EOFError, KeyboardInterrupt):
                break
            if q in {"/exit", "exit", "quit"}:
                break
            if not q:
                continue
            console.print(Panel(asyncio.run(agent.run(q)), title="MeshPilot"))

@app.command()
def voice(voice_name: str = typer.Option("jarvis", "--voice", "-v", help="jarvis|khan|anime"),
          seconds: float = typer.Option(5.0, "--seconds")):
    """Голосовой режим: запись -> STT -> ответ -> TTS."""
    setup_logger()
    from meshpilot.core.assistant import Assistant
    assistant = Assistant(voice=voice_name)
    console.print(f"Голосовой режим ({voice_name}). Записываю {seconds} сек...")
    reply = asyncio.run(assistant.run_once(seconds=seconds))
    console.print(Panel(reply, title=f"MeshPilot ({voice_name})"))

@app.command()
def optimize(dry_run: bool = typer.Option(True, "--dry-run/--no-dry-run", help="Без удаления")):
    """Полная чистка CachyOS."""
    setup_logger()
    from meshpilot.optimizer import cleaner
    results = cleaner.clean_all(dry_run=dry_run)
    for r in results:
        console.print(f"[bold]{r.name}[/bold]: {r.details}")

@app.command()
def mirrors(dry_run: bool = typer.Option(True, "--dry-run/--no-dry-run")):
    """Подбор лучших зеркал pacman."""
    setup_logger()
    from meshpilot.optimizer import mirrors as m
    console.print(m.auto_mirrors(dry_run=dry_run))

@app.command()
def dns_cmd(dry_run: bool = typer.Option(True, "--dry-run/--no-dry-run")):
    """Бенчмарк и установка DNS."""
    setup_logger()
    from meshpilot.optimizer import dns
    console.print(dns.auto_dns(dry_run=dry_run))

@app.command()
def zapret(action: str = typer.Argument("status", help="status|install|start|stop|restart|auto|logs"),
           dry_run: bool = typer.Option(True, "--dry-run/--no-dry-run")):
    """Управление Zapret2."""
    setup_logger()
    from meshpilot.optimizer import zapret as z
    if action == "status":
        console.print(str(z.get_status()))
    elif action == "install":
        console.print(z.install(dry_run=dry_run))
    elif action == "start":
        console.print(z.start())
    elif action == "stop":
        console.print(z.stop())
    elif action == "restart":
        console.print(z.restart())
    elif action == "auto":
        console.print(z.auto_strategy(dry_run=dry_run))
    elif action == "logs":
        console.print(z.logs())
    else:
        console.print(f"Неизвестное действие: {action}")

@app.command()
def health():
    """Health-check системы."""
    setup_logger()
    from meshpilot.optimizer import health as h
    console.print(h.health_report())

@app.command()
def gui():
    """Запуск GUI (PySide6): главное окно + персонаж + панель."""
    setup_logger()
    import sys
    from PySide6.QtWidgets import QApplication
    from meshpilot.gui.main_window import MainWindow
    from meshpilot.gui.panel import BottomPanel
    from meshpilot.gui.tray import create_tray
    from meshpilot.avatar.overlay import OverlayWidget

    app_qt = QApplication(sys.argv)
    app_qt.setApplicationName("MeshPilot")
    from meshpilot.gui.theme import apply_theme
    apply_theme(app_qt)
    # модель персонажа из конфига (Live2D/VRM/PNG)
    from meshpilot.config import config as _cfg
    model_path = _cfg.get("avatar", "model", default="assets/models/meshpilot.model3.json")
    window = MainWindow()
    window.show()
    panel = BottomPanel()
    panel.show()
    panel.dock_bottom()
    overlay = OverlayWidget(model_path=model_path)
    overlay.show()
    dock = _cfg.get("avatar", "dock", default="bottom-right")
    overlay.dock_to(dock)
    tray = create_tray(app_qt, on_show=window.show)
    sys.exit(app_qt.exec())

@app.command()
def version():
    from meshpilot import __version__
    console.print(f"MeshPilot {__version__}")

if __name__ == "__main__":
    app()
