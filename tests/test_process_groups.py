"""Strict interpretation of complete native process-group membership evidence."""
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
from verification.process_groups import parse_group_members


class GroupEvidence(unittest.TestCase):
    def test_complete_rows_include_kernel_ids_and_all_matching_members(self):
        snapshot = '0 0\n7 7\n43 99\n42 99\n'
        self.assertEqual(parse_group_members(snapshot, 99, 7), (42, 43))
        self.assertEqual(parse_group_members(snapshot, 123, 7), ())

    def test_incomplete_malformed_negative_and_duplicate_rows_are_rejected(self):
        for snapshot in ['', '\n', '42 99\n', '7 7\n42\n', '7 7\n42 99 Z\n',
                         '7 7\n-42 99\n', '7 7\n42 -99\n', '7 7\nx 99\n',
                         '7 7\n42 99\n42 123\n']:
            with self.subTest(snapshot=snapshot):
                with self.assertRaises(ValueError):
                    parse_group_members(snapshot, 99, 7)


if __name__ == '__main__':
    unittest.main()
