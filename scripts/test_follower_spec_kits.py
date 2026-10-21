"""Copy-only follower-kit producer behavioral tests against native 70448 inputs."""
import hashlib
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import unittest
from types import SimpleNamespace

from scripts.follower_spec_kits import prepare, read_periodic_description_child

ROOT = Path(__file__).resolve().parents[1]
DB2 = Path('/home/osso/Projects/world-of-osso/game-engine/data/db2/active')
SOURCE = Path(os.environ['FOLLOWER_KIT_SOURCE_OUTPUT'])


class DescriptionReferenceTests(unittest.TestCase):
    def setUp(self):
        self.db = sqlite3.connect(':memory:', uri=True)
        self.addCleanup(self.db.close)
        native = (ROOT / 'data/world.db').resolve().as_uri() + '?mode=ro'
        self.db.execute('ATTACH DATABASE ? AS native', (native,))
        self.db.execute('CREATE TABLE spell AS SELECT ID,Description_lang FROM native.spell WHERE ID=8921')
        self.db.execute('CREATE TABLE spell_name AS SELECT ID,Name_lang FROM native.spell_name WHERE ID IN(8921,164812,155625)')
        self.db.execute('CREATE TABLE spell_effect AS SELECT ID,SpellID,DifficultyID,Effect,EffectAura,EffectTriggerSpell FROM native.spell_effect WHERE SpellID IN(8921,164812,155625)')

    def test_native_reference_selects_periodic_child(self):
        link = read_periodic_description_child(self.db, 8921, 'Moonfire')
        self.assertEqual(link['child_spell_id'], 164812)
        self.assertEqual(link['fact'], 'ROOT_CHILD_LINK_DESCRIPTION_ONLY')

    def test_missing_reference_fails_explicitly(self):
        self.db.execute("UPDATE spell SET Description_lang=''")
        with self.assertRaisesRegex(ValueError, 'found 0'):
            read_periodic_description_child(self.db, 8921, 'Moonfire')

    def test_absent_native_child_fails_explicitly(self):
        self.db.execute('DELETE FROM spell_name WHERE ID=164812')
        with self.assertRaisesRegex(ValueError, 'found 0'):
            read_periodic_description_child(self.db, 8921, 'Moonfire')

    def test_multiple_native_same_name_periodic_children_fail_explicitly(self):
        self.db.execute("UPDATE spell SET Description_lang='$164812d $155625d'")
        with self.assertRaisesRegex(ValueError, 'found 2'):
            read_periodic_description_child(self.db, 8921, 'Moonfire')


