"""Selected-TC bonus provenance and deploy artifact delivery, without a real host."""
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import unittest
from contextlib import closing

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
import retail_content


class QuestRewardBonusTest(unittest.TestCase):
    def test_selected_retail_rows_preserve_zero_and_do_not_borrow_legacy_same_id(self):
        # Native TDB62/35/54083 values;7 remains a legacy content row, not selected TC.
        with closing(sqlite3.connect(":memory:")) as db, tempfile.TemporaryDirectory() as directory:
            db.executescript("""
                CREATE TABLE content_quest_template(ID INTEGER PRIMARY KEY,RewardBonusMoney INTEGER);
                INSERT INTO content_quest_template VALUES(62,120),(35,0),(54083,46200),(7,NULL);
            """)
            output = Path(directory) / "quest-reward-bonus.json"
            retail_content.write_quest_reward_bonus(db, output)
            self.assertEqual(json.loads(output.read_text()), [
                {"quest_id": 35, "bonus_money": 0},
                {"quest_id": 62, "bonus_money": 120},
                {"quest_id": 54083, "bonus_money": 46200},
            ])

    def test_native_selected_template_mapping_retains_bonus_and_legacy_provenance(self):
        from content_common import Report
        source = (ROOT / "data/world.db").resolve()
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "world.copy.db"
            with closing(sqlite3.connect(source.as_uri() + "?mode=ro", uri=True)) as original:
                with closing(sqlite3.connect(path)) as db:
                    original.backup(db)
            with closing(sqlite3.connect(path)) as db:
                # The maintained full rebuild starts from the AC-shaped table.
                for column in ["ContentTuningID", "RewardMoneyMultiplier", "RewardBonusMoney"]:
                    columns = {row[1] for row in db.execute("PRAGMA table_info(content_quest_template)")}
                    if column in columns:
                        db.execute(f"ALTER TABLE content_quest_template DROP COLUMN {column}")
                offset = retail_content.RETAIL_GUID_OFFSET
                db.execute(f"""CREATE TEMP TABLE retail_entry AS SELECT DISTINCT c.id1 entry
                    FROM content_creature c JOIN tdb_creature t
                    ON c.guid=t.guid+{offset} AND c.id1=t.id""")
                db.execute(f"""CREATE TEMP TABLE retail_gameobject AS SELECT DISTINCT c.id
                    FROM content_gameobject c JOIN tdb_gameobject t
                    ON c.guid=t.guid+{offset} AND c.id=t.id""")
                self.assertEqual(db.execute(
                    "SELECT RewardBonusMoney FROM tdb_quest_template WHERE ID=55"
                ).fetchone(), (570,))
                retail_content.build_quests(db, Report())
                db.commit()
                output = Path(directory) / "quest-reward-bonus.json"
                retail_content.write_quest_reward_bonus(db, output)
                bonuses = {row["quest_id"]: row["bonus_money"]
                           for row in json.loads(output.read_text())}
                self.assertEqual(bonuses[62], 120)
                self.assertEqual(bonuses[35], 0)
                self.assertNotIn(1, bonuses)
                self.assertNotIn(55, bonuses)
                self.assertEqual(db.execute(
                    "SELECT RewardMoney,RewardBonusMoney FROM content_quest_template WHERE ID=62"
                ).fetchone(), (105000, 120))

    def test_deploy_ships_required_bonus_bytes_beside_world_database(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            work = root / "server"
            work.mkdir()
            files = [
                "target/release/game-server", "target/release/game-server-admin",
                "data/world.db", "data/game_object_locks.json", "data/economy/config.json",
                "data/toy_required_areas.json", "data/toy_facing_requirements.json",
                "content/trading-post-monthly-catalog.json",
            ]
            for name in files:
                path = work / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("{}")
            expected = b'[{"quest_id":62,"bonus_money":120}]\n'
            (work / "content/quest-reward-bonus.json").write_bytes(expected)
            (work / "data/db2/70448").mkdir(parents=True)
            (work / "data/db2/active").symlink_to("70448")
            (work / "data/ground/navmesh").mkdir(parents=True)
            (root / "game-engine/data/los/34").mkdir(parents=True)
            (root / "game-engine/data/los/eye_heights.tsv").write_text("fixture")
            commands = root / "commands"
            commands.mkdir()
            sink = root / "received"
            sink.mkdir()
            stub = """#!/usr/bin/python3
from pathlib import Path
import json,os,sys
name=Path(sys.argv[0]).name
with open(os.environ['COMMAND_LOG'],'a') as log:
    log.write(json.dumps([name,*sys.argv[1:]])+'\\n')
if name=='scp':
    source=Path(sys.argv[1])
    destination=sys.argv[2]
    (Path(os.environ['SINK'])/Path(destination).name).write_bytes(source.read_bytes())
"""
            for name in ["python3", "ssh", "scp", "rsync"]:
                path = commands / name
                path.write_text(stub)
                path.chmod(0o755)
            log = root / "commands.jsonl"
            result = subprocess.run(
                [str(ROOT / "deploy.sh")], cwd=work,
                env={**os.environ, "PATH": f"{commands}:{os.environ['PATH']}",
                     "SINK": str(sink), "COMMAND_LOG": str(log)},
                text=True, capture_output=True, timeout=15,
            )
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            received = sink / "quest-reward-bonus.json"
            self.assertTrue(received.is_file(), "deploy omitted required quest bonus catalog")
            self.assertEqual(received.read_bytes(), expected)
            transfers = [json.loads(line) for line in log.read_text().splitlines()]
            self.assertIn(["scp", "content/quest-reward-bonus.json",
                           "sakuin:/var/lib/game-server/data/quest-reward-bonus.json"], transfers)


if __name__ == "__main__":
    unittest.main()
