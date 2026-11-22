from meshpilot.config import config

def test_voice_names():
    names = config.voice_names()
    assert "jarvis" in names
    assert "khan" in names
    assert "anime" in names

def test_voice_profile():
    p = config.voice_profile("jarvis")
    assert "persona" in p
    assert "tts" in p

def test_zapret_strategies():
    assert len(config.zapret.get("strategies", [])) >= 3
