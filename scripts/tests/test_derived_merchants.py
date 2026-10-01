"""Derived placement publication against isolated real world/TDB SQLite schemas."""

import copy
import importlib.util
import json
import sqlite3
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
import retail_content
from content_common import clone_table

if importlib.util.find_spec("derived_merchants"):
    import derived_merchants as adapter
else:
    adapter = None


def merchant():
    return {
        "entry": 243531,
        "name": "Zaralda",
        "minlevel": 80,
        "maxlevel": 90,
        "display_id": 138959,
        "guid": 90000001,
        "map": 0,
        "area_id": 15969,
        "position_x": 8729.1,
        "position_y": -4546.0,
        "position_z": 42.5,
        "orientation": 0,
        "source_url": "https://www.wowhead.com/npc=243531/zaralda",
        "placement_kind": "derived",
        "vendors": [
            {
                "slot": slot,
                "item": item,
                "buy_price": 700,
                "buy_count": 1,
                "maxcount": 0,
                "incrtime": 0,
                "extended_cost": 0,
            }
            for slot, item in enumerate((251665, 251691))
        ],
    }


def fixture():
    conn = sqlite3.connect(":memory:")
    conn.executescript((SCRIPTS / "world_schema.sql").read_text())
    conn.executescript((SCRIPTS / "tdb_schema.sql").read_text())
    for raw, content in (
        ("creature_template", "content_creature_template"),
        ("creature_template_model", "content_creature_template_model"),
        ("creature", "content_creature"),
        ("npc_vendor", "content_npc_vendor"),
    ):
        clone_table(conn, raw, content)
    for column in (
        "ContentTuningID",
        "HealthScalingExpansion",
        "LevelScalingDeltaMin",
        "LevelScalingDeltaMax",
    ):
        conn.execute(
            f"ALTER TABLE content_creature_template ADD COLUMN {column} INTEGER NOT NULL DEFAULT 0"
        )
    conn.execute(
        "ALTER TABLE content_creature ADD COLUMN modelid INTEGER NOT NULL DEFAULT 0"
    )
    conn.execute(
        "ALTER TABLE content_creature ADD COLUMN spawnDifficulties TEXT NOT NULL DEFAULT '0'"
    )
    # DB2 source/projection columns consumed by this bounded adapter.
    conn.executescript("""
        CREATE TABLE item_sparse (ID INTEGER PRIMARY KEY, BuyPrice INTEGER, VendorStackCount INTEGER);
        CREATE TABLE content_item (ID INTEGER PRIMARY KEY, BuyPrice INTEGER, VendorStackCount INTEGER);
        CREATE TABLE creature_display_info (ID INTEGER PRIMARY KEY);
        CREATE TABLE content_tuning_level (ID INTEGER PRIMARY KEY, minlevel INTEGER, maxlevel INTEGER,
            MinLevelWithDelta INTEGER, ScalingFactionGroup INTEGER);
        INSERT INTO creature_display_info VALUES (138959);
        INSERT INTO item_sparse VALUES (251665, 700, 1), (251691, 700, 1);
        INSERT INTO content_item SELECT * FROM item_sparse;
        INSERT INTO tdb_creature_template (entry, name, subname, npcflag, faction, speed_walk,
            BaseAttackTime, unit_class, VerifiedBuild)
            VALUES (243531, 'Zaralda', 'Leatherworking Supplies', 1, 35, 1.25, 2000, 1, 69497);
        INSERT INTO tdb_creature_template_difficulty (Entry, DifficultyID, HealthScalingExpansion,
            ContentTuningID, HealthModifier, ManaModifier, ArmorModifier, DamageModifier,
            LevelScalingDeltaMin, LevelScalingDeltaMax, VerifiedBuild)
            VALUES (243531, 0, 11, 321, 1.5, 2, 3, 4, -2, 2, 69497);
        INSERT INTO tdb_creature_template_model VALUES (243531, 0, 138959, 1, 1, 69497);
        INSERT INTO content_tuning_level VALUES (321, 70, 90, 70, 0);
        CREATE TEMP TABLE tdb_difficulty AS SELECT * FROM tdb_creature_template_difficulty;
        INSERT INTO creature (guid, id1) VALUES (77, 100);
        INSERT INTO tdb_creature (guid, id) VALUES (88, 101);
        INSERT INTO npc_vendor (entry, item) VALUES (100, 251665);
        INSERT INTO content_creature (guid, id1, Comment) VALUES (99, 102, 'unrelated');
        INSERT INTO content_npc_vendor (entry, item) VALUES (102, 251691);
    """)
    conn.create_function(
        "health_expansion", 1, retail_content.health_expansion, deterministic=True
    )
    conn.commit()
    return conn


