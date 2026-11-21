from unittest import TestCase, mock
from checks import Failure
from net_checks import verify_cli_net, verify_loopback_card, verify_shared_window


class Net(TestCase):
    @mock.patch("checks.time.time_ns", return_value=12_000_000_000)
    def test_cli_per_direction_bounds_interval_utc_and_devices(self, _):
        rates = dict(rx_delta_bytes=4, tx_delta_bytes=10, rx_bytes_per_sec=6.4, tx_bytes_per_sec=16)
        lo = dict(name="lo", rates=rates)
        s = dict(interfaces=[lo], source="/proc/net/dev", interval_ms=625, observed_at_utc="1970-01-01T00:00:11.000Z")
        args = (10_000_000_000, 700, {"lo":(100,200)}, {"lo":(105,210)})
        verify_cli_net(s, *args)
        for change in [dict(source="eth0"), dict(observed_at_utc="1970-01-01T00:00:09.000Z"), dict(interval_ms=701),
                       dict(interfaces=[]), dict(interfaces=[lo, lo]), dict(interfaces=[lo | {"name":"eth0"}]),
                       dict(interfaces=[lo | {"rates":None}]), dict(interfaces=[lo | {"rates":rates | {"tx_delta_bytes":11}}]),
                       dict(interfaces=[lo | {"rates":rates | {"rx_bytes_per_sec":4}}])]:
            with self.subTest(change=change), self.assertRaises(Failure):
                verify_cli_net(s | change, *args)

    def test_status_intervals_must_overlap_not_merely_finish_in_time(self):
        cpu = dict(observed_at_utc="1970-01-01T00:00:11.000Z", interval_ms=501)
        verify_shared_window(cpu, cpu | {"observed_at_utc": "1970-01-01T00:00:11.005Z"})
        with self.assertRaises(Failure):
            verify_shared_window(cpu, cpu | {"observed_at_utc": "1970-01-01T00:00:11.500Z"})

    @mock.patch("net_checks.time.time", return_value=12)
    def test_zero_is_a_loopback_measurement_not_no_nic_or_no_data(self, _):
        label = "Трафик сети lo только локальная петля Приём 0 Б/с Передача 1,6 Б/с за 625 мс /proc/net/dev 1970-01-01 00:00:11 UTC"
        args = (10,700,{"lo":(100,200)},{"lo":(100,201)})
        verify_loopback_card(label, *args)
        for bad in ["нет данных", label.replace("только локальная петля ", ""), label.replace("lo", "eth0"), label.replace("0 Б/с", "1 Б/с"),
                    label.replace("1,6 Б/с", "1,6 КиБ/с"), label.replace("625", "450"), label.replace("/proc/net/dev", "нет данных"),
                    label.replace("00:00:11", "00:00:09")]:
            with self.subTest(bad=bad), self.assertRaises(Failure):
                verify_loopback_card(bad, *args)
