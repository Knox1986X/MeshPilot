from meshpilot.voice.voices import get_profile, PROFILES

def test_profiles_exist():
    assert set(PROFILES.keys()) == {"jarvis", "khan", "anime"}

def test_jarvis_british():
    p = get_profile("jarvis")
    assert "en_GB" in p.piper_voice or "en-GB" in p.edge_voice

def test_khan_deep():
    p = get_profile("khan")
    assert p.pitch < 0
    assert p.rate < 1.0

def test_anime_energetic():
    p = get_profile("anime")
    assert p.pitch > 0
    assert p.rate > 1.0
