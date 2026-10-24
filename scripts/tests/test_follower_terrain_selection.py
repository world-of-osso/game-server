"""Anub-only project ground selection against real authored ADT/WMO fixtures."""
import copy
import json
import os
from pathlib import Path
import struct
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from follower_placement_sources import digest, read_db2, read_journal_marker, read_source_floor_support
from follower_placements import read_binding_tables


@unittest.skipUnless('FOLLOWER_PLACEMENT_DB2' in os.environ, 'requires authenticated source assets')
class AuthoredTerrainSelection(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        path = Path(__file__).resolve().parents[2] / 'content/follower-placements/dawnbreaker-anub.json'
        cls.record = json.loads(path.read_text())['entries'][0]
        db2 = Path(os.environ['FOLLOWER_PLACEMENT_DB2'])
        tables, _ = read_db2(db2)
        binding, _ = read_binding_tables(db2)
        _, cls.marker, cls.assignments = read_journal_marker(tables | binding, 2662, 211089, 2581)
        cls.heights = [124.99189880457607, 169.3219764394878, 173.89186446312368, 1104.8230000000003]

    def select(self, record, heights=None):
        return read_source_floor_support(record, self.marker, self.assignments,
                                         self.heights if heights is None else heights)

    def replace_proof_file(self, record, old, new):
        evidence = record['source_floor_evidence']
        proof = next(p for p in evidence['source_files'] if p['path'] == old)
        proof.update(path=str(new), sha256=digest(new))

    def test_authored_nonhole_ground_selects_only_baked_125_floor(self):
        self.assertEqual(self.select(self.record), [124.99189880457607])

    def test_two_baked_candidates_within_one_yard_are_ambiguous(self):
        with self.assertRaisesRegex(ValueError, 'exactly one baked candidate'):
            self.select(self.record, self.heights + [125.3])

    def test_missing_covering_wmo_group_is_not_accepted(self):
        record = copy.deepcopy(self.record)
        record['source_floor_evidence']['covering_wmo_roots'][0]['checked_group_file_ids'].pop()
        with self.assertRaisesRegex(ValueError, 'covering WMO audit incomplete'):
            self.select(record)

    def test_upward_wmo_at_a_rejected_candidate_also_blocks_ground_selection(self):
        record = copy.deepcopy(self.record)
        old = record['source_floor_evidence']['covering_wmo_report']
        report = json.loads(Path(old).read_text())
        hit = next(hit for group in report['groups'] for hit in group['hits'].get('211089', []) if hit['upward'])
        for vertex in hit['world_vertices']:
            vertex[2] = self.heights[1]
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'wmo-report.json'
            path.write_text(json.dumps(report))
            record['source_floor_evidence']['covering_wmo_report'] = str(path)
            self.replace_proof_file(record, old, path)
            with self.assertRaisesRegex(ValueError, 'upward WMO floor near a baked candidate'):
                self.select(record)

    def test_hole_or_duplicate_source_triangles_are_rejected(self):
        for make_hole in (True, False):
            with self.subTest(hole=make_hole), tempfile.TemporaryDirectory() as directory:
                record = copy.deepcopy(self.record)
                evidence = record['source_floor_evidence']
                old = evidence['terrain_file']
                raw = bytearray(Path(old).read_bytes())
                target = evidence['source_terrain_research']['source_terrain_hits'][0]
                offset = 0
                while offset + 8 <= len(raw):
                    size = struct.unpack_from('<I', raw, offset + 4)[0]
                    if raw[offset:offset + 4] == b'KNCM' and list(struct.unpack_from('<3f', raw, offset + 8 + 104)) == target['chunk_origin']:
                        break
                    offset += 8 + size
                self.assertLess(offset + 8, len(raw))
                if make_hole:
                    flags = struct.unpack_from('<I', raw, offset + 8)[0]
                    struct.pack_into('<I', raw, offset + 8, flags | 0x10000)
                    row, col = target['cell']
                    struct.pack_into('<Q', raw, offset + 8 + 20, 1 << (row * 8 + col))
                else:
                    raw.extend(raw[offset:offset + 8 + size])
                path = Path(directory) / 'terrain.adt'
                path.write_bytes(raw)
                evidence['terrain_file'] = str(path)
                self.replace_proof_file(record, old, path)
                with self.assertRaisesRegex(ValueError, 'unique authored non-hole terrain triangle'):
                    self.select(record)

    def test_changed_source_hash_is_rejected(self):
        record = copy.deepcopy(self.record)
        record['source_floor_evidence']['source_files'][0]['sha256'] = '0' * 64
        with self.assertRaisesRegex(ValueError, 'source terrain evidence file changed'):
            self.select(record)


if __name__ == '__main__':
    unittest.main()
