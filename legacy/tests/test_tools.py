import asyncio
from meshpilot.optimizer.tools import ALL_TOOLS

def test_tools_registered():
    names = [t[0] for t in ALL_TOOLS]
    assert "clean_all" in names
    assert "optimize_mirrors" in names
    assert "zapret_status" in names
    assert "system_health" in names

def test_health_tool():
    async def _run():
        from meshpilot.optimizer.tools import t_health
        return await t_health()
    out = asyncio.run(_run())
    assert "Диск" in out or "disk" in out.lower() or len(out) > 10
