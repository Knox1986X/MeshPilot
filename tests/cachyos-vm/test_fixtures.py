"""Проверки выбора носителя без сети, KVM и системных изменений."""
import subprocess
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent


class Fixtures(unittest.TestCase):
    def test_reject_unpinned_id_before_io(self):
        with tempfile.TemporaryDirectory() as tmp:
            dest = Path(tmp) / "media"
            for name in ("", "latest", "../251129", "https://example.invalid/img"):
                p = subprocess.run([HERE / "media.sh", dest, name], capture_output=True, timeout=3)
                self.assertEqual(p.returncode, 2)
                self.assertFalse(dest.exists())
                q = subprocess.run([HERE / "run.sh", "iso", "cli", "gui", name], capture_output=True, timeout=3)
                self.assertEqual(q.returncode, 2)

    def test_neither_fixture_overwrites_existing_media(self):
        with tempfile.TemporaryDirectory() as tmp:
            for release in ("260809", "251129"):
                path = Path(tmp) / f"cachyos-desktop-linux-{release}.iso"
                path.write_bytes(b"existing")
                p = subprocess.run([HERE / "media.sh", tmp, release], capture_output=True, timeout=3)
                self.assertEqual(p.returncode, 2)
                self.assertEqual(path.read_bytes(), b"existing")


if __name__ == "__main__":
    unittest.main()
