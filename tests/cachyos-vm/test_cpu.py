from unittest import TestCase, mock
from checks import Failure, verify_cli_cpu, verify_cpu_card


class Cpu(TestCase):
    @mock.patch("checks.time.time_ns", return_value=12_000_000_000)
    def test_cli_deltas_fit_outer_sample_and_explain_percent(self, _):
        record = dict(busy_ticks=5, total_ticks=25, usage_percent=20, interval_ms=510,
                      source="/proc/stat", observed_at_utc="1970-01-01T00:00:11.000Z")
        args = (10_000_000_000, 530, (100, 200), (107, 221))
        verify_cli_cpu(record, *args)
        for change in [{"busy_ticks": 8}, {"total_ticks": 30}, {"interval_ms": 650},
                       {"usage_percent": 25}, {"usage_percent": False}, {"source": "/proc/loadavg"},
                       {"observed_at_utc": "1970-01-01T00:00:09.000Z"}]:
            with self.subTest(change=change), self.assertRaises(Failure):
                verify_cli_cpu(record | change, *args)

    @mock.patch("checks.time.time", return_value=12)
    def test_cpu_card_requires_percentage_source_interval_and_utc(self, _):
        label = "Загрузка CPU 17,5 % за 501 мс /proc/stat 1970-01-01 00:00:11 UTC"
        verify_cpu_card(label, 10, 2000)
        for wrong in ["нет данных", label.replace("17,5", "101"), label.replace("501 мс", "1 с"),
                      label.replace("501", "499"), label.replace("/proc/stat", "браузер"), label.replace(":11 UTC", ":13 UTC")]:
            with self.subTest(wrong=wrong), self.assertRaises(Failure):
                verify_cpu_card(wrong, 10, 2000)
