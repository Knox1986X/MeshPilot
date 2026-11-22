"""Live2D рендер для MeshPilot (как у Втуберов) — Cubism 4 через PixiJS.

Live2D Cubism — стандарт Втуберов: 2D-модель с ригом (.model3.json + .moc3 + текстуры + physics).
Рендер: QWebEngineView + PixiJS + @pixi/live2d-display (Cubism 4, поддерживает .moc3 от Booth).

Почему PixiJS, а не live2d-widget 3.1.4:
- live2d-widget 3.1.4 — это Cubism 2 эра, не умеет .moc3 (Cubism 4) от Booth (Jane Doe и др.)
- @pixi/live2d-display 0.5 — Cubism 4, expressions/motions/physics из коробки

Модель отдаётся через локальный HTTP (127.0.0.1), а не file://:
- file:// блокируется CORS в Chromium (не грузит .moc3/текстуры)
- китайские имена файлов (简.model3.json) ломают file:// URL
- HTTP решает оба

Управление из Python через page().runJavaScript:
  - setMouth(0..1), setExpression(name), setLook(x, y), setMotion(name)
"""
from __future__ import annotations
import json
import logging
import threading
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import quote

log = logging.getLogger("meshpilot.live2d")

# PixiJS 7 + Live2D Display 0.5 (Cubism 4) с jsdelivr (надёжнее unpkg в RU)
LIVE2D_HTML = """<!DOCTYPE html>
<html><head><meta charset="utf-8">
<style>
html,body{margin:0;padding:0;background:transparent !important;overflow:hidden;width:100%;height:100%}
canvas{display:block;background:transparent !important}
#err{position:absolute;top:8px;left:8px;color:#f66;font:12px monospace;white-space:pre-wrap;max-width:90%}
</style>
<script src="https://cdn.jsdelivr.net/npm/pixi.js@7.4.2/dist/pixi.min.js"></script>
<script src="https://cdn.jsdelivr.net/npm/@pixi/live2d-display@0.5.0/dist/index.min.js"></script>
</head><body>
<div id="err"></div>
<script>
let app = null, model = null;
let mouthOpen = 0;

function showErr(msg) {
  const el = document.getElementById('err');
  if (el) el.textContent = msg;
  console.error(msg);
}

async function loadModel(modelUrl) {
  try {
    if (typeof PIXI === 'undefined') { showErr('PIXI не загрузился (нет интернета для CDN)'); return 'error: no PIXI'; }
    if (!PIXI.live2d) { showErr('Live2D Display не загрузился'); return 'error: no live2d'; }
    app = new PIXI.Application({
      width: 400, height: 500,
      backgroundAlpha: 0,  // прозрачный фон!
      antialias: true,
    });
    document.body.appendChild(app.view);
    model = await PIXI.live2d.Live2DModel.from(modelUrl);
    model.scale.set(0.35);
    model.position.set(200, 480);
    model.anchor.set(0.5, 1.0);
    app.stage.add(model);
    // idle анимация
    try { model.motion('Idle'); } catch(e) {}
    return 'ok';
  } catch(e) {
    showErr('loadModel error: ' + (e && e.message ? e.message : e));
    return 'error: ' + (e && e.message ? e.message : e);
  }
}

function setMouth(v) {
  if (!model) return;
  mouthOpen = Math.max(0, Math.min(1, v));
  try {
    // Cubism 4: ParamMouthOpenY
    model.internalModel.coreModel.setParameterValueById('ParamMouthOpenY', mouthOpen);
  } catch(e) {}
}

function setExpression(name) {
  if (!model || !name) return;
  try { model.expression(name); } catch(e) {}
}

function setLook(x, y) {
  if (!model) return;
  try {
    model.internalModel.coreModel.setParameterValueById('ParamAngleX', x * 30);
    model.internalModel.coreModel.setParameterValueById('ParamAngleY', y * 30);
    model.internalModel.coreModel.setParameterValueById('ParamEyeBallX', x);
    model.internalModel.coreModel.setParameterValueById('ParamEyeBallY', y);
  } catch(e) {}
}

function setMotion(name) {
  if (!model) return;
  try { model.motion(name); } catch(e) {}
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
        file_refs = self.model_json.get("FileReferences", {})
        exprs = file_refs.get("Expressions", []) or file_refs.get("Expressions3", [])
        self.expressions = []
        for e in exprs:
            if isinstance(e, dict):
                name = e.get("Name") or e.get("name") or ""
                if name:
                    self.expressions.append(name)
            elif isinstance(e, str):
                self.expressions.append(e)
        if not self.expressions:
            for f in sorted(self.path.parent.glob("*.exp3.json")):
                self.expressions.append(f.stem)
        motions = file_refs.get("Motions", {}) or file_refs.get("Motions3", {})
        if isinstance(motions, dict):
            self.motions = list(motions.keys())
        elif isinstance(motions, list):
            self.motions = [str(m) for m in motions]
        else:
            self.motions = []
        if not self.motions:
            for f in sorted(self.path.parent.glob("*.motion3.json")):
                self.motions.append(f.stem)
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
        """file:// URL (fallback, если HTTP сервер недоступен)."""
        return self.path.resolve().as_uri()

    def http_url(self, base_url: str) -> str:
        """HTTP URL через локальный сервер (основной путь, без CORS).

        base_url: http://127.0.0.1:PORT (корень = assets/models/)
        Возвращает URL-encoded путь от корня.
        """
        root = _find_models_root(self.path)
        rel = self.path.resolve().relative_to(root.resolve())
        # quote каждый сегмент (китайские имена -> %E7%AE%80...)
        parts = [quote(p, safe="") for p in rel.parts]
        return base_url.rstrip("/") + "/" + "/".join(parts)


def _find_models_root(model_path: Path) -> Path:
    """Ищет корень assets/models/ вверх от модели (для HTTP сервера)."""
    cur = model_path.resolve().parent
    for _ in range(6):
        if cur.name == "models" and (cur.parent.name == "assets"):
            return cur
        if cur.name == "assets":
            m = cur / "models"
            if m.exists():
                return m
        cur = cur.parent
    # fallback: папка модели
    return model_path.resolve().parent


class ModelHTTPServer:
    """Локальный HTTP сервер для файлов модели (без CORS, с китайскими именами)."""

    def __init__(self, root: Path) -> None:
        self.root = Path(root).resolve()
        self._server: ThreadingHTTPServer | None = None
        self._thread: threading.Thread | None = None
        self.port: int = 0

    def start(self) -> str:
        """Запускает сервер, возвращает base_url http://127.0.0.1:PORT."""
        root = self.root

        class Handler(SimpleHTTPRequestHandler):
            def __init__(self, *args, **kwargs):
                super().__init__(*args, directory=str(root), **kwargs)

            def log_message(self, fmt, *args):  # тихий лог
                pass

        self._server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        self.port = self._server.server_address[1]
        self._thread = threading.Thread(target=self._server.serve_forever, daemon=True)
        self._thread.start()
        url = f"http://127.0.0.1:{self.port}"
        log.info("Model HTTP server: %s (root=%s)", url, root)
        return url

    def stop(self) -> None:
        if self._server:
            self._server.shutdown()
            self._server = None


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
        for f in p.rglob("*.model3.json"):
            return "live2d"
        for f in p.rglob("*.vrm"):
            return "vrm"
    return "unknown"

def find_model_in_dir(directory: str | Path) -> Path | None:
    """Ищет модель в папке (рекурсивно): сначала .model3.json, потом .vrm, потом .png."""
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
