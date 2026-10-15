"""Concrete root/CSV/radius fixtures; generated provenance is supplied by the export apply."""
import json
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "generate_toy_facing_requirements.py"


class ToyFacingRequirements(unittest.TestCase):
    def test_source_export_build_and_selected_flags_survive_generation(self):
        with tempfile.TemporaryDirectory() as name:
            directory = Path(name)
            database = directory / "world.db"
            with sqlite3.connect(database) as connection:
                connection.executescript("""
                    CREATE TABLE toy(ItemID INTEGER PRIMARY KEY);
                    CREATE TABLE item_x_item_effect(ItemID INTEGER,ItemEffectID INTEGER);
                    CREATE TABLE item_effect(ID INTEGER,LegacySlotIndex INTEGER,TriggerType INTEGER,SpellID INTEGER);
                    INSERT INTO toy VALUES(141300),(204818);
                    INSERT INTO item_x_item_effect VALUES(141300,1),(204818,2);
                    INSERT INTO item_effect VALUES(1,0,0,225826),(2,0,0,406870);
                """)
            requirements = directory / "SpellCastingRequirements.csv"
            requirements.write_text("SpellID,FacingCasterFlags\n225826,1\n406870,1\n999999,1\n")
            dump = directory / "model-info.sql"
            dump.write_text("INSERT INTO `creature_model_info` VALUES (42247,6,24,0,70391);\n")
            output = directory / "facing.json"
            before = database.read_bytes()
            for build in ["12.1.5.70391", "12.1.6.71000"]:
                result = subprocess.run([
                    sys.executable, str(SCRIPT), "--database", str(database),
                    "--requirements", str(requirements), "--dump", str(dump),
                    "--output", str(output), "--build", build,
                ], capture_output=True, text=True)
                self.assertEqual(result.returncode, 0, result.stderr)
                data = json.loads(output.read_text())
                self.assertEqual(data["build"], build)
                self.assertEqual(data["spells"], {"225826": 1, "406870": 1})
                self.assertEqual(data["model_radii"], {"42247": 6.0})
                self.assertEqual(database.read_bytes(), before)


if __name__ == "__main__":
    unittest.main()
