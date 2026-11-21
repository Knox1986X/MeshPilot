import unittest
from verdict import EXPECTED, verdict


class Verdict(unittest.TestCase):
    def trace(self, phases):
        return ["MESHPILOT/1 current " + s for s in phases]

    def test_complete_trace(self):
        verdict(self.trace(EXPECTED), "current", 0)

    def test_clean_shutdown_is_not_enough(self):
        for phases in ([], ["PASS"], EXPECTED[:-1], ["GUEST", "CLI", "UNCHANGED", "FAIL GUI_RUNTIME libexample.so"], EXPECTED + ["PASS"]):
            with self.assertRaises(ValueError):
                verdict(self.trace(phases), "current", 0)

    def test_display_is_not_the_meshpilot_window(self):
        trace = self.trace(["GUEST", "CLI", "SESSION", "UNCHANGED", "FAIL GUI_RUNTIME libwebkit2gtk-4.1.so.0"])
        with self.assertRaisesRegex(ValueError, "SESSION, UNCHANGED, FAIL GUI_RUNTIME"):
            verdict(trace, "current", 0)
        with self.assertRaises(ValueError):
            verdict(self.trace(["GUEST", "CLI", "SESSION", "UNCHANGED", "PASS"]), "current", 0)

    def test_window_without_ram_card_does_not_pass_the_new_gate(self):
        with self.assertRaises(ValueError):
            verdict(self.trace(["GUEST", "CLI", "SESSION", "WINDOW", "UNCHANGED", "PASS"]), "current", 0)

    def test_ram_without_cpu_does_not_pass(self):
        with self.assertRaises(ValueError):
            verdict(self.trace(["GUEST", "CLI", "SESSION", "WINDOW", "RAM_CARD", "UNCHANGED", "PASS"]), "current", 0)

    def test_replay_or_timeout_is_not_success(self):
        for nonce, rc in [("previous", 0), ("current", 124)]:
            with self.assertRaises(ValueError):
                verdict(self.trace(EXPECTED), nonce, rc)


if __name__ == "__main__":
    unittest.main()
