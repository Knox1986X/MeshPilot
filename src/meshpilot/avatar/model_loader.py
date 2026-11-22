"""Автодетект и загрузка модели персонажа: Live2D | VRM | PNG.

Порядок: .model3.json (Live2D) -> .vrm (VRM) -> .png (fallback).
"""
from __future__ import annotations
import logging
from pathlib import Path

from meshpilot.avatar.live2d import Live2DModel, detect_model_type, find_model_in_dir
from meshpilot.avatar.vrm import VRMModel

log = logging.getLogger("meshpilot.model_loader")

class LoadedModel:
    def __init__(self, model_type: str, path: Path, model_obj=None) -> None:
        self.type = model_type  # live2d | vrm | png
        self.path = path
        self.model_obj = model_obj

    def __repr__(self) -> str:
        return f"LoadedModel({self.type}, {self.path})"

def load_model(model_path: str | Path) -> LoadedModel:
    """Загружает модель по пути (файл или папка)."""
    p = Path(model_path)
    # если папка — ищем внутри
    if p.is_dir():
        found = find_model_in_dir(p)
        if found is None:
            raise FileNotFoundError(f"В папке {p} нет модели (.model3.json/.vrm/.png)")
        p = found
    model_type = detect_model_type(p)
    if model_type == "live2d":
        obj = Live2DModel(p)
        if not obj.is_valid:
            log.warning("Live2D .moc3 не найден рядом с %s, проверяй папку модели", p)
        return LoadedModel("live2d", p, obj)
    if model_type == "vrm":
        obj = VRMModel(p)
        return LoadedModel("vrm", p, obj)
    if model_type == "png":
        if not p.exists():
            raise FileNotFoundError(f"PNG модель не найдена: {p}")
        return LoadedModel("png", p, None)
    raise ValueError(f"Неизвестный тип модели: {p}. Поддерживаются .model3.json (Live2D), .vrm, .png")

def default_model_path() -> Path:
    """Путь по умолчанию из конфига."""
    from meshpilot.config import config
    model = config.get("avatar", "model", default="assets/models/meshpilot.model3.json")
    return Path(model)
