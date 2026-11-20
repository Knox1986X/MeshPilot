"""Инварианты схемы проекта; запуск из корня, Python >= 3.11."""
import json
import re
import tomllib
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DESKTOP = ROOT / "apps/desktop"


class Project(unittest.TestCase):
    def test_all_backlog_ids_have_verification(self):
        rows = [line.strip("|").split("|") for line in (ROOT / "docs/ROADMAP.md").read_text().splitlines()
                if re.match(r"^\| (\d{2,3}) \|", line)]
        self.assertEqual([r[0].strip() for r in rows], [f"{n:02}" for n in range(1, 101)])
        self.assertTrue(all(len(r) == 6 and all(c.strip() for c in r) for r in rows))

    def test_crate_identity_and_desktop_version(self):
        workspace = tomllib.loads((ROOT / "Cargo.toml").read_text())["workspace"]
        for member in workspace["members"]:
            name = tomllib.loads((ROOT / member / "Cargo.toml").read_text())["package"]["name"]
            self.assertEqual(name, Path(member).name)
            self.assertTrue(name.startswith("meshpilot-"))
        version = workspace["package"]["version"]
        conf = json.loads((DESKTOP / "src-tauri/tauri.conf.json").read_text())
        self.assertEqual(conf["identifier"], "io.github.Knox1986X.MeshPilot")
        self.assertEqual(conf["version"], version)
        self.assertEqual(json.loads((DESKTOP / "package.json").read_text())["version"], version)
        self.assertEqual(tomllib.loads((DESKTOP / "src-tauri/Cargo.toml").read_text())["package"]["version"], version)

    def test_window_capabilities_are_local_and_narrow(self):
        caps = list((DESKTOP / "src-tauri/capabilities").glob("*.json"))
        self.assertEqual(len(caps), 1)
        cap = json.loads(caps[0].read_text())
        self.assertNotIn("remote", cap)
        self.assertEqual(cap["windows"], ["main"])
        self.assertEqual(set(cap["permissions"]), {"core:window:allow-" + op for op in
                         ("minimize", "toggle-maximize", "close", "start-dragging")} | {"allow-read-ram"})
        permission, = tomllib.loads((DESKTOP / "src-tauri/permissions/ram.toml").read_text())["permission"]
        self.assertEqual(permission["identifier"], "allow-read-ram")
        self.assertEqual(permission["commands"], {"allow": ["read_ram"]})


if __name__ == "__main__":
    unittest.main()
