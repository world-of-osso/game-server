"""Production Retail content importer retains the authentic Trading Post service flag."""
import os
from pathlib import Path
import sqlite3
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import retail_content
from content_common import Report


class TradingPostNpcTests(unittest.TestCase):
    def test_tawny_source_template_preserves_high_service_flag(self):
        source = Path(os.environ['TRADING_POST_WORLD_COPY']).resolve()
        with sqlite3.connect(':memory:', uri=True) as conn:
            conn.execute('ATTACH DATABASE ? AS source', (f'file:{source}?mode=ro',))
            ddl = conn.execute("SELECT sql FROM source.sqlite_master WHERE name='content_creature_template'").fetchone()[0]
            conn.execute(ddl)
            conn.execute('CREATE TABLE creature_template (entry INTEGER PRIMARY KEY)')
            conn.execute('CREATE TABLE tdb_creature_template AS SELECT * FROM source.tdb_creature_template WHERE entry=185468')
            conn.execute('CREATE TABLE content_tuning_level AS SELECT * FROM source.content_tuning_level')
            conn.execute('CREATE TEMP TABLE tdb_difficulty AS SELECT * FROM source.tdb_creature_template_difficulty WHERE Entry=185468 AND DifficultyID=0')
            conn.create_function('health_expansion', 1, retail_content.health_expansion, deterministic=True)
            retail_content.insert_tdb_templates(conn, 'SELECT entry FROM tdb_creature_template')
            name, flags = conn.execute('SELECT name,npcflag FROM content_creature_template WHERE entry=185468').fetchone()
            self.assertEqual(name, 'Tawny Seabraid')
            self.assertEqual(flags, 2251799813685251)
            self.assertEqual(conn.execute('SELECT guid,map,position_x,position_y,position_z FROM source.tdb_creature WHERE id=185468').fetchone(), (9003513, 0, -8828.16, 837.295, 99.5652))
            ddl = conn.execute("SELECT sql FROM source.sqlite_master WHERE name='content_gossip_menu_option'").fetchone()[0]
            conn.execute(ddl)
            conn.execute('CREATE TABLE tdb_gossip_menu_option AS SELECT * FROM source.tdb_gossip_menu_option WHERE MenuID=30315')
            report = Report()
            retail_content.build_gossip_options(conn, '30315', report)
            self.assertEqual(conn.execute('SELECT OptionText,OptionType,OptionNpcFlag FROM content_gossip_menu_option WHERE MenuID=30315 AND OptionID=0').fetchone(), ("I'd like to see what you have to offer this month.", 47, 2251799813685248))
            self.assertEqual(report.rows, [])


if __name__ == '__main__':
    unittest.main()
