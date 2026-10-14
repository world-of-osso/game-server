import importlib.util
from pathlib import Path
import sqlite3
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location('guide', Path(__file__).resolve().parents[1] / 'adventure_guide_catalog.py')


class FollowInstallTest(unittest.TestCase):
    def test_catalog_provenance_uses_the_declared_export_build_and_keeps_world_read_only(self):
        module = importlib.util.module_from_spec(SPEC)
        SPEC.loader.exec_module(module)
        with tempfile.TemporaryDirectory() as directory:
            world = Path(directory) / 'world.sqlite'
            with sqlite3.connect(world) as db:
                db.executescript('CREATE TABLE criteria_tree(ID INTEGER); CREATE TABLE player_condition(ID INTEGER);')
            before = world.read_bytes()
            sources = {table: {'name': name, 'fdid': fdid, 'layout': f'{layout:08X}', 'unreadable': 0, 'records': []}
                       for name, (table, fdid, layout) in module.TABLES.items()}
            sql, report = module.prepare_sql(sources, world, '12.1.5.70391')
            self.assertEqual(world.read_bytes(), before)
            with sqlite3.connect(':memory:') as db:
                db.executescript(sql)
                self.assertEqual(db.execute('SELECT DISTINCT build FROM adventure_guide_source').fetchall(), [('12.1.5.70391',)])
            self.assertEqual(report['build'], '12.1.5.70391')


if __name__ == '__main__':
    unittest.main()
