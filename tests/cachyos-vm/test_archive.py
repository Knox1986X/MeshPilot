import unittest
from archive import LIMIT, NoRange, ranges, size_from_head, verify_range


class Archive(unittest.TestCase):
    def test_coverage_including_remainder(self):
        for total in (1024**2, 123456789, LIMIT):
            spans = ranges(total)
            self.assertEqual(spans[0][0], 0)
            self.assertEqual(spans[-1][1], total-1)
            self.assertEqual(len(spans), 4)
            self.assertTrue(all(a[1]+1 == b[0] for a, b in zip(spans, spans[1:])))
        for total in (1, LIMIT+1):
            with self.assertRaises(ValueError):
                ranges(total)

    def test_head_is_only_a_size_hint(self):
        self.assertEqual(size_from_head("200 123456789"), 123456789)
        for meta in ("302 4096", "200", "200 1 junk", "200 no-content-length"):
            with self.assertRaises(NoRange):
                size_from_head(meta)
        for meta in ("200 8192", f"200 {LIMIT+1}"):
            with self.assertRaises(ValueError):
                size_from_head(meta)

    def test_only_exact_partial_response(self):
        self.assertEqual(verify_range("206 bytes 0-0/1234567", 1, 0, 0, None), 1234567)
        good = "206 bytes 101-110/1234567"
        self.assertEqual(verify_range(good, 10, 101, 110, 1234567), 1234567)
        for reply, size in ((good, 9), (good.replace("206", "200"), 10), (good.replace("110/", "111/"), 10),
                            (good.replace("1234567", "1234568"), 10), (good+" extra", 10)):
            with self.assertRaises(ValueError):
                verify_range(reply, size, 101, 110, 1234567)


if __name__ == "__main__":
    unittest.main()
