"""Фикстуры приёмки версии ядра. Не читают procfs пользователя и не публикуют дерево окна."""
from unittest import TestCase, mock
from checks import Failure, verify_cli_kernel, verify_kernel_card
from fs_witness import parse_kernel


LINE = b"Linux version 6.12.9-cachyos1-1 (build@cachyos) (gcc) #1 SMP PREEMPT_DYNAMIC Mon Nov 17 10:00:00 UTC 2026"


class KernelWitness(TestCase):
    def test_parse_keeps_second_word_verbatim(self):
        self.assertEqual(parse_kernel(LINE), "6.12.9-cachyos1-1")
        self.assertEqual(parse_kernel(LINE + b"\n"), "6.12.9-cachyos1-1")
        self.assertEqual(parse_kernel(b"Linux version 6.1.158+ (root@runner) (gcc) #1 SMP"), "6.1.158+")

    def test_parse_rejects_malformed_lines(self):
        for bad in [b"", b"\n", b"Linux 6.12 (a) (b) #1", b"Linux version",
                    b"Linux version\n6.12", b"linux version 6.12 (a) (b) #1",
                    b"Linux version 6.1\x012 (a) (b) #1",
                    b"Linux version " + b"1" * 65 + b" (a) (b) #1"]:
            with self.subTest(bad=bad), self.assertRaises(ValueError):
                parse_kernel(bad)


class KernelCard(TestCase):
    def test_cli_version_is_bracketed_by_witness(self):
        s = dict(version="6.12.9-cachyos1-1", source="/proc/version", observed_at_utc="1970-01-01T00:00:11.000Z")
        verify_cli_kernel(s, 10_000_000_000, "6.12.9-cachyos1-1", "6.12.9-cachyos1-1")
        for bad in [None, s | {"version": "6.12.9-other"}, s | {"version": ""},
                    s | {"version": "6.1\x012"}, s | {"source": "/proc/stat"},
                    s | {"observed_at_utc": "1970-01-01T00:00:09.999Z"},
                    s | {"version": "1" * 65}]:
            with self.subTest(bad=bad), self.assertRaises(Failure):
                verify_cli_kernel(bad, 10_000_000_000, "6.12.9-cachyos1-1", "6.12.9-cachyos1-1")

    @mock.patch("checks.time.time", return_value=12)
    def test_card_version_must_come_from_this_launch(self, _):
        label = "Ядро 6.12.9-cachyos1-1 /proc/version 1970-01-01 00:00:11 UTC"
        verify_kernel_card(label, 10, "6.12.9-cachyos1-1", "6.12.9-cachyos1-1")
        for bad in ["Ядро — Нет достоверного замера: /proc/version",
                    "Ядро 6.12.9-other /proc/version 1970-01-01 00:00:11 UTC",
                    label.replace("/proc/version", "/proc/stat"),
                    label.replace("6.12.9-cachyos1-1 /proc/version", "6.12.9-cachyos1-1 /proc/version extra"),
                    label.replace(":11 UTC", ":09 UTC")]:
            with self.subTest(bad=bad), self.assertRaises(Failure):
                verify_kernel_card(bad, 10, "6.12.9-cachyos1-1", "6.12.9-cachyos1-1")


if __name__ == "__main__":
    import unittest
    unittest.main()
