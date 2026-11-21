"""Фикстуры приёмки времени работы. Не читают procfs пользователя и не публикуют дерево окна."""
from unittest import TestCase, mock
from checks import Failure, verify_cli_uptime, verify_uptime_card, parse_uptime_value
from fs_witness import parse_uptime


class UptimeWitness(TestCase):
    def test_parse_keeps_integer_part_of_first_field(self):
        self.assertEqual(parse_uptime(b"11931.90 22378.30\n"), 11931)
        self.assertEqual(parse_uptime(b"0.50 0.10"), 0)
        self.assertEqual(parse_uptime(b"7 9"), 7)

    def test_parse_rejects_malformed_tables(self):
        for bad in [b"", b"12.5", b"12.5 8.0 extra", b" 12 13", b"12 13 ", b".5 1.0",
                    b"5. 1.0", b"1.2.3 4.0", b"1e2 3.0", b"-1 2.0", b"99999999999999999999999 1"]:
            with self.subTest(bad=bad), self.assertRaises(ValueError):
                parse_uptime(bad)


class UptimeCard(TestCase):
    def test_displayed_value_is_pairs_of_number_and_unit(self):
        self.assertEqual(parse_uptime_value("0 с"), 0)
        self.assertEqual(parse_uptime_value("3 ч 18 м 51 с"), 11931)
        self.assertEqual(parse_uptime_value("1 д 0 ч 0 м 0 с"), 86400)
        for bad in ["0", "3", "3 ч", "ч 3", "3 х 5 мин", "11931", "3 ч 18"]:
            with self.subTest(bad=bad), self.assertRaises(Failure):
                parse_uptime_value(bad)

    @mock.patch("checks.time.time_ns", return_value=12_000_000_000)
    def test_cli_seconds_are_bracketed_by_witness(self, _):
        s = dict(uptime_seconds=11931, source="/proc/uptime", observed_at_utc="1970-01-01T00:00:11.000Z")
        verify_cli_uptime(s, 10_000_000_000, 11930, 11931)
        for bad in [None, s | {"uptime_seconds": 11932}, s | {"uptime_seconds": 11929},
                    s | {"uptime_seconds": False}, s | {"source": "/proc/stat"},
                    s | {"observed_at_utc": "1970-01-01T00:00:09.999Z"}]:
            with self.subTest(bad=bad), self.assertRaises(Failure):
                verify_cli_uptime(bad, 10_000_000_000, 11930, 11931)

    @mock.patch("checks.time.time", return_value=12)
    def test_card_seconds_must_come_from_this_launch(self, _):
        label = "Время работы 3 ч 18 м 51 с /proc/uptime 1970-01-01 00:00:11 UTC"
        verify_uptime_card(label, 10, 11930, 11931)
        for bad in ["Время работы — Нет достоверного замера: /proc/uptime",
                    "Время работы 3 ч 18 м 52 с /proc/uptime 1970-01-01 00:00:11 UTC",
                    "Время работы 3 ч 18 м 49 с /proc/uptime 1970-01-01 00:00:11 UTC",
                    label.replace("/proc/uptime", "/proc/stat"),
                    label.replace(":11 UTC", ":09 UTC")]:
            with self.subTest(bad=bad), self.assertRaises(Failure):
                verify_uptime_card(bad, 10, 11930, 11931)


if __name__ == "__main__":
    import unittest
    unittest.main()
