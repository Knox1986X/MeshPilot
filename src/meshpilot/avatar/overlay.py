"""Overlay-окно персонажа MeshPilot (PySide6): Live2D / VRM / PNG.

«Сидит» на окнах: прозрачное, always-on-top, draggable, dock к краям,
кликабельное (клик = голосовой ввод), click-through для игр.

- Live2D (.model3.json): QWebEngineView + Cubism Web SDK, lip-sync/expressions/physics
- VRM (.vrm): QWebEngineView + three.js + three-vrm, blendshapes, idle
- PNG: fallback с blink/bounce (если WebEngine недоступен или модель PNG)
"""
from __future__ import annotations
import logging
from pathlib import Path

from PySide6.QtCore import Qt, QTimer, QPoint, QUrl
from PySide6.QtGui import QPixmap, QPainter, QColor
from PySide6.QtWidgets import QWidget, QApplication, QVBoxLayout

from meshpilot.avatar.character import Character, AvatarState
from meshpilot.avatar.model_loader import load_model, LoadedModel

log = logging.getLogger("meshpilot.overlay")

class OverlayWidget(QWidget):
    def __init__(
        self,
        character: Character | None = None,
        model_path: str = "assets/models/meshpilot.model3.json",
    ) -> None:
        super().__init__()
        self.character = character or Character()
        self.model_path = Path(model_path)
        self.loaded: LoadedModel | None = None
        self._webview = None
        self._drag_offset = QPoint()
        self._mouth = 0.0
        self._blink = 0.0
        self._use_web = False

        self.setWindowFlags(Qt.WindowStaysOnTopHint | Qt.FramelessWindowHint | Qt.Tool)
        self.setAttribute(Qt.WA_TranslucentBackground, True)
        self.setAttribute(Qt.WA_TransparentForMouseEvents, False)
        self.resize(280, 340)

        layout = QVBoxLayout(self)
        layout.setContentsMargins(0, 0, 0, 0)

        # пробуем загрузить Live2D/VRM через WebEngine
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
                from meshpilot.avatar.live2d import LIVE2D_HTML
                from meshpilot.avatar.vrm import VRM_HTML

                self._webview = QWebEngineView(self)
                self._webview.setAttribute(Qt.WA_TranslucentBackground, True)
                html = LIVE2D_HTML if self.loaded.type == "live2d" else VRM_HTML
                self._webview.setHtml(html)
                layout.addWidget(self._webview)
                self._use_web = True
                # загружаем модель после загрузки страницы
                self._webview.loadFinished.connect(self._on_web_loaded)
                log.info("WebEngine overlay для %s", self.loaded.type)
            except ImportError as e:
                log.warning("QWebEngineView недоступен (%s), PNG fallback", e)
                self._use_web = False
        else:
            log.info("PNG overlay: %s", self.loaded.path)

    def _on_web_loaded(self, ok: bool) -> None:
        if not ok or self._webview is None or self.loaded is None:
            return
        url = self.loaded.model_obj.model_url() if self.loaded.model_obj else ""
        if self.loaded.type == "live2d":
            self._webview.runJavaScript(f"loadModel('{url}')")
        elif self.loaded.type == "vrm":
            self._webview.runJavaScript(f"loadVRM('{url}')")

    def _tick(self) -> None:
        params = self.character.tick(0.05)
        self._blink = params["blink"]
        if self._use_web and self._webview is not None:
            # lip-sync + взгляд в Web-модель
            self._webview.runJavaScript(f"setMouth({self._mouth:.3f})")
        else:
            self.update()

    # Маппинг эмоций MeshPilot -> имена expressions в модели.
    # Поддерживает английские (smile/sad) и китайские (Jane Doe ZZZ: 脸红/生气/泪...).
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
            # выбираем первое имя, которое есть в модели (если модель загружена)
            expr = candidates[0]
            if self.loaded is not None and self.loaded.type == "live2d" and self.loaded.model_obj is not None:
                available = set(self.loaded.model_obj.expressions)
                for c in candidates:
                    if c in available:
                        expr = c
                        break
            if expr:
                self._webview.runJavaScript(f"setExpression('{expr}')")

    def set_mouth(self, value: float) -> None:
        self._mouth = max(0.0, min(1.0, value))

    def set_look(self, x: float, y: float) -> None:
        """Взгляд за курсором/микрофоном: x,y в -1..1."""
        if self._use_web and self._webview is not None:
            self._webview.runJavaScript(f"setLook({x:.3f}, {y:.3f})")

    def paintEvent(self, event) -> None:  # noqa: N802
        if self._use_web:
            return  # рисует WebEngine
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
