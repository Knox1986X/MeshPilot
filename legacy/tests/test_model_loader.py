from pathlib import Path
from meshpilot.avatar.live2d import detect_model_type, find_model_in_dir

def test_detect_live2d():
    assert detect_model_type("miko.model3.json") == "live2d"
    assert detect_model_type("model.moc3") == "live2d"

def test_detect_vrm():
    assert detect_model_type("avatar.vrm") == "vrm"

def test_detect_png():
    assert detect_model_type("meshpilot.png") == "png"
    assert detect_model_type("photo.jpg") == "png"

def test_detect_unknown():
    assert detect_model_type("model.txt") == "unknown"

def test_find_in_dir(tmp_path):
    (tmp_path / "miko.model3.json").write_text("{}")
    found = find_model_in_dir(tmp_path)
    assert found is not None
    assert found.name == "miko.model3.json"

def test_find_png_fallback(tmp_path):
    (tmp_path / "pic.png").write_bytes(b"fake")
    found = find_model_in_dir(tmp_path)
    assert found is not None
    assert found.suffix == ".png"

def test_find_nested_booth(tmp_path):
    """Booth-архивы часто вложенные: jane/简/简.model3.json"""
    nested = tmp_path / "jane" / "简"
    nested.mkdir(parents=True)
    (nested / "简.model3.json").write_text("{}")
    found = find_model_in_dir(tmp_path / "jane")
    assert found is not None
    assert found.name == "简.model3.json"