def snapshot(conn):
    tables = [
        name
        for (name,) in conn.execute(
            "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name"
        )
    ]
    return {
        name: conn.execute(f'SELECT * FROM "{name}" ORDER BY rowid').fetchall()
        for name in tables
    }


class DerivedMerchantTests(unittest.TestCase):
    def setUp(self):
        self.assertIsNotNone(adapter, "derived_merchants module not implemented")
        self.conn = fixture()
        self.addCleanup(self.conn.close)

    def apply(self, records=None):
        with self.conn:
            self.conn.execute("BEGIN")
            return adapter.apply_derived_merchants(
                self.conn, records if records is not None else [merchant()]
            )

    def test_publishes_authentic_template_models_and_local_spawn_vendors(self):
        before = snapshot(self.conn)
        self.assertEqual(self.apply(), 1)
        template = self.conn.execute("""SELECT name, subname, minlevel, maxlevel, npcflag, faction,
            speed_walk, BaseAttackTime, HealthModifier, ManaModifier, ArmorModifier, DamageModifier,
            HealthScalingExpansion, ContentTuningID, LevelScalingDeltaMin, LevelScalingDeltaMax,
            VerifiedBuild FROM content_creature_template WHERE entry=243531""").fetchone()
        self.assertEqual(
            template,
            (
                "Zaralda",
                "Leatherworking Supplies",
                80,
                90,
                129,
                35,
                1.25,
                2000,
                1.5,
                2,
                3,
                4,
                11,
                321,
                -2,
                2,
                69497,
            ),
        )
        self.assertEqual(
            self.conn.execute(
                "SELECT * FROM content_creature_template_model"
            ).fetchall(),
            [(243531, 0, 138959, 1, 1, 69497)],
        )
        spawn = self.conn.execute("""SELECT id1, map, areaId, position_x, position_y, position_z,
            orientation, MovementType, wander_distance, spawntimesecs, modelid, VerifiedBuild,
            Comment FROM content_creature WHERE guid=90000001""").fetchone()
        self.assertEqual(
            spawn[:-1],
            (243531, 0, 15969, 8729.1, -4546, 42.5, 0, 0, 0, 120, 138959, None),
        )
        self.assertIn("derived", spawn[-1])
        self.assertIn(merchant()["source_url"], spawn[-1])
        self.assertIn("stationary", spawn[-1])
        self.assertEqual(
            self.conn.execute(
                "SELECT * FROM content_npc_vendor WHERE entry=243531 ORDER BY slot"
            ).fetchall(),
            [(243531, 0, 251665, 0, 0, 0, None), (243531, 1, 251691, 0, 0, 0, None)],
        )
        after = snapshot(self.conn)
        for name, rows in before.items():
            if not name.startswith("content_creature") and name != "content_npc_vendor":
                self.assertEqual(after[name], rows, name)
        self.assertIn(before["content_creature"][0], after["content_creature"])
        self.assertIn(before["content_npc_vendor"][0], after["content_npc_vendor"])

    def test_repeated_identical_application_is_noop(self):
        self.apply()
        before = snapshot(self.conn)
        self.assertEqual(self.apply(), 1)
        self.assertEqual(snapshot(self.conn), before)

    def test_existing_native_template_and_matching_models_are_supported(self):
        with self.conn:
            retail_content.insert_tdb_templates(self.conn, "243531")
            self.conn.execute(
                "INSERT INTO content_creature_template_model SELECT * FROM tdb_creature_template_model"
            )
        self.apply()
        self.assertEqual(
            self.conn.execute(
                "SELECT minlevel, maxlevel, npcflag FROM content_creature_template"
            ).fetchone(),
            (80, 90, 129),
        )

    def assert_rejected_without_writes(self, record, pattern):
        before = snapshot(self.conn)
        with self.conn:
            self.conn.execute("BEGIN")
            with self.assertRaisesRegex(ValueError, pattern):
                adapter.apply_derived_merchants(self.conn, [record])
            self.assertEqual(snapshot(self.conn), before)

    def test_manifest_requires_explicit_evidence_and_supported_policy(self):
        variants = [
            ("placement_kind", "native"),
            ("orientation", 1),
            ("position_z", float("nan")),
            ("position_x", float("inf")),
            ("position_y", "123"),
            ("guid", True),
            ("name", "Wrong NPC"),
            ("display_id", 999),
            ("minlevel", 91),
            ("source_url", ""),
        ]
        for field, value in variants:
            with self.subTest(field=field, value=value):
                record = merchant()
                record[field] = value
                self.assert_rejected_without_writes(record, field)
        for field in merchant():
            with self.subTest(missing=field):
                record = merchant()
                del record[field]
                self.assert_rejected_without_writes(record, field)
        record = merchant()
        record["vendors"] = []
        self.assert_rejected_without_writes(record, "vendors")

    def test_non_object_vendor_rejected_without_writes(self):
        record = merchant()
        record["vendors"][1] = None
        before = snapshot(self.conn)
        with self.conn:
            self.conn.execute("BEGIN")
            with self.assertRaisesRegex(TypeError, "vendor: expected an object"):
                adapter.apply_derived_merchants(self.conn, [record])
            self.assertEqual(snapshot(self.conn), before)

    def test_vendor_limits_currency_missing_and_mismatched_evidence_rejected(self):
        for field, value in (
            ("maxcount", 1),
            ("incrtime", 60),
            ("extended_cost", 7),
            ("buy_price", 736),
            ("buy_count", 2),
            ("item", 999),
            ("slot", -1),
        ):
            with self.subTest(field=field):
                record = merchant()
                record["vendors"][1][field] = value
                self.assert_rejected_without_writes(record, field)
        for field in merchant()["vendors"][0]:
            record = merchant()
            del record["vendors"][1][field]
            self.assert_rejected_without_writes(record, field)
        record = merchant()
        record["vendors"][1]["currency"] = 3263
        self.assert_rejected_without_writes(record, "currency")
        record = merchant()
        record["vendors"][1] = copy.deepcopy(record["vendors"][0])
        self.assert_rejected_without_writes(record, "duplicate")

    def test_missing_source_rows_and_content_items_rejected(self):
        for table in (
            "tdb_creature_template",
            "tdb_creature_template_model",
            "tdb_creature_template_difficulty",
            "item_sparse",
            "content_item",
            "creature_display_info",
        ):
            with self.subTest(table=table):
                conn = fixture()
                try:
                    conn.execute(f"DELETE FROM {table}")
                    conn.commit()
                    before = snapshot(conn)
                    with conn:
                        conn.execute("BEGIN")
                        with self.assertRaises(ValueError):
                            adapter.apply_derived_merchants(conn, [merchant()])
                        self.assertEqual(snapshot(conn), before)
                finally:
                    conn.close()

    def test_guid_collision_with_content_or_raw_spawn_is_rejected(self):
        for table, column in (
            ("content_creature", "id1"),
            ("creature", "id1"),
            ("tdb_creature", "id"),
        ):
            for entry in (999, 243531):
                with self.subTest(table=table, entry=entry):
                    self.conn.execute(
                        f"INSERT INTO {table} (guid, {column}) VALUES (90000001, ?)",
                        (entry,),
                    )
                    self.conn.commit()
                    self.assert_rejected_without_writes(merchant(), "guid")
                    self.conn.execute(f"DELETE FROM {table} WHERE guid=90000001")
                    self.conn.commit()

    def test_changed_record_does_not_overwrite_prior_publication(self):
        self.apply()
        record = merchant()
        record["position_z"] += 1
        self.assert_rejected_without_writes(record, "guid")

    def test_conflicting_template_model_and_vendor_rows_are_preserved(self):
        statements = [
            "INSERT INTO content_creature_template (entry, name) VALUES (243531, 'Other')",
            "INSERT INTO content_creature_template_model VALUES (243531, 0, 138959, 2, 1, 69497)",
            "INSERT INTO content_npc_vendor (entry, slot, item, maxcount) VALUES (243531, 0, 251665, 3)",
            "INSERT INTO content_npc_vendor (entry, slot, item) VALUES (243531, 0, 888)",
        ]
        for statement in statements:
            with self.subTest(statement=statement):
                conn = fixture()
                try:
                    conn.execute(statement)
                    conn.commit()
                    before = snapshot(conn)
                    with conn:
                        conn.execute("BEGIN")
                        with self.assertRaises(ValueError):
                            adapter.apply_derived_merchants(conn, [merchant()])
                        self.assertEqual(snapshot(conn), before)
                finally:
                    conn.close()

    def test_all_records_validate_before_mutation(self):
        second = merchant()
        second["guid"] += 1
        second["entry"] = 999
        before = snapshot(self.conn)
        with self.conn:
            self.conn.execute("BEGIN")
            with self.assertRaises(ValueError):
                adapter.apply_derived_merchants(self.conn, [merchant(), second])
            self.assertEqual(snapshot(self.conn), before)

    def test_caller_transaction_rolls_back_sql_failure_and_prior_work(self):
        self.conn.execute("""CREATE TRIGGER reject_vendor BEFORE INSERT ON content_npc_vendor
            WHEN NEW.entry=243531 BEGIN SELECT RAISE(ABORT, 'publication failed'); END""")
        self.conn.commit()
        before = snapshot(self.conn)
        with (
            self.assertRaisesRegex(sqlite3.IntegrityError, "publication failed"),
            self.conn,
        ):
            self.conn.execute("BEGIN")
            self.conn.execute(
                "INSERT INTO content_creature (guid, id1) VALUES (123, 456)"
            )
            adapter.apply_derived_merchants(self.conn, [merchant()])
        self.assertEqual(snapshot(self.conn), before)

    def test_requires_caller_transaction_and_supports_empty_manifest(self):
        with self.assertRaisesRegex(ValueError, "transaction"):
            adapter.apply_derived_merchants(self.conn, [merchant()])
        self.assertEqual(self.apply([]), 0)

    def test_optional_provenance_metadata_is_preserved_in_spawn_comment(self):
        record = merchant()
        record["provenance"] = {
            "observation": "zaralda-observed.json",
            "terrain": {"build": "12.1.0.69933"},
        }
        self.apply([record])
        comment = self.conn.execute(
            "SELECT Comment FROM content_creature WHERE guid=90000001"
        ).fetchone()[0]
        self.assertIn("zaralda-observed.json", comment)
        self.assertIn("12.1.0.69933", comment)
        record["guid"] += 1
        record["provenance"] = "unstructured"
        self.assert_rejected_without_writes(record, "provenance")

    def test_loads_explicit_json_list_and_rejects_invalid_shape(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "merchants.json"
            path.write_text(json.dumps([merchant()]))
            self.assertEqual(adapter.load_manifest(path), [merchant()])
            for data, error in (
                ({"merchants": []}, TypeError),
                ([None], TypeError),
                ([{"entry": 243531}], ValueError),
                ([merchant(), merchant()], ValueError),
            ):
                with self.subTest(data=data):
                    path.write_text(json.dumps(data))
                    with self.assertRaises(error):
                        adapter.load_manifest(path)
            path.write_text("[NaN]")
            with self.assertRaises(ValueError):
                adapter.load_manifest(path)
            path.unlink()
            with self.assertRaises(FileNotFoundError):
                adapter.load_manifest(path)


if __name__ == "__main__":
    unittest.main()
