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

    def test_only_recognizable_html_is_a_delivery_miss(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "response"
            for data, html in [(b"<!DOCTYPE html>\n<html lang=en>", True), (b"  <html>", True),
                               (b"<htmlish>", False), (b"wrong bytes", False), (bytes(2048), False)]:
                path.write_bytes(data)
                p = subprocess.run(["bash", "-c", 'source "$1"; html_response "$2"', "test",
                                    HERE / "fixtures.sh", path], capture_output=True, timeout=3)
                self.assertEqual(p.returncode == 0, html)

    def test_no_profile_overwrites_existing_media(self):
        with tempfile.TemporaryDirectory() as tmp:
            for profile, edition, release in [("260809", "desktop", "260809"), ("251129", "desktop", "251129"),
                                              ("handheld-260628", "handheld", "260628")]:
                path = Path(tmp) / f"cachyos-{edition}-linux-{release}.iso"
                path.write_bytes(b"existing")
                p = subprocess.run([HERE / "media.sh", tmp, profile], capture_output=True, timeout=3)
                self.assertEqual(p.returncode, 2)
                self.assertEqual(path.read_bytes(), b"existing")


if __name__ == "__main__":
    unittest.main()
