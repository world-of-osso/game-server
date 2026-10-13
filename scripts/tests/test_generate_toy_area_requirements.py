"""Generate source-required-area data from genuine Retail item/requirement projections."""
import csv
import hashlib
import json
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "generate_toy_area_requirements.py"
REQUIREMENTS = """ID,SpellID,FacingCasterFlags,MinFactionID,MinReputation,RequiredAreasID,RequiredAuraVision,RequiresSpellFocus
3910,61810,0,0,0,1103,0,0
14922,171352,0,0,0,4031,0,0
"""
# Exact AreaGroupMember projections: local69933 IDs/members of source4031.
MEMBERS = """ID,AreaID,AreaGroupID
4660,6662,4031
4661,6719,4031
4662,6720,4031
4663,6721,4031
4664,6722,4031
4665,6723,4031
7751,6755,4031
7752,7462,4031
8670,6682,4031
8671,7025,4031
8672,7663,4031
5056,3576,1103
5057,222,1103
5058,159,1103
5059,186,1103
5060,3665,1103
5061,87,1103
5447,131,1103
5448,362,1103
5449,223,1103
33381,13862,1103
39154,14771,1103
"""


class ToyAreaRequirements(unittest.TestCase):
    def write_inputs(self, directory):
        database = directory / "world.db"
        with sqlite3.connect(database) as conn:
            conn.executescript("""
                CREATE TABLE toy(ItemID INTEGER PRIMARY KEY);
                CREATE TABLE item_x_item_effect(ItemID INTEGER,ItemEffectID INTEGER);
                CREATE TABLE item_effect(ID INTEGER,LegacySlotIndex INTEGER,TriggerType INTEGER,SpellID INTEGER);
                INSERT INTO toy VALUES(188698),(115503);
                INSERT INTO item_x_item_effect VALUES(188698,139819),(115503,21230);
                INSERT INTO item_effect VALUES(139819,0,0,61810),(21230,0,0,171352);
            """)
        requirements = directory / "SpellCastingRequirements.csv"
        members = directory / "AreaGroupMember.csv"
        requirements.write_text(REQUIREMENTS)
        members.write_text(MEMBERS)
        return database, requirements, members

    def generate(self, database, requirements, members, output):
        return subprocess.run([
            sys.executable, str(SCRIPT), "--db", str(database),
            "--casting-requirements", str(requirements),
            "--area-group-members", str(members), "--output", str(output),
        ], capture_output=True, text=True)

    def test_source_cast_group_identity_and_all_members_survive_generation(self):
        with tempfile.TemporaryDirectory() as name:
            directory = Path(name)
            database, requirements, members = self.write_inputs(directory)
            output = directory / "requirements.json"
            before = database.read_bytes()
            result = self.generate(database, requirements, members, output)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(database.read_bytes(), before, "source database must remain unchanged")
            self.assertEqual(json.loads(output.read_text()), {
                "build": "12.1.0.69933",
                "sources": {
                    "SpellCastingRequirements": hashlib.sha256(requirements.read_bytes()).hexdigest(),
                    "AreaGroupMember": hashlib.sha256(members.read_bytes()).hexdigest(),
                },
                "spells": {"61810": 1103, "171352": 4031},
                "groups": {
                    "1103": [87, 131, 159, 186, 222, 223, 362, 3576, 3665, 13862, 14771],
                    "4031": [6662, 6682, 6719, 6720, 6721, 6722, 6723, 6755, 7025, 7462, 7663],
                },
            })

    def test_missing_source_group_refuses_without_replacing_existing_manifest(self):
        with tempfile.TemporaryDirectory() as name:
            directory = Path(name)
            database, requirements, members = self.write_inputs(directory)
            # Withhold genuine4031 records; do not invent substitute area IDs.
            rows = [row for row in csv.DictReader(MEMBERS.splitlines()) if row["AreaGroupID"] != "4031"]
            with members.open("w", newline="") as handle:
                writer = csv.DictWriter(handle, fieldnames=["ID", "AreaID", "AreaGroupID"])
                writer.writeheader()
                writer.writerows(rows)
            output = directory / "requirements.json"
            output.write_text("previous source manifest\n")
            result = self.generate(database, requirements, members, output)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("area group4031 has no source members", result.stderr)
            self.assertEqual(output.read_text(), "previous source manifest\n")


if __name__ == "__main__":
    unittest.main()
