"""Overlay-окно персонажа MeshPilot (PySide6): Live2D / VRM / PNG.

«Сидит» на окнах: прозрачное, always-on-top, draggable, dock к краям,
кликабельное (клик = голосовой ввод), click-through для игр.

- Live2D (.model3.json): QWebEngineView + PixiJS Cubism 4, модель через локальный HTTP
- VRM (.vrm): QWebEngineView + three.js + three-vrm
- PNG: fallback с blink/bounce (если WebEngine недоступен или модель PNG)

Прозрачность WebEngine: page background transparent + WA_TranslucentBackground,
иначе белый прямоугольник вместо модели.
"""
from __future__ import annotations
import logging
from pathlib import Path

from PySide6.QtCore import Qt, QTimer, QPoint
from PySide6.QtGui import QPixmap, QPainter, QColor
from PySide6.QtWidgets import QWidget, QApplication, QVBoxLayout

from meshpilot.avatar.character import Character, AvatarState
from meshpilot.avatar.model_loader import load_model, LoadedModel

log = logging.getLogger("meshpilot.overlay")

class OverlayWidget(QWidget):
    def __init__(
        self,
        character: Character | None = None,
        model_path: str | None = None,
    ) -> None:
        if model_path is None:
            try:
                from meshpilot.config import config as _cfg
                model_path = _cfg.get("avatar", "model", default="assets/models/meshpilot.model3.json")
            except Exception:
                model_path = "assets/models/meshpilot.model3.json"
        super().__init__()
        self.character = character or Character()
        self.model_path = Path(model_path)
        self.loaded: LoadedModel | None = None
        self._webview = None
        self._web_ready = False
        self._http_server = None
        self._drag_offset = QPoint()
        self._mouth = 0.0
        self._blink = 0.0
        self._use_web = False

        self.setWindowFlags(Qt.WindowStaysOnTopHint | Qt.FramelessWindowHint | Qt.Tool)
        self.setAttribute(Qt.WA_TranslucentBackground, True)
        self.setAttribute(Qt.WA_TransparentForMouseEvents, False)
        self.resize(400, 500)

        layout = QVBoxLayout(self)
        layout.setContentsMargins(0, 0, 0, 0)

        self._try_load_web_model(layout)

        self._timer = QTimer(self)
        self._timer.timeout.connect(self._tick)
        self._timer.start(50)

    def _try_load_web_model(self, layout) -> None:
        """Пытается поднять Live2D/VRM. При неудаче — PNG fallback."""
        try:
            self.loaded = load_model(self.model_path)
            log.info("Модель персонажа: %s", self.loaded)
        except Exception as e:
            log.warning("Модель не загружена (%s), PNG fallback", e)
            self.loaded = None
            return

        if self.loaded.type in ("live2d", "vrm"):
            try:
                from PySide6.QtWebEngineWidgets import QWebEngineView
                from meshpilot.avatar.live2d import LIVE2D_HTML, ModelHTTPServer, _find_models_root
                from meshpilot.avatar.vrm import VRM_HTML

                # HTTP сервер для файлов модели + vendor SDK (без CORS, с китайскими именами)
                # Корень = assets/ (отдаёт и models/, и vendor/)
                if self.loaded.type == "live2d":
                    from meshpilot.avatar.live2d import _find_assets_root
                    root = _find_assets_root(self.loaded.path)
                    self._http_server = ModelHTTPServer(root)
                    base_url = self._http_server.start()
                    self._model_http_url = self.loaded.model_obj.http_url(base_url)
                    self._vendor_base = base_url + "/vendor"
                    log.info("Live2D HTTP URL: %s", self._model_http_url)
                else:
                    self._model_http_url = self.loaded.model_obj.model_url()
                    self._vendor_base = "" 

                self._webview = QWebEngineView(self)
                self._webview.setAttribute(Qt.WA_TranslucentBackground, True)
                # прозрачный фон страницы — иначе белый прямоугольник!
                try:
                    page = self._webview.page()
                    if page is not None:
                        page.setBackgroundColor(Qt.transparent)
                except Exception:
                    pass
                html = LIVE2D_HTML if self.loaded.type == "live2d" else VRM_HTML
                # подставляем локальный vendor (офлайн, без CDN)
                if self.loaded.type == "live2d" and getattr(self, "_vendor_base", ""):
                    html = html.replace("__VENDOR_BASE__", self._vendor_base)
                self._webview.setHtml(html)
                layout.addWidget(self._webview)
                self._use_web = True
                self._webview.loadFinished.connect(self._on_web_loaded)
                log.info("WebEngine overlay для %s", self.loaded.type)
            except ImportError as e:
                log.warning("QWebEngineView недоступен (%s), PNG fallback", e)
                self._use_web = False
            except Exception as e:
                log.warning("WebEngine overlay failed (%s), PNG fallback", e)
                self._use_web = False
        else:
            log.info("PNG overlay: %s", self.loaded.path)

    def _js(self, script: str, require_ready: bool = True) -> None:
        """Безопасный вызов JS в WebEngine (PySide6: page().runJavaScript)."""
        if self._webview is None:
            return
        if require_ready and not self._web_ready:
            return
        try:
            page = self._webview.page()
            if page is not None:
                page.runJavaScript(script)
        except Exception:
            pass

    def _on_web_loaded(self, ok: bool) -> None:
        if not ok or self._webview is None or self.loaded is None:
            return
        url = getattr(self, "_model_http_url", "")
        if not url and self.loaded.model_obj:
            try:
                url = self.loaded.model_obj.model_url()
            except Exception as e:
                log.warning("Не удалось получить URL модели: %s", e)
                return
        if self.loaded.type == "live2d":
            self._js(f"loadModel('{url}')", require_ready=False)
            QTimer.singleShot(4000, lambda: setattr(self, "_web_ready", True))
        elif self.loaded.type == "vrm":
            self._js(f"loadVRM('{url}')", require_ready=False)
            QTimer.singleShot(4000, lambda: setattr(self, "_web_ready", True))

    def _tick(self) -> None:
        params = self.character.tick(0.05)
        self._blink = params["blink"]
        if self._use_web and self._webview is not None:
            self._js(f"setMouth({self._mouth:.3f})")
        else:
            self.update()

    EXPRESSION_MAP = {
        AvatarState.SPEAKING: ["smile", "脸红", "爱心眼", "星星眼"],
        AvatarState.LISTENING: ["neutral", ""],
        AvatarState.THINKING: ["thinking", "白眼"],
        AvatarState.ERROR: ["sad", "泪", "脸黑", "血"],
        AvatarState.IDLE: ["neutral", ""],
        AvatarState.SLEEPING: ["sleepy", ""],
    }

    def set_state(self, state: AvatarState | str) -> None:
        self.character.set_state(state)
        if self._use_web and self._webview is not None:
            st = AvatarState(state) if isinstance(state, str) else state
            candidates = self.EXPRESSION_MAP.get(st, ["neutral"])
            expr = candidates[0]
            if self.loaded is not None and self.loaded.type == "live2d" and self.loaded.model_obj is not None:
                available = set(self.loaded.model_obj.expressions)
                for c in candidates:
                    if c in available:
                        expr = c
                        break
            if expr:
                self._js(f"setExpression('{expr}')")

    def set_mouth(self, value: float) -> None:
        self._mouth = max(0.0, min(1.0, value))

    def set_look(self, x: float, y: float) -> None:
        """Взгляд за курсором/микрофоном: x,y в -1..1."""
        if self._use_web and self._webview is not None:
            self._js(f"setLook({x:.3f}, {y:.3f})")

    def paintEvent(self, event) -> None:  # noqa: N802
        if self._use_web:
            return
        painter = QPainter(self)
        painter.setRenderHint(QPainter.Antialiasing)
        img_path = self.loaded.path if self.loaded and self.loaded.type == "png" else Path("assets/models/meshpilot.png")
        if img_path.exists():
            pix = QPixmap(str(img_path))
            w, h = self.width(), self.height()
            scale_y = 1.0 - self._blink * 0.08
            scaled = pix.scaled(int(w), int(h * scale_y), Qt.KeepAspectRatio, Qt.SmoothTransformation)
            x = (w - scaled.width()) // 2
            y = (h - scaled.height()) // 2
            painter.drawPixmap(x, y, scaled)
            if self.character.state == AvatarState.SPEAKING and self._mouth > 0.05:
                cx, cy = w // 2, int(h * 0.72)
                painter.setBrush(QColor(40, 20, 20, 200))
                painter.setPen(Qt.NoPen)
                painter.drawEllipse(cx - 10, cy - int(6 * self._mouth), 20, int(12 * self._mouth))
        else:
            painter.setBrush(QColor(30, 30, 40, 220))
            painter.setPen(QColor(120, 200, 255))
            painter.drawEllipse(20, 20, self.width() - 40, self.height() - 40)
            painter.setPen(QColor(200, 230, 255))
            painter.drawText(self.rect(), Qt.AlignCenter, "M")
        painter.end()

    def mousePressEvent(self, event) -> None:  # noqa: N802
        if event.button() == Qt.LeftButton:
            self._drag_offset = event.pos()

    def mouseMoveEvent(self, event) -> None:  # noqa: N802
        if event.buttons() & Qt.LeftButton:
            self.move(self.pos() + event.pos() - self._drag_offset)

    def mouseDoubleClickEvent(self, event) -> None:  # noqa: N802
        self.set_state(AvatarState.LISTENING)

    def dock_to(self, position: str = "bottom-right") -> None:
        screen = QApplication.primaryScreen().geometry()
        x, y = self.x(), self.y()
        if position == "bottom-right":
            x, y = screen.width() - self.width() - 12, screen.height() - self.height() - 64
        elif position == "bottom-left":
            x, y = 12, screen.height() - self.height() - 64
        elif position == "top-right":
            x, y = screen.width() - self.width() - 12, 12
        elif position == "top-left":
            x, y = 12, 12
        self.move(x, y)

    def set_click_through(self, enabled: bool) -> None:
        self.setAttribute(Qt.WA_TransparentForMouseEvents, enabled)

    def closeEvent(self, event) -> None:  # noqa: N802
        if self._http_server:
            self._http_server.stop()
        super().closeEvent(event)
