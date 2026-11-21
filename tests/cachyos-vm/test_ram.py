"""Правила приёмки. Фикстуры не заменяют наблюдение внутри настоящего гостя."""
from unittest import TestCase, mock
from checks import Failure, verify_ram_card, verify_cli_ram


@mock.patch("checks.proc_total", return_value=4 * 1024**3)
class Ram(TestCase):
    @mock.patch("checks.time.time", return_value=12)
    def test_card_must_report_observed_values_source_unit_and_time(self, *_):
        label = "Занято RAM 1,12 ГиБ из 4 ГиБ /proc/meminfo 1970-01-01 00:00:11 UTC"
        verify_ram_card(label, 10)
        for wrong in ["нет данных", label.replace("1,12", "5"), label.replace("4 ГиБ", "4 ГБ"),
                      label.replace("4 ГиБ", "3 ГиБ"), label.replace("/proc/meminfo", "—"),
                      label.replace(":11 UTC", ":09 UTC"), label.replace(":11 UTC", ":13 UTC")]:
            with self.subTest(wrong=wrong), self.assertRaises(Failure):
                verify_ram_card(wrong, 10)

    @mock.patch("checks.time.time_ns", return_value=12_000_000_000)
    def test_cli_wire_contract_is_fresh_and_exact(self, *_):
        ram = dict(used_bytes=0, total_bytes=4 * 1024**3, source="/proc/meminfo",
                   observed_at_utc="1970-01-01T00:00:11.000Z")
        verify_cli_ram(ram, 10_000_000_000)
        for bad in [None, {}, ram | {"used_bytes": False}, ram | {"total_bytes": 4 * 1024**2},
                    ram | {"observed_at_utc": "1970-01-01T00:00:09.999Z"}]:
            with self.subTest(bad=bad), self.assertRaises(Failure):
                verify_cli_ram(bad, 10_000_000_000)
