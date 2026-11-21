from types import SimpleNamespace
from unittest import TestCase, mock
from checks import Failure, verify_cli_root_fs, verify_root_fs_card
from fs_witness import root_bytes


class RootFs(TestCase):
    def setUp(self):
        self.low = dict(unit=1024, used_bytes=20 * 1024, total_bytes=40 * 1024,
                        available_bytes=14 * 1024, unavailable_free=4 * 1024)
        self.high = self.low | dict(used_bytes=22 * 1024, available_bytes=16 * 1024)

    def test_independent_probe_uses_root_and_fragment_size(self):
        stats = SimpleNamespace(f_bsize=4096, f_frsize=1024, f_blocks=40, f_bfree=20, f_bavail=16)
        with mock.patch("fs_witness.os.statvfs", return_value=stats) as statvfs:
            self.assertEqual(root_bytes(), self.low | {"available_bytes": 16 * 1024})
            statvfs.assert_called_once_with("/")

    @mock.patch("checks.time.time_ns", return_value=12_000_000_000)
    def test_cli_distinguishes_unused_and_available_and_requires_exact_bytes(self, _):
        s = dict(used_bytes=21 * 1024, available_bytes=15 * 1024, total_bytes=40 * 1024,
                 source="statvfs(/)", observed_at_utc="1970-01-01T00:00:11.000Z")
        args = (10_000_000_000, self.low, self.high)
        verify_cli_root_fs(s, *args)
        for bad in [None, s | {"available_bytes": 19 * 1024}, s | {"available_bytes": 14 * 1024},
                    s | {"available_bytes": 15361}, s | {"used_bytes": False}, s | {"total_bytes": 80 * 1024},
                    s | {"source": "df"}, s | {"observed_at_utc": "1970-01-01T00:00:09.999Z"}]:
            with self.subTest(bad=bad), self.assertRaises(Failure):
                verify_cli_root_fs(bad, *args)

    @mock.patch("checks.time.time", return_value=12)
    def test_card_requires_all_three_quantities_from_this_launch(self, _):
        label = "Занято на / 21 КиБ из 40 КиБ Доступно пользователю: 15 КиБ statvfs(/) 1970-01-01 00:00:11 UTC"
        verify_root_fs_card(label, 10, self.low, self.high)
        for bad in ["нет данных", label.replace("21 КиБ", "5 КиБ"), label.replace("40 КиБ", "40 Б"),
                    label.replace("15 КиБ", "19 КиБ"), label.replace("15 КиБ", "0 ПиБ"), label.replace("Доступно пользователю: 15 КиБ ", ""),
                    label.replace("statvfs(/)", "statvfs(/home)"), label.replace(":11 UTC", ":09 UTC")]:
            with self.subTest(bad=bad), self.assertRaises(Failure):
                verify_root_fs_card(bad, 10, self.low, self.high)
