"""Фикстуры приёмки версии системы. Не читают os-release пользователя и не публикуют дерево окна."""
from unittest import TestCase, mock
from checks import Failure, verify_cli_os_version, verify_os_version_card
from fs_witness import parse_os_version


FILE = b"NAME=\"Debian GNU/Linux\"\nVERSION_ID=\"12\"\nVERSION=\"12 (bookworm)\"\nID=debian\n"


class OsVersionWitness(TestCase):
    def test_parse_keeps_value_verbatim(self):
        self.assertEqual(parse_os_version(FILE), "12 (bookworm)")
        self.assertEqual(parse_os_version(b"ID=arch\nVERSION=26.11.20261108\n"), "26.11.20261108")
        self.assertEqual(parse_os_version(b"VERSION='CachyOS (26.11.20261108)'\n"), "CachyOS (26.11.20261108)")
        # Конец файла без перевода строки допустим.
        self.assertEqual(parse_os_version(b"VERSION=12 (bookworm)"), "12 (bookworm)")

    def test_parse_rejects_malformed_tables(self):
        for bad in [b"", b"VERSION_ID=12\n", b"VERSION=\n", b"VERSION=\"\"\n",
                    b"VERSION=\"\n", b"VERSION=a\"\n", b"VERSION=\"a\" \"b\"\n",
                    b"VERSION= 12\n", b"VERSION=12 \n", b"VERSION=12\nVERSION=13\n",
                    b"VERSION=\"12\t(bookworm)\"\n", b"VERSION=\"12\x01(bookworm)\"\n",
                    "VERSION=Тест\n".encode(),
                    b"VERSION=\"" + b"1" * 65 + b"\"\n",
                    b"VERSION=12\n" + b"x" * 65536]:
            with self.subTest(bad=bad), self.assertRaises(ValueError):
                parse_os_version(bad)


class OsVersionCard(TestCase):
    def test_cli_version_is_bracketed_by_witness(self):
        s = dict(version="12 (bookworm)", source="/etc/os-release", observed_at_utc="1970-01-01T00:00:11.000Z")
        verify_cli_os_version(s, 10_000_000_000, "12 (bookworm)", "12 (bookworm)")
        for bad in [None, s | {"version": "13 (trixie)"}, s | {"version": ""},
                    s | {"version": " 12 (bookworm)"}, s | {"version": "12 (bookworm) "},
                    s | {"version": '12 (book"worm)'}, s | {"version": "12\x01(bookworm)"},
                    s | {"source": "/proc/version"},
                    s | {"observed_at_utc": "1970-01-01T00:00:09.999Z"},
                    s | {"version": "1" * 65}]:
            with self.subTest(bad=bad), self.assertRaises(Failure):
                verify_cli_os_version(bad, 10_000_000_000, "12 (bookworm)", "12 (bookworm)")

    @mock.patch("checks.time.time", return_value=12)
    def test_card_version_must_come_from_this_launch(self, _):
        label = "Версия системы 12 (bookworm) /etc/os-release 1970-01-01 00:00:11 UTC"
        verify_os_version_card(label, 10, "12 (bookworm)", "12 (bookworm)")
        for bad in ["Версия системы — Нет достоверного замера: /etc/os-release",
                    "Версия системы 13 (trixie) /etc/os-release 1970-01-01 00:00:11 UTC",
                    label.replace("/etc/os-release", "/etc/os-release.d/custom"),
                    label.replace("12 (bookworm)", "12 (bookworm) extra"),
                    label.replace(":11 UTC", ":09 UTC")]:
            with self.subTest(bad=bad), self.assertRaises(Failure):
                verify_os_version_card(bad, 10, "12 (bookworm)", "12 (bookworm)")


if __name__ == "__main__":
    import unittest
    unittest.main()
