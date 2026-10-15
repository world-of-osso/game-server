import importlib.util
import struct
import unittest
from pathlib import Path

MODULE = Path(__file__).resolve().parents[1] / 'import_achievement_node_counts.py'
SPEC = importlib.util.spec_from_file_location('node_counts', MODULE)
node_counts = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(node_counts)

# Installed70391 WorldStateExpression8656: occupied Alliance1779==3 OR Horde1778==3.
FULL_COVERAGE = '0102F306000000010103000000000202F2060000000101030000000000'
FIVE_BASES = '0102F306000000010105000000000202F2060000000101050000000000'


class NodeCountExpressions(unittest.TestCase):
    def test_authored_full_coverage_and_five_base_expressions_decode_the_actual_threshold(self):
        self.assertEqual(node_counts.decode_node_count_expression(FULL_COVERAGE), 3)
        self.assertEqual(node_counts.decode_node_count_expression(FIVE_BASES), 5)
        reversed_teams = bytearray.fromhex(FULL_COVERAGE)
        struct.pack_into('<i', reversed_teams, 2, 1778)
        struct.pack_into('<i', reversed_teams, 16, 1779)
        self.assertEqual(node_counts.decode_node_count_expression(reversed_teams.hex()), 3)

    def test_unknown_operator_fields_trailing_bytes_and_nonpositive_counts_do_not_qualify(self):
        for offset, value in [(0, 0), (1, 3), (6, 1), (7, 2), (8, 2),
                              (13, 1), (14, 1), (28, 2)]:
            with self.subTest(offset=offset, value=value):
                changed = bytearray.fromhex(FULL_COVERAGE)
                changed[offset] = value
                self.assertIsNone(node_counts.decode_node_count_expression(changed.hex()))
        for changed in ('', FULL_COVERAGE[:-2], FULL_COVERAGE + '00'):
            self.assertIsNone(node_counts.decode_node_count_expression(changed))
        for field in (1779, 999999):
            changed = bytearray.fromhex(FULL_COVERAGE)
            struct.pack_into('<i', changed, 16, field)
            self.assertIsNone(node_counts.decode_node_count_expression(changed.hex()))
        for count in (0, 6):
            changed = bytearray.fromhex(FULL_COVERAGE)
            struct.pack_into('<i', changed, 9, count)
            struct.pack_into('<i', changed, 23, count)
            self.assertIsNone(node_counts.decode_node_count_expression(changed.hex()))
        asymmetric = bytearray.fromhex(FULL_COVERAGE)
        struct.pack_into('<i', asymmetric, 23, 2)
        self.assertIsNone(node_counts.decode_node_count_expression(asymmetric.hex()))

    def test_only_complete_known_expressions_are_returned_and_malformed_hex_fails_loudly(self):
        expressions = {8656: {'Expression': FULL_COVERAGE}, 5717: {'Expression': FIVE_BASES},
                       1: {'Expression': '000000'}, 2: {'Expression': FULL_COVERAGE + '01'}}
        self.assertEqual(node_counts.qualified_node_counts(expressions), {8656: 3, 5717: 5})
        with self.assertRaises(ValueError):
            node_counts.decode_node_count_expression('not-hex')


if __name__ == '__main__':
    unittest.main()
