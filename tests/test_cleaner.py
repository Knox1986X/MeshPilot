from meshpilot.optimizer import cleaner

def test_clean_all_dry_run():
    results = cleaner.clean_all(dry_run=True)
    assert len(results) >= 10
    for r in results:
        assert r.name

def test_pacman_cache_dry():
    r = cleaner.clean_pacman_cache(dry_run=True)
    assert r.name == "pacman cache"

def test_orphans_dry():
    r = cleaner.remove_orphans(dry_run=True)
    assert r.name == "orphans"