class FollowerKitProducerTests(unittest.TestCase):
    def command(self, output, receipt, diff, world_db=None, source=None):
        return [sys.executable, str(ROOT / 'scripts/follower_spec_kits.py'),
                '--world-db', str(world_db or ROOT / 'data/world.db'), '--db2-root', str(DB2),
                '--source-json', str(source or SOURCE), '--output-db', str(output),
                '--receipt', str(receipt), '--sqldiff', str(diff)]

    def test_copy_contains_level_gated_garrick_and_crenna_kits(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            output, receipt, diff = [root / name for name in ['world.db', 'receipt.json', 'kit.sql']]
            source = (ROOT / 'data/world.db').resolve()
            with source.open('rb') as stream:
                original_sha = hashlib.file_digest(stream, 'sha256').hexdigest()
            result = subprocess.run(self.command(output, receipt, diff), capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            with sqlite3.connect(output) as db:
                for spec, spells in [(66, [633, 53600, 26573, 85673]), (105, [774, 18562, 102342, 33763])]:
                    rows = db.execute('SELECT spell_id FROM follower_spec_kit WHERE spec_id=? AND min_level<=32 AND max_level>=32', (spec,)).fetchall()
                    self.assertTrue(set(spells).issubset({row[0] for row in rows}), (spec, rows))
                for spec, root_spell, child in [(103, 8921, 164812), (105, 8921, 164812), (105, 93402, 164815)]:
                    conditions = db.execute('SELECT conditions FROM follower_spec_kit WHERE spec_id=? AND spell_id=?', (spec, root_spell)).fetchone()[0]
                    self.assertEqual(json.loads(conditions)['missing_aura'], child)
                self.assertFalse(db.execute('SELECT 1 FROM follower_spec_kit WHERE min_level>max_level').fetchone())
                self.assertEqual(db.execute('PRAGMA integrity_check').fetchone()[0], 'ok')
            report = json.loads(receipt.read_text())
            self.assertEqual(report['build'], '12.1.5.70448')
            self.assertEqual(len(report['chosen_loadouts']), 11 * 90)
            links = report['aura_reference_rows']
            self.assertEqual({(row['spec_id'], row['spell_id'], row['child_spell_id']) for row in links},
                             {(103, 8921, 164812), (105, 8921, 164812), (105, 93402, 164815)})
            for row in links:
                self.assertIn(f"${row['child_spell_id']}", row['description'])
                self.assertEqual(row['fact'], 'ROOT_CHILD_LINK_DESCRIPTION_ONLY')
                self.assertTrue(row['child_periodic_effect_rows'])
                self.assertTrue(all(effect['EffectTriggerSpell'] == 0 for effect in row['root_effect_rows']))
            delta = diff.read_text()
            self.assertEqual(report['changed_tables'], ['follower_spec_kit'] if delta.strip() else [])
            # The input can already contain published core kits. Replay the real
            # SQL delta instead of assuming it must CREATE an absent table.
            with sqlite3.connect(source.as_uri() + '?mode=ro', uri=True) as original, sqlite3.connect(root / 'replayed.db') as replayed:
                original.backup(replayed)
                replayed.executescript(delta)
                with sqlite3.connect(output) as expected:
                    self.assertEqual(replayed.execute('SELECT * FROM follower_spec_kit ORDER BY spec_id,spell_id,min_level').fetchall(), expected.execute('SELECT * FROM follower_spec_kit ORDER BY spec_id,spell_id,min_level').fetchall())
            with source.open('rb') as stream:
                self.assertEqual(hashlib.file_digest(stream, 'sha256').hexdigest(), original_sha)

    def test_rebuilds_existing_kit_only_in_copy(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            first = root / 'first.db'
            result = subprocess.run(self.command(first, root / 'first.json', root / 'first.sql'), capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            source = json.loads(SOURCE.read_text())
            source['world_db'] = str(first)
            source_path = root / 'source.json'
            source_path.write_text(json.dumps(source))
            second = root / 'second.db'
            result = subprocess.run(self.command(second, root / 'second.json', root / 'second.sql', first, source_path), capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            with sqlite3.connect(first) as before, sqlite3.connect(second) as after:
                self.assertEqual(before.execute('SELECT * FROM follower_spec_kit ORDER BY spec_id,spell_id,min_level').fetchall(), after.execute('SELECT * FROM follower_spec_kit ORDER BY spec_id,spell_id,min_level').fetchall())
                self.assertEqual(before.execute('SELECT * FROM follower_profiles ORDER BY entry').fetchall(), after.execute('SELECT * FROM follower_profiles ORDER BY entry').fetchall())
            self.assertEqual((root / 'second.sql').read_text(), '')
            self.assertEqual(json.loads((root / 'second.json').read_text())['changed_tables'], [])

    def test_scoped_delta_composes_with_disjoint_spec_in_both_orders(self):
        directory_root = ROOT / 'target/follower-policy-fixtures'
        directory_root.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=directory_root) as directory:
            root = Path(directory)
            source = (ROOT / 'data/world.db').resolve()
            owned = [66, 73, 103, 266, 269]
            prepare(SimpleNamespace(world_db=source, db2_root=DB2, source_json=SOURCE,
                                    output_db=root / 'owned.db', receipt=root / 'owned.json',
                                    sqldiff=root / 'owned.sql', spec_ids=owned))
            delta = (root / 'owned.sql').read_text()
            # Actual a3 ownership shape; the peer's native262/470411 row must
            # retain its authored refresh predicate across either application order.
            peer = '''UPDATE follower_spec_kit SET priority=84,
                conditions='{"refreshable_aura": 188389, "target": "enemy"}'
                WHERE spec_id=262 AND spell_id=470411 AND min_level=3;'''
            def rows(db):
                return db.execute('SELECT * FROM follower_spec_kit ORDER BY spec_id,spell_id,min_level').fetchall()
            with sqlite3.connect(source.as_uri() + '?mode=ro', uri=True) as native:
                with sqlite3.connect(root / 'owned-first.db') as first, sqlite3.connect(root / 'peer-first.db') as second:
                    native.backup(first)
                    native.backup(second)
                    first.executescript(delta)
                    first.executescript(peer)
                    second.executescript(peer)
                    second.executescript(delta)
                    self.assertEqual(rows(first), rows(second), 'disjoint stable-key deltas must commute')
                    expected = rows(first)
                with sqlite3.connect(root / 'peer-only.db') as peer_only:
                    native.backup(peer_only)
                    peer_only.executescript(peer)
                    self.assertEqual([row for row in expected if row[0] not in owned],
                                     [row for row in rows(peer_only) if row[0] not in owned],
                                     'our delta must not rewrite any other spec')
            with sqlite3.connect(root / 'owned.db') as prepared:
                self.assertEqual([row for row in expected if row[0] in owned],
                                 [row for row in rows(prepared) if row[0] in owned])
            self.assertNotIn('rowid', delta.lower(), 'published SQL contract uses stable native primary keys')
            self.assertEqual(json.loads((root / 'owned.json').read_text())['spec_ids'], owned)

    def test_refuses_source_database_output(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            result = subprocess.run(self.command(ROOT / 'data/world.db', root / 'receipt.json', root / 'kit.sql'), capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('refusing', result.stderr.lower())


if __name__ == '__main__':
    unittest.main()
