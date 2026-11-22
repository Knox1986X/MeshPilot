"""Live2D рендер для MeshPilot (как у Втуберов).

Live2D Cubism — стандарт Втуберов: 2D-модель с ригом (.model3.json + .moc3 + текстуры + physics).
Работает через QWebEngineView + Cubism Web SDK (JS), поэтому не нужны нативные зависимости.

Модель: папка с .model3.json, .moc3, .png текстурами, physics3.json, expressions.
Пример структуры:
  assets/models/myvtuber/
    myvtuber.model3.json
    myvtuber.moc3
    texture_00.png
    physics3.json
    expressions/...

Управление из Python через runJavaScript:
  - setMouth(0..1) — lip-sync
  - setExpression("smile") — эмоции
  - setMotion("idle"/"tap") — анимации
  - setLook(x, y) — взгляд за курсором/микрофоном
"""
from __future__ import annotations
import json
import logging
from pathlib import Path

log = logging.getLogger("meshpilot.live2d")

LIVE2D_HTML = """<!DOCTYPE html>
<html><head><meta charset="utf-8">
<style>html,body{margin:0;padding:0;background:transparent;overflow:hidden}
canvas{display:block}</style>
<script src="https://cubism.live2d.com/sdk-web/cubism2.min.js"></script>
<script src="https://unpkg.com/live2d-widget@3.1.4/lib/live2d.min.js"></script>
</head><body>
<canvas id="canvas"></canvas>
<script>
let model = null;
let mouthOpen = 0;
let lookX = 0, lookY = 0;
let currentExpression = null;

async function loadModel(modelUrl) {
  try {
    // Live2D Widget API (простой путь для .model3.json)
    window.L2Dwidget.init({
      model: { json: modelUrl },
      display: { position: "absolute", width: 400, height: 500, left: 0, top: 0 },
      react: { opacity: 1.0 }
    });
    return "ok";
  } catch(e) {
    return "error: " + e.message;
  }
}

function setMouth(v) {
  mouthOpen = Math.max(0, Math.min(1, v));
  // Live2D Widget: mouth через параметры модели
  try {
    if (window.L2Dwidget && window.L2Dwidget.model) {
      window.L2Dwidget.model.setParameterValueById("ParamMouthOpenY", mouthOpen);
    }
  } catch(e) {}
}

function setExpression(name) {
  currentExpression = name;
  try {
    if (window.L2Dwidget && window.L2Dwidget.model) {
      window.L2Dwidget.model.setExpression(name);
    }
  } catch(e) {}
}

function setLook(x, y) {
  lookX = x; lookY = y;
  try {
    if (window.L2Dwidget && window.L2Dwidget.model) {
      window.L2Dwidget.model.setParameterValueById("ParamAngleX", x * 30);
      window.L2Dwidget.model.setParameterValueById("ParamAngleY", y * 30);
      window.L2Dwidget.model.setParameterValueById("ParamEyeBallX", x);
      window.L2Dwidget.model.setParameterValueById("ParamEyeBallY", y);
    }
  } catch(e) {}
}

function setMotion(name) {
  try {
    if (window.L2Dwidget && window.L2Dwidget.model) {
      window.L2Dwidget.model.startMotion(name);
    }
  } catch(e) {}
}
</script>
</body></html>
"""

class Live2DModel:
    """Обёртка над Live2D моделью: детект, валидация, параметры."""

    def __init__(self, model_path: str | Path) -> None:
        self.path = Path(model_path)
        self.model_json: dict = {}
        self.expressions: list[str] = []
        self.motions: list[str] = []
        self._validate()

    def _validate(self) -> None:
        if not self.path.exists():
            raise FileNotFoundError(f"Live2D модель не найдена: {self.path}")
        if self.path.suffix != ".json":
            raise ValueError(f"Live2D модель должна быть .model3.json, получено: {self.path}")
        self.model_json = json.loads(self.path.read_text(encoding="utf-8"))
        # парсим expressions и motions из model3.json
        file_refs = self.model_json.get("FileReferences", {})
        self.expressions = [e.get("Name", "") for e in file_refs.get("Expressions", [])]
        self.motions = list(file_refs.get("Motions", {}).keys())
        log.info("Live2D модель: %s expressions=%s motions=%s", self.path.name, self.expressions, self.motions)

    @property
    def moc3_path(self) -> Path:
        refs = self.model_json.get("FileReferences", {})
        moc = refs.get("Moc", "")
        return self.path.parent / moc if moc else self.path.with_suffix(".moc3")

    @property
    def is_valid(self) -> bool:
        return self.moc3_path.exists()

    def model_url(self) -> str:
        """file:// URL для QWebEngineView."""
        return self.path.as_uri()

def detect_model_type(path: str | Path) -> str:
    """Определяет тип модели по расширению: live2d | vrm | png | unknown."""
    p = Path(path)
    suffix = p.suffix.lower()
    name = p.name.lower()
    if name.endswith(".model3.json") or suffix == ".moc3":
        return "live2d"
    if suffix == ".vrm":
        return "vrm"
    if suffix in {".png", ".jpg", ".jpeg", ".webp", ".gif"}:
        return "png"
    if p.is_dir():
        # папка с моделью — ищем рекурсивно (Booth-архивы часто вложенные)
        for f in p.rglob("*.model3.json"):
            return "live2d"
        for f in p.rglob("*.vrm"):
            return "vrm"
    return "unknown"

def find_model_in_dir(directory: str | Path) -> Path | None:
    """Ищет модель в папке (рекурсивно): сначала .model3.json, потом .vrm, потом .png.

    Поддерживает вложенные папки Booth-архивов, напр.:
      assets/models/jane/简/简.model3.json
    """
    d = Path(directory)
    if not d.exists():
        return None
    if d.is_file():
        return d
    for pattern in ["*.model3.json", "*.vrm", "*.png", "*.jpg"]:
        found = sorted(d.rglob(pattern))
        if found:
            return found[0]
    return None
