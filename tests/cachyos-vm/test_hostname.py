"""Фикстуры приёмки имени узла. Не читают окружение пользователя и не публикуют дерево окна."""
from unittest import TestCase, mock
from checks import Failure, verify_cli_hostname, verify_hostname_card
from fs_witness import parse_hostname


class HostnameWitness(TestCase):
    def test_parse_keeps_name_verbatim(self):
        self.assertEqual(parse_hostname(b"cachyos\n"), "cachyos")
        self.assertEqual(parse_hostname(b"e2b.local"), "e2b.local")
        self.assertEqual(parse_hostname(b"a b c d\n"), "a b c d")

    def test_parse_rejects_malformed_tables(self):
        for bad in [b"", b"\n", b" x\n", b"x \n", b"x" * 65 + b"\n",
                    b"a\x01b\n", "тест\n".encode(), b"a\nb\n", b"x" * 65537]:
            with self.subTest(bad=bad), self.assertRaises(ValueError):
                parse_hostname(bad)


class HostnameCard(TestCase):
    def test_cli_name_is_bracketed_by_witness(self):
        s = dict(name="cachyos", source="/proc/sys/kernel/hostname", observed_at_utc="1970-01-01T00:00:11.000Z")
        verify_cli_hostname(s, 10_000_000_000, "cachyos", "cachyos")
        for bad in [None, s | {"name": "other"}, s | {"name": ""}, s | {"name": " x"},
                    s | {"name": "a\x01b"}, s | {"source": "gethostname"},
                    s | {"observed_at_utc": "1970-01-01T00:00:09.999Z"},
                    s | {"name": "x" * 65}]:
            with self.subTest(bad=bad), self.assertRaises(Failure):
                verify_cli_hostname(bad, 10_000_000_000, "cachyos", "cachyos")

    def test_cli_name_may_differ_within_observed_window(self):
        # Имя узла могут сменить в работе: любое из наблюдавшихся значений допустимо.
        s = dict(name="new-name", source="/proc/sys/kernel/hostname", observed_at_utc="1970-01-01T00:00:11.000Z")
        verify_cli_hostname(s, 10_000_000_000, "a-name", "z-name")
        with self.assertRaises(Failure):
            verify_cli_hostname(s, 10_000_000_000, "b-name", "c-name")

    @mock.patch("checks.time.time", return_value=12)
    def test_card_name_must_come_from_this_launch(self, _):
        label = "Hostname cachyos /proc/sys/kernel/hostname 1970-01-01 00:00:11 UTC"
        verify_hostname_card(label, 10, "cachyos", "cachyos")
        for bad in ["Hostname — Нет достоверного замера: /proc/sys/kernel/hostname",
                    "Hostname other /proc/sys/kernel/hostname 1970-01-01 00:00:11 UTC",
                    label.replace("/proc/sys/kernel/hostname", "gethostname"),
                    label.replace("cachyos /proc/sys/kernel/hostname", "cachyos /proc/sys/kernel/hostname extra"),
                    label.replace(":11 UTC", ":09 UTC")]:
            with self.subTest(bad=bad), self.assertRaises(Failure):
                verify_hostname_card(bad, 10, "cachyos", "cachyos")


if __name__ == "__main__":
    import unittest
    unittest.main()
