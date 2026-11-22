from meshpilot.avatar.character import Character, AvatarState

def test_states():
    c = Character()
    c.set_state(AvatarState.SPEAKING)
    assert c.state == AvatarState.SPEAKING
    params = c.tick(0.05)
    assert "blink" in params and "mouth" in params
