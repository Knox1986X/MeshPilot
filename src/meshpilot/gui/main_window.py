"""Главное окно MeshPilot (PySide6): чат, оптимизатор, настройки, система."""
from __future__ import annotations
import asyncio

from PySide6.QtCore import Qt, QThread, Signal
from PySide6.QtWidgets import (
    QMainWindow, QTabWidget, QWidget, QVBoxLayout, QHBoxLayout,
    QTextEdit, QLineEdit, QPushButton, QLabel, QComboBox, QCheckBox,
    QProgressBar, QListWidget, QSplitter,
)

from meshpilot.config import config

class AsyncWorker(QThread):
    finished = Signal(str)
    def __init__(self, coro_fn, *args, **kwargs):
        super().__init__()
        self._coro_fn = coro_fn
        self._args = args
        self._kwargs = kwargs
    def run(self):
        try:
            result = asyncio.run(self._coro_fn(*self._args, **self._kwargs))
            self.finished.emit(str(result))
        except Exception as e:
            self.finished.emit(f"Ошибка: {e}")

class MainWindow(QMainWindow):
    def __init__(self) -> None:
        super().__init__()
        self.setWindowTitle("MeshPilot — голосовой ассистент + оптимизатор CachyOS")
        self.resize(960, 640)
        self._build_ui()

    def _build_ui(self) -> None:
        tabs = QTabWidget()
        tabs.addTab(self._chat_tab(), "💬 Чат / Голос")
        tabs.addTab(self._optimizer_tab(), "🧹 Оптимизатор")
        tabs.addTab(self._zapret_tab(), "🛡️ Zapret2 / DNS")
        tabs.addTab(self._settings_tab(), "⚙️ Настройки")
        self.setCentralWidget(tabs)

    def _chat_tab(self) -> QWidget:
        w = QWidget()
        layout = QVBoxLayout(w)
        self.chat_log = QTextEdit()
        self.chat_log.setReadOnly(True)
        layout.addWidget(self.chat_log, 1)
        row = QHBoxLayout()
        self.chat_input = QLineEdit()
        self.chat_input.setPlaceholderText("Спроси MeshPilot... (например: почисти кэш и поставь лучшие зеркала)")
        self.chat_input.returnPressed.connect(self._send_chat)
        send_btn = QPushButton("Отправить")
        send_btn.clicked.connect(self._send_chat)
        mic_btn = QPushButton("🎤 Голос")
        mic_btn.clicked.connect(self._voice_input)
        row.addWidget(self.chat_input, 1)
        row.addWidget(send_btn)
        row.addWidget(mic_btn)
        layout.addLayout(row)
        self.voice_label = QLabel("Голос: Джарвис (jarvis) — смени в Настройках")
        layout.addWidget(self.voice_label)
        return w

    def _optimizer_tab(self) -> QWidget:
        w = QWidget()
        layout = QVBoxLayout(w)
        layout.addWidget(QLabel("Оптимизатор CachyOS — чистка, зеркала, система"))
        self.opt_log = QTextEdit()
        self.opt_log.setReadOnly(True)
        layout.addWidget(self.opt_log, 1)
        self.opt_progress = QProgressBar()
        self.opt_progress.setRange(0, 0)  # indeterminate
        self.opt_progress.hide()
        layout.addWidget(self.opt_progress)
        row = QHBoxLayout()
        self.dry_run_check = QCheckBox("Dry-run (без удаления)")
        self.dry_run_check.setChecked(True)
        row.addWidget(self.dry_run_check)
        for label, fn in [
            ("🧹 Полная чистка", self._opt_clean_all),
            ("📦 Кэш pacman", self._opt_cache),
            ("🗑️ Orphan-пакеты", self._opt_orphans),
            ("🪞 Зеркала", self._opt_mirrors),
            ("❤️ Health-check", self._opt_health),
        ]:
            btn = QPushButton(label)
            btn.clicked.connect(fn)
            row.addWidget(btn)
        layout.addLayout(row)
        return w

    def _zapret_tab(self) -> QWidget:
        w = QWidget()
        layout = QVBoxLayout(w)
        layout.addWidget(QLabel("Zapret2 — DPI-обход + DNS"))
        self.zapret_log = QTextEdit()
        self.zapret_log.setReadOnly(True)
        layout.addWidget(self.zapret_log, 1)
        row = QHBoxLayout()
        for label, fn in [
            ("📊 Статус", self._zapret_status),
            ("⬇️ Установить Zapret2", self._zapret_install),
            ("🎯 Автоподбор стратегии", self._zapret_auto),
            ("▶️ Перезапуск", self._zapret_restart),
            ("🌐 DNS бенчмарк", self._dns_bench),
        ]:
            btn = QPushButton(label)
            btn.clicked.connect(fn)
            row.addWidget(btn)
        layout.addLayout(row)
        self.zapret_dry = QCheckBox("Dry-run")
        self.zapret_dry.setChecked(True)
        layout.addWidget(self.zapret_dry)
        return w

    def _settings_tab(self) -> QWidget:
        w = QWidget()
        layout = QVBoxLayout(w)
        layout.addWidget(QLabel("Настройки MeshPilot"))
        row = QHBoxLayout()
        row.addWidget(QLabel("Голос:"))
        self.voice_combo = QComboBox()
        self.voice_combo.addItems(["jarvis — Джарвис", "khan — Хан", "anime — Мико (аниме)"])
        self.voice_combo.setCurrentIndex(0)
        row.addWidget(self.voice_combo)
        layout.addLayout(row)
        row2 = QHBoxLayout()
        row2.addWidget(QLabel("LLM провайдер (каскад):"))
        self.llm_label = QLabel("Groq → GigaChat → Hermes (Ollama) → OpenAI-compat")
        row2.addWidget(self.llm_label)
        layout.addLayout(row2)
        layout.addWidget(QLabel("Ключи API задаются в .env: GROQ_API_KEY, GIGACHAT_CLIENT_ID/SECRET"))
        layout.addWidget(QLabel("Hermes локально: ollama pull nous-hermes3:8b && ollama serve"))
        layout.addStretch()
        return w

    # --- handlers ---
    def _send_chat(self) -> None:
        text = self.chat_input.text().strip()
        if not text:
            return
        self.chat_input.clear()
        self.chat_log.append(f"Вы: {text}")
        from meshpilot.core.agent import build_default_agent
        agent = build_default_agent()
        worker = AsyncWorker(agent.run, text)
        worker.finished.connect(lambda r: self.chat_log.append(f"MeshPilot: {r}"))
        worker.start()

    def _voice_input(self) -> None:
        self.chat_log.append("🎤 Слушаю... (запись 5 сек)")
        from meshpilot.voice.stt import STT, record_wav
        import tempfile
        tmp = tempfile.mktemp(suffix=".wav")
        try:
            record_wav(tmp, seconds=5.0)
        except Exception as e:
            self.chat_log.append(f"Микрофон недоступен: {e}")
            return
        stt = STT()
        worker = AsyncWorker(stt.transcribe_file, tmp)
        worker.finished.connect(lambda r: (self.chat_log.append(f"🎤 Распознано: {r}"), self.chat_input.setText(r)))
        worker.start()

    def _opt_clean_all(self) -> None:
        from meshpilot.optimizer import cleaner
        dry = self.dry_run_check.isChecked()
        self.opt_progress.show()
        worker = AsyncWorker(lambda: "\n".join(f"{r.name}: {r.details}" for r in cleaner.clean_all(dry_run=dry)))
        worker.finished.connect(lambda r: (self.opt_log.append(r), self.opt_progress.hide()))
        worker.start()

    def _opt_cache(self) -> None:
        from meshpilot.optimizer import cleaner
        dry = self.dry_run_check.isChecked()
        worker = AsyncWorker(lambda: str(cleaner.clean_pacman_cache(dry_run=dry).details))
        worker.finished.connect(self.opt_log.append)
        worker.start()

    def _opt_orphans(self) -> None:
        from meshpilot.optimizer import cleaner
        dry = self.dry_run_check.isChecked()
        worker = AsyncWorker(lambda: str(cleaner.remove_orphans(dry_run=dry).details))
        worker.finished.connect(self.opt_log.append)
        worker.start()

    def _opt_mirrors(self) -> None:
        from meshpilot.optimizer import mirrors
        dry = self.dry_run_check.isChecked()
        worker = AsyncWorker(mirrors.auto_mirrors, dry_run=dry)
        worker.finished.connect(self.opt_log.append)
        worker.start()

    def _opt_health(self) -> None:
        from meshpilot.optimizer import health
        worker = AsyncWorker(health.health_report)
        worker.finished.connect(self.opt_log.append)
        worker.start()

    def _zapret_status(self) -> None:
        from meshpilot.optimizer import zapret
        worker = AsyncWorker(lambda: str(zapret.get_status()))
        worker.finished.connect(self.zapret_log.append)
        worker.start()

    def _zapret_install(self) -> None:
        from meshpilot.optimizer import zapret
        dry = self.zapret_dry.isChecked()
        worker = AsyncWorker(zapret.install, dry_run=dry)
        worker.finished.connect(self.zapret_log.append)
        worker.start()

    def _zapret_auto(self) -> None:
        from meshpilot.optimizer import zapret
        dry = self.zapret_dry.isChecked()
        worker = AsyncWorker(zapret.auto_strategy, dry_run=dry)
        worker.finished.connect(self.zapret_log.append)
        worker.start()

    def _zapret_restart(self) -> None:
        from meshpilot.optimizer import zapret
        worker = AsyncWorker(zapret.restart)
        worker.finished.connect(self.zapret_log.append)
        worker.start()

    def _dns_bench(self) -> None:
        from meshpilot.optimizer import dns
        dry = self.zapret_dry.isChecked()
        worker = AsyncWorker(dns.auto_dns, dry_run=dry)
        worker.finished.connect(self.zapret_log.append)
        worker.start()
