"""Главное окно MeshPilot (PySide6): топовый тёмный интерфейс.

Дизайн: сайдбар-навигация, карточки, пузыри чата, дашборд оптимизатора.
"""
from __future__ import annotations
import asyncio

from PySide6.QtCore import Qt, QThread, Signal, QTimer
from PySide6.QtWidgets import (
    QMainWindow, QWidget, QVBoxLayout, QHBoxLayout, QFrame,
    QTextEdit, QLineEdit, QPushButton, QLabel, QComboBox, QCheckBox,
    QProgressBar, QStackedWidget, QScrollArea, QSizePolicy,
)

from meshpilot.config import config
from meshpilot.gui.theme import apply_theme

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
    def cleanup(self):
        self.deleteLater()

class MainWindow(QMainWindow):
    def __init__(self) -> None:
        super().__init__()
        self.setWindowTitle("MeshPilot — голосовой ассистент + оптимизатор CachyOS")
        self.resize(1100, 720)
        self.setMinimumSize(900, 600)
        self._workers: list[AsyncWorker] = []
        self._build_ui()
        # авто-обновление статистики
        self._stat_timer = QTimer(self)
        self._stat_timer.timeout.connect(self._refresh_stats)
        self._stat_timer.start(5000)
        self._refresh_stats()

    def _start_worker(self, coro_fn, *args, **kwargs) -> AsyncWorker:
        worker = AsyncWorker(coro_fn, *args, **kwargs)
        self._workers.append(worker)
        worker.finished.connect(worker.cleanup)
        worker.start()
        return worker

    def closeEvent(self, event) -> None:  # noqa: N802
        for w in list(self._workers):
            if w.isRunning():
                w.quit()
                w.wait(2000)
        super().closeEvent(event)

    def _build_ui(self) -> None:
        root = QWidget()
        root_layout = QHBoxLayout(root)
        root_layout.setContentsMargins(0, 0, 0, 0)
        root_layout.setSpacing(0)

        # --- сайдбар ---
        sidebar = QFrame()
        sidebar.setObjectName("sidebar")
        sidebar.setFixedWidth(210)
        sb_layout = QVBoxLayout(sidebar)
        sb_layout.setContentsMargins(10, 16, 10, 16)
        sb_layout.setSpacing(4)

        logo = QLabel("◈ MeshPilot")
        logo.setObjectName("title")
        sb_layout.addWidget(logo)
        sub = QLabel("voice + optimizer")
        sub.setObjectName("subtitle")
        sb_layout.addWidget(sub)
        sb_layout.addSpacing(16)

        self.nav_buttons = []
        self.stack = QStackedWidget()
        pages = [
            ("💬  Чат / Голос", self._chat_page()),
            ("🧹  Оптимизатор", self._optimizer_page()),
            ("🛡️  Zapret2 / DNS", self._zapret_page()),
            ("⚙️  Настройки", self._settings_page()),
        ]
        for i, (label, page) in enumerate(pages):
            btn = QPushButton(label)
            btn.setObjectName("nav")
            btn.setCheckable(True)
            btn.setChecked(i == 0)
            btn.clicked.connect(lambda _checked, idx=i: self._switch_page(idx))
            sb_layout.addWidget(btn)
            self.nav_buttons.append(btn)
            self.stack.addWidget(page)

        sb_layout.addStretch()
        # статус внизу сайдбара
        self.status_dot = QLabel("●  готов")
        self.status_dot.setObjectName("subtitle")
        sb_layout.addWidget(self.status_dot)
        self.voice_chip = QLabel("Голос: Джарвис")
        self.voice_chip.setObjectName("subtitle")
        sb_layout.addWidget(self.voice_chip)

        root_layout.addWidget(sidebar)
        root_layout.addWidget(self.stack, 1)
        self.setCentralWidget(root)

    def _switch_page(self, idx: int) -> None:
        self.stack.setCurrentIndex(idx)
        for i, b in enumerate(self.nav_buttons):
            b.setChecked(i == idx)

    def _set_status(self, text: str) -> None:
        self.status_dot.setText(f"●  {text}")

    # ---------- Чат ----------
    def _chat_page(self) -> QWidget:
        page = QWidget()
        layout = QVBoxLayout(page)
        layout.setContentsMargins(20, 16, 20, 16)
        layout.setSpacing(12)

        header = QHBoxLayout()
        title = QLabel("Чат с MeshPilot")
        title.setObjectName("title")
        header.addWidget(title)
        header.addStretch()
        self.chat_voice_label = QLabel("Джарвис • Groq → GigaChat → Hermes")
        self.chat_voice_label.setObjectName("subtitle")
        header.addWidget(self.chat_voice_label)
        layout.addLayout(header)

        self.chat_log = QTextEdit()
        self.chat_log.setReadOnly(True)
        self.chat_log.setPlaceholderText("MeshPilot готов. Спроси: «почисти кэш и поставь лучшие зеркала»")
        layout.addWidget(self.chat_log, 1)

        row = QHBoxLayout()
        self.chat_input = QLineEdit()
        self.chat_input.setPlaceholderText("Напиши сообщение... Enter — отправить")
        self.chat_input.returnPressed.connect(self._send_chat)
        send_btn = QPushButton("Отправить")
        send_btn.setObjectName("primary")
        send_btn.clicked.connect(self._send_chat)
        mic_btn = QPushButton("🎤  Голос")
        mic_btn.clicked.connect(self._voice_input)
        row.addWidget(self.chat_input, 1)
        row.addWidget(send_btn)
        row.addWidget(mic_btn)
        layout.addLayout(row)
        return page

    # ---------- Оптимизатор ----------
    def _optimizer_page(self) -> QWidget:
        page = QWidget()
        layout = QVBoxLayout(page)
        layout.setContentsMargins(20, 16, 20, 16)
        layout.setSpacing(12)

        title = QLabel("Оптимизатор CachyOS")
        title.setObjectName("title")
        layout.addWidget(title)
        sub = QLabel("Чистка, зеркала, система — всё в один клик. Dry-run по умолчанию безопасен.")
        sub.setObjectName("subtitle")
        layout.addWidget(sub)

        # карточки статистики
        stats = QHBoxLayout()
        self.stat_disk = self._stat_card("Диск", "—")
        self.stat_ram = self._stat_card("RAM", "—")
        self.stat_cache = self._stat_card("Кэш pacman", "—")
        self.stat_cpu = self._stat_card("CPU", "—")
        for c in [self.stat_disk, self.stat_ram, self.stat_cache, self.stat_cpu]:
            stats.addWidget(c)
        layout.addLayout(stats)

        self.opt_log = QTextEdit()
        self.opt_log.setReadOnly(True)
        self.opt_log.setPlaceholderText("Лог оптимизатора...")
        layout.addWidget(self.opt_log, 1)

        self.opt_progress = QProgressBar()
        self.opt_progress.setRange(0, 0)
        self.opt_progress.hide()
        layout.addWidget(self.opt_progress)

        row = QHBoxLayout()
        self.dry_run_check = QCheckBox("Dry-run (без удаления)")
        self.dry_run_check.setChecked(True)
        row.addWidget(self.dry_run_check)
        row.addStretch()
        layout.addLayout(row)

        grid = QHBoxLayout()
        for label, fn in [
            ("🧹 Полная чистка", self._opt_clean_all),
            ("📦 Кэш pacman", self._opt_cache),
            ("🗑️ Orphan-пакеты", self._opt_orphans),
            ("🪞 Зеркала", self._opt_mirrors),
            ("❤️ Health-check", self._opt_health),
        ]:
            btn = QPushButton(label)
            btn.clicked.connect(fn)
            grid.addWidget(btn)
        layout.addLayout(grid)
        return page

    def _stat_card(self, label: str, value: str) -> QFrame:
        card = QFrame()
        card.setObjectName("card")
        lay = QVBoxLayout(card)
        lay.setContentsMargins(14, 10, 14, 10)
        v = QLabel(value)
        v.setObjectName("stat_value")
        l = QLabel(label)
        l.setObjectName("stat_label")
        lay.addWidget(v)
        lay.addWidget(l)
        card._value_label = v  # type: ignore[attr-defined]
        return card

    def _refresh_stats(self) -> None:
        try:
            import psutil
            from pathlib import Path
            vm = psutil.virtual_memory()
            disk = psutil.disk_usage("/")
            cpu = psutil.cpu_percent(interval=0.1)
            cache = Path("/var/cache/pacman/pkg")
            cache_mb = 0
            if cache.exists():
                try:
                    cache_mb = sum(f.stat().st_size for f in cache.iterdir() if f.is_file()) / 1e6
                except Exception:
                    pass
            self.stat_disk._value_label.setText(f"{disk.percent:.0f}%")  # type: ignore[attr-defined]
            self.stat_ram._value_label.setText(f"{vm.percent:.0f}%")  # type: ignore[attr-defined]
            self.stat_cache._value_label.setText(f"{cache_mb:.0f} МБ")  # type: ignore[attr-defined]
            self.stat_cpu._value_label.setText(f"{cpu:.0f}%")  # type: ignore[attr-defined]
        except Exception:
            pass

    # ---------- Zapret ----------
    def _zapret_page(self) -> QWidget:
        page = QWidget()
        layout = QVBoxLayout(page)
        layout.setContentsMargins(20, 16, 20, 16)
        layout.setSpacing(12)

        title = QLabel("Zapret2 + DNS")
        title.setObjectName("title")
        layout.addWidget(title)
        sub = QLabel("DPI-обход для RU + лучшие DNS. Стратегия подбирается автоматически.")
        sub.setObjectName("subtitle")
        layout.addWidget(sub)

        self.zapret_log = QTextEdit()
        self.zapret_log.setReadOnly(True)
        self.zapret_log.setPlaceholderText("Статус Zapret2, логи, бенчмарк DNS...")
        layout.addWidget(self.zapret_log, 1)

        row = QHBoxLayout()
        for label, fn in [
            ("📊 Статус", self._zapret_status),
            ("⬇️ Установить", self._zapret_install),
            ("🎯 Автоподбор", self._zapret_auto),
            ("▶️ Перезапуск", self._zapret_restart),
            ("🌐 DNS", self._dns_bench),
        ]:
            btn = QPushButton(label)
            btn.clicked.connect(fn)
            row.addWidget(btn)
        layout.addLayout(row)
        self.zapret_dry = QCheckBox("Dry-run")
        self.zapret_dry.setChecked(True)
        layout.addWidget(self.zapret_dry)
        return page

    # ---------- Настройки ----------
    def _settings_page(self) -> QWidget:
        page = QWidget()
        layout = QVBoxLayout(page)
        layout.setContentsMargins(20, 16, 20, 16)
        layout.setSpacing(12)

        title = QLabel("Настройки")
        title.setObjectName("title")
        layout.addWidget(title)

        # голоса — карточки
        voice_title = QLabel("Голос ассистента")
        voice_title.setObjectName("subtitle")
        layout.addWidget(voice_title)
        voices = QHBoxLayout()
        self.voice_buttons = {}
        for key, label, desc in [
            ("jarvis", "🎩 Джарвис", "Тони Старк\nбританский, формальный"),
            ("khan", "🎖️ Хан", "Metro\nглубокий, хриплый"),
            ("anime", "🌸 Мико", "аниме\nэнергичная, милая"),
        ]:
            card = QFrame()
            card.setObjectName("card")
            lay = QVBoxLayout(card)
            lay.setContentsMargins(12, 10, 12, 10)
            btn = QPushButton(label)
            btn.setCheckable(True)
            btn.setChecked(key == "jarvis")
            btn.clicked.connect(lambda _c, k=key: self._select_voice(k))
            lay.addWidget(btn)
            d = QLabel(desc)
            d.setObjectName("subtitle")
            lay.addWidget(d)
            voices.addWidget(card)
            self.voice_buttons[key] = btn
        layout.addLayout(voices)

        layout.addSpacing(8)
        llm_title = QLabel("LLM каскад")
        llm_title.setObjectName("subtitle")
        layout.addWidget(llm_title)
        llm = QLabel("Groq (Llama 3.3 70B)  →  GigaChat (Sber)  →  Hermes локально (Ollama)  →  OpenAI-compat")
        layout.addWidget(llm)
        keys = QLabel("Ключи в .env: GROQ_API_KEY, GIGACHAT_CLIENT_ID/SECRET. Hermes: ollama pull nous-hermes3:8b")
        keys.setObjectName("subtitle")
        layout.addWidget(keys)

        layout.addSpacing(8)
        model_title = QLabel("Модель персонажа (Live2D/VRM)")
        model_title.setObjectName("subtitle")
        layout.addWidget(model_title)
        model_path = config.get("avatar", "model", default="assets/models/meshpilot.model3.json")
        model_label = QLabel(f"Текущая: {model_path}")
        layout.addWidget(model_label)
        hint = QLabel("Скачай Втубер-модель с Booth.pm / VRoid Hub → положи в assets/models/ → укажи путь в config/default.yaml")
        hint.setObjectName("subtitle")
        layout.addWidget(hint)

        layout.addStretch()
        return page

    def _select_voice(self, key: str) -> None:
        for k, b in self.voice_buttons.items():
            b.setChecked(k == key)
        labels = {"jarvis": "Джарвис", "khan": "Хан", "anime": "Мико"}
        self.voice_chip.setText(f"Голос: {labels.get(key, key)}")
        self.chat_voice_label.setText(f"{labels.get(key, key)} • Groq → GigaChat → Hermes")
        self._set_status(f"голос: {labels.get(key, key)}")

    # --- handlers ---
    def _send_chat(self) -> None:
        text = self.chat_input.text().strip()
        if not text:
            return
        self.chat_input.clear()
        self.chat_log.append(f"Вы: {text}")
        self._set_status("думаю...")
        from meshpilot.core.agent import build_default_agent
        agent = build_default_agent()
        worker = self._start_worker(agent.run, text)
        worker.finished.connect(lambda r: (self.chat_log.append(f"MeshPilot: {r}"), self._set_status("готов")))
        worker.start()

    def _voice_input(self) -> None:
        self.chat_log.append("🎤 Слушаю... (запись 5 сек)")
        self._set_status("слушаю...")
        from meshpilot.voice.stt import STT, record_wav
        import tempfile
        tmp = tempfile.mktemp(suffix=".wav")
        try:
            record_wav(tmp, seconds=5.0)
        except Exception as e:
            self.chat_log.append(f"Микрофон недоступен: {e}")
            self._set_status("готов")
            return
        stt = STT()
        worker = self._start_worker(stt.transcribe_file, tmp)
        worker.finished.connect(lambda r: (self.chat_log.append(f"🎤 Распознано: {r}"), self.chat_input.setText(r), self._set_status("готов")))
        worker.start()

    def _opt_clean_all(self) -> None:
        from meshpilot.optimizer import cleaner
        dry = self.dry_run_check.isChecked()
        self.opt_progress.show()
        self._set_status("чищу...")
        worker = self._start_worker(lambda: "\n".join(f"{r.name}: {r.details}" for r in cleaner.clean_all(dry_run=dry)))
        worker.finished.connect(lambda r: (self.opt_log.append(r), self.opt_progress.hide(), self._set_status("готов"), self._refresh_stats()))
        worker.start()

    def _opt_cache(self) -> None:
        from meshpilot.optimizer import cleaner
        dry = self.dry_run_check.isChecked()
        worker = self._start_worker(lambda: str(cleaner.clean_pacman_cache(dry_run=dry).details))
        worker.finished.connect(self.opt_log.append)
        worker.start()

    def _opt_orphans(self) -> None:
        from meshpilot.optimizer import cleaner
        dry = self.dry_run_check.isChecked()
        worker = self._start_worker(lambda: str(cleaner.remove_orphans(dry_run=dry).details))
        worker.finished.connect(self.opt_log.append)
        worker.start()

    def _opt_mirrors(self) -> None:
        from meshpilot.optimizer import mirrors
        dry = self.dry_run_check.isChecked()
        worker = self._start_worker(mirrors.auto_mirrors, dry_run=dry)
        worker.finished.connect(self.opt_log.append)
        worker.start()

    def _opt_health(self) -> None:
        from meshpilot.optimizer import health
        worker = self._start_worker(health.health_report)
        worker.finished.connect(self.opt_log.append)
        worker.start()

    def _zapret_status(self) -> None:
        from meshpilot.optimizer import zapret
        worker = self._start_worker(lambda: str(zapret.get_status()))
        worker.finished.connect(self.zapret_log.append)
        worker.start()

    def _zapret_install(self) -> None:
        from meshpilot.optimizer import zapret
        dry = self.zapret_dry.isChecked()
        worker = self._start_worker(zapret.install, dry_run=dry)
        worker.finished.connect(self.zapret_log.append)
        worker.start()

    def _zapret_auto(self) -> None:
        from meshpilot.optimizer import zapret
        dry = self.zapret_dry.isChecked()
        worker = self._start_worker(zapret.auto_strategy, dry_run=dry)
        worker.finished.connect(self.zapret_log.append)
        worker.start()

    def _zapret_restart(self) -> None:
        from meshpilot.optimizer import zapret
        worker = self._start_worker(zapret.restart)
        worker.finished.connect(self.zapret_log.append)
        worker.start()

    def _dns_bench(self) -> None:
        from meshpilot.optimizer import dns
        dry = self.zapret_dry.isChecked()
        worker = self._start_worker(dns.auto_dns, dry_run=dry)
        worker.finished.connect(self.zapret_log.append)
        worker.start()
