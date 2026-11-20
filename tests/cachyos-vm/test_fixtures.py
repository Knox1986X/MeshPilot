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

    def test_archive_is_local_only(self):
        with tempfile.TemporaryDirectory() as tmp:
            dest = Path(tmp) / "archive"
            out = subprocess.run([HERE / "media.sh", dest, "251129"], capture_output=True, timeout=3)
            self.assertEqual(out.returncode, 2)
            self.assertFalse(dest.exists())

    def test_standalone_media_verifier_rejects_missing_signature_and_wrong_digest(self):
        with tempfile.TemporaryDirectory() as tmp:
            iso = Path(tmp) / "looks-valid.iso"
            iso.write_bytes(b"not an ISO")
            cmd = [HERE / "verify.sh", iso, "handheld-260628"]
            p = subprocess.run(cmd, capture_output=True, text=True, timeout=3)
            self.assertEqual(p.returncode, 1)
            self.assertIn(".sig", p.stderr)
            Path(f"{iso}.sig").write_bytes(b"not a signature")
            p = subprocess.run(cmd, capture_output=True, text=True, timeout=3)
            self.assertEqual(p.returncode, 1)
            self.assertIn("SHA-256", p.stderr)

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
