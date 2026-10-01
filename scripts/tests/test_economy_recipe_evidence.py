"""Offline published evidence and concrete SQLite crosscheck fixtures."""

import copy
import hashlib
import importlib
import json
from pathlib import Path
import sqlite3
import sys
import tempfile
import unittest

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
import published_reagents

try:
    evidence = importlib.import_module("economy_recipe_evidence")
except ModuleNotFoundError:
    evidence = None

CACHE = SCRIPTS.parent / "data/reference/midnight-20261001/published-reagent-pages"
VEST = 1237528
ITEMS = [[238511, 15], [238513, 25]]


def fixture_page(spell=VEST):
    rows = "".join(
        f'<tr data-icon-list-quantity="{count}"><td>'
        f'<a href="/item={item}/material">Material</a></td></tr>'
        for item, count in ITEMS
    )
    return (
        '<html><script type="application/json" id="data.page.info">'
        f'{{"entityId":{spell},"entityType":6}}</script>'
        '<h2 id="icon-list-heading-reagents">Reagents</h2>'
        f'<table class="icon-list">{rows}</table></html>'
    ).encode()


def write_snapshot(directory, raw=None, spell=VEST):
    raw = fixture_page(spell) if raw is None else raw
    metadata = {
        "source_url": f"https://www.wowhead.com/spell={spell}",
        "sha256": hashlib.sha256(raw).hexdigest(),
        "captured_utc": "2026-10-01T10:52:29Z",
        "provenance": {
            "kind": "published-spell-page",
            "capture_method": "http-get",
            "timestamp_basis": "retrieval-time",
            "native_capture": False,
        },
    }
    record = {
        **published_reagents.parse_published_reagents(raw.decode(), spell),
        **metadata,
    }
    (directory / f"spell-{spell}.html").write_bytes(raw)
    (directory / f"spell-{spell}.json").write_text(json.dumps(metadata))
    snapshot = directory / "snapshot.json"
    snapshot.write_text(
        json.dumps(
            {
                "schema_version": 1,
                "effect": 288,
                "skillup_line": 2915,
                "records": [record],
            }
        )
    )
    return snapshot, record


def fixture_db(row_factory=None):
    db = sqlite3.connect(":memory:")
    db.row_factory = row_factory
    db.executescript((SCRIPTS / "crafting_schema.sql").read_text())
    schema = (SCRIPTS / "db2_schema.sql").read_text()
    statement = schema.split("CREATE TABLE spell_reagents (", 1)[1].split(";", 1)[0]
    db.executescript("CREATE TABLE spell_reagents (" + statement + ";")
    db.execute(
        "CREATE TABLE item_sparse (ID INTEGER PRIMARY KEY, ModifiedCraftingReagentItemID INTEGER NOT NULL)"
    )
    for slot, count, item in [(421, 15, 238511), (422, 25, 238513), (470, 80, 999)]:
        source = int(slot == 470)
        db.execute(
            "INSERT INTO modified_crafting_reagent_slot VALUES ('Material', ?, 2, 0, 1, ?, 0, 0)",
            (slot, source),
        )
        db.execute(
            "INSERT INTO modified_crafting_spell_slot VALUES (?, ?, ?, ?, 0, ?, 8)",
            (slot, VEST, slot, slot, count),
        )
        db.execute(
            "INSERT INTO modified_crafting_category VALUES (?, 'Material', '', 0, 0, 0)",
            (slot,),
        )
        db.execute(
            "INSERT INTO mcr_slot_x_mcr_category VALUES (?, ?, 0, ?)",
            (slot, slot, slot),
        )
        db.execute(
            "INSERT INTO modified_crafting_reagent_item VALUES (?, '', ?, 0, 0, 0, 0)",
            (slot, slot),
        )
        db.execute("INSERT INTO item_sparse VALUES (?, ?)", (item, slot))
    return db


class EvidenceTests(unittest.TestCase):
    def setUp(self):
        self.assertIsNotNone(
            evidence, "economy_recipe_evidence adapter not implemented"
        )
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.directory = Path(self.temp.name)
        self.snapshot, self.record = write_snapshot(self.directory)

    def load(self):
        return evidence.load_published_evidence(self.snapshot, self.directory)

    def test_load_preserves_exact_published_provenance(self):
        self.assertEqual(self.load(), {VEST: self.record})
        self.assertIs(self.load()[VEST]["provenance"]["native_capture"], False)

    def test_genuine_cached_vest_and_full_spark_page(self):
        for spell in (VEST, 1237486):
            with self.subTest(spell=spell):
                snapshot, expected = write_snapshot(
                    self.directory, (CACHE / f"spell-{spell}.html").read_bytes(), spell
                )
                metadata = json.loads((CACHE / f"spell-{spell}.json").read_text())
                expected.update(metadata)
                (self.directory / f"spell-{spell}.json").write_text(
                    json.dumps(metadata)
                )
                wrapper = json.loads(snapshot.read_text())
                wrapper["records"] = [expected]
                snapshot.write_text(json.dumps(wrapper))
                loaded = self.load()[spell]
                self.assertEqual(loaded, expected)
                if spell == VEST:
                    self.assertEqual(loaded["items"], ITEMS)
                    self.assertTrue(loaded["eligible"])
                    with fixture_db() as db:
                        self.assertEqual(
                            evidence.crosscheck_published_inputs(db, spell, loaded),
                            (None, ITEMS),
                        )
                else:
                    self.assertFalse(loaded["eligible"])
                    self.assertTrue(
                        any("Spark" in row["text"] for row in loaded["unresolved"])
                    )
                    with fixture_db() as db:
                        self.assertEqual(
                            evidence.crosscheck_published_inputs(db, spell, loaded),
                            ("ineligible_published_reagents", []),
                        )

    def test_reject_edited_snapshot_fields_and_duplicate_spell(self):
        original = json.loads(self.snapshot.read_text())
        edits = [
            ("items", [[238511, 14], [238513, 25]]),
            ("unresolved", [{"reason": "invented"}]),
            ("eligible", False),
            ("eligible", 1),
            ("sha256", "0" * 64),
            ("source_url", "https://example.com/spell=1237528"),
            ("captured_utc", "2026-10-01T11:00:00Z"),
            ("provenance", {"kind": "native", "native_capture": True}),
        ]
        for key, value in edits:
            with self.subTest(key=key):
                wrapper = copy.deepcopy(original)
                wrapper["records"][0][key] = value
                self.snapshot.write_text(json.dumps(wrapper))
                with self.assertRaises(ValueError):
                    self.load()
        original["records"].append(original["records"][0])
        self.snapshot.write_text(json.dumps(original))
        with self.assertRaisesRegex(ValueError, "duplicate"):
            self.load()

    def test_reject_schema_effect_and_skillup_line(self):
        original = json.loads(self.snapshot.read_text())
        for key, value in [
            ("schema_version", 2),
            ("schema_version", True),
            ("effect", 24),
            ("skillup_line", 0),
            ("skillup_line", "2915"),
            ("records", {}),
        ]:
            with self.subTest(key=key, value=value):
                wrapper = copy.deepcopy(original)
                wrapper[key] = value
                self.snapshot.write_text(json.dumps(wrapper))
                with self.assertRaises(ValueError):
                    self.load()

    def test_require_raw_and_acquisition_metadata_without_creating_cache(self):
        for suffix in ("html", "json"):
            with self.subTest(suffix=suffix):
                write_snapshot(self.directory)
                (self.directory / f"spell-{VEST}.{suffix}").unlink()
                with self.assertRaises(ValueError):
                    self.load()
        absent = self.directory / "absent"
        with self.assertRaises(ValueError):
            evidence.load_published_evidence(self.snapshot, absent)
        self.assertFalse(absent.exists())

    def test_reject_raw_hash_source_identity_and_native_metadata(self):
        for mode in (
            "hash",
            "source",
            "identity",
            "native",
            "timestamp",
            "capture_method",
        ):
            with self.subTest(mode=mode):
                write_snapshot(self.directory)
                metadata_path = self.directory / f"spell-{VEST}.json"
                metadata = json.loads(metadata_path.read_text())
                if mode == "hash":
                    (self.directory / f"spell-{VEST}.html").write_bytes(b"edited raw")
                elif mode == "identity":
                    raw = fixture_page(1237486)
                    (self.directory / f"spell-{VEST}.html").write_bytes(raw)
                    metadata["sha256"] = hashlib.sha256(raw).hexdigest()
                elif mode == "source":
                    metadata["source_url"] += "/other"
                elif mode == "native":
                    metadata["provenance"]["native_capture"] = True
                elif mode == "timestamp":
                    metadata["captured_utc"] = "invalidZ"
                else:
                    del metadata["provenance"]["capture_method"]
                metadata_path.write_text(json.dumps(metadata))
                wrapper = json.loads(self.snapshot.read_text())
                wrapper["records"][0].update(metadata)
                self.snapshot.write_text(json.dumps(wrapper))
                with self.assertRaises(ValueError):
                    self.load()


class CrosscheckTests(unittest.TestCase):
    def setUp(self):
        self.assertIsNotNone(
            evidence, "economy_recipe_evidence adapter not implemented"
        )
        self.db = fixture_db()
        self.addCleanup(self.db.close)
        self.record = {
            "spell_id": VEST,
            "items": ITEMS,
            "eligible": True,
            "unresolved": [],
        }

    def check(self, reason=None, items=None):
        self.assertEqual(
            evidence.crosscheck_published_inputs(self.db, VEST, self.record),
            (reason, ITEMS if items is None and reason is None else items or []),
        )

    def test_concrete_slots_sorted_read_only_and_row_factory_independent(self):
        before = self.db.total_changes
        self.db.execute("PRAGMA query_only=ON")
        self.record["items"] = list(reversed(ITEMS))
        self.check()
        self.assertEqual(self.db.total_changes, before)
        self.db.row_factory = sqlite3.Row
        self.check()

    def test_no_inference_of_unlisted_slot_requiredness(self):
        self.db.execute(
            "UPDATE modified_crafting_reagent_slot SET ReagentType=99, Flags=999"
        )
        self.record["items"] = [[238511, 15]]
        self.check(items=[[238511, 15]])

    def test_fixed_inputs_must_occur_with_exact_aggregated_count(self):
        self.db.execute(
            "INSERT INTO spell_reagents (ID, SpellID, Reagent_0, ReagentCount_0, Reagent_1, ReagentCount_1) VALUES (1, ?, 777, 2, 777, 3)",
            (VEST,),
        )
        self.check("missing_fixed_published_reagent")
        self.record["items"] = ITEMS + [[777, 4]]
        self.check("incorrect_fixed_published_count")
        self.record["items"] = ITEMS + [[777, 5]]
        self.check(items=[[777, 5]] + ITEMS)

    def test_fixed_source_recraft_and_invalid_pairs_not_ignored(self):
        self.db.execute(
            "INSERT INTO spell_reagents (ID, SpellID) VALUES (1, ?)", (VEST,)
        )
        for column, value in [
            ("ReagentSource_7", 1),
            ("ReagentSource_7", 99),
            ("ReagentReCraftCount_7", 1),
            ("Reagent_7", -1),
            ("ReagentCount_7", 2),
        ]:
            with self.subTest(column=column, value=value):
                self.db.execute(f"UPDATE spell_reagents SET {column}=?", (value,))
                self.check("special_reagents")
                self.db.execute(f"UPDATE spell_reagents SET {column}=0")

    def test_duplicate_fixed_rows_are_unsupported(self):
        self.db.executemany(
            "INSERT INTO spell_reagents (ID, SpellID) VALUES (?, ?)",
            [(1, VEST), (2, VEST)],
        )
        self.check("ambiguous_fixed_reagents")

    def test_reject_ineligible_or_nonconcrete_record(self):
        original = copy.deepcopy(self.record)
        changes = [
            ("eligible", False),
            ("eligible", 1),
            ("unresolved", [{}]),
            ("unresolved", None),
            ("spell_id", 1237486),
        ]
        for key, value in changes:
            with self.subTest(key=key):
                self.record = {**original, key: value}
                self.check("ineligible_published_reagents")
        for pairs in [
            [],
            [[0, 15]],
            [[238511, 0]],
            [[238511, -1]],
            [[True, 1]],
            [[238511, True]],
            [[238511, 1.5]],
            [["238511", 15]],
            [[238511, 2**32]],
            ITEMS + [ITEMS[0]],
            [[238511]],
            None,
        ]:
            with self.subTest(pairs=pairs):
                self.record = {**original, "items": pairs}
                self.check("invalid_published_reagents")

    def test_reject_wrong_count_unmapped_and_selected_currency_source(self):
        self.record["items"] = [[238511, 14], [238513, 25]]
        self.check("incorrect_modified_published_count")
        self.record["items"] = ITEMS + [[888, 1]]
        self.check("unmapped_published_reagent")
        self.record["items"] = ITEMS + [[999, 80]]
        self.check("special_modified_reagent_source")
        self.record["items"] = ITEMS
        self.db.execute(
            "UPDATE modified_crafting_reagent_slot SET ReagentSource=99 WHERE ID=421"
        )
        self.check("special_modified_reagent_source")

    def test_independent_category_and_item_relation_required(self):
        self.db.execute(
            "UPDATE item_sparse SET ModifiedCraftingReagentItemID=238511 WHERE ID=238511"
        )
        self.check("unmapped_published_reagent")
        self.db.execute(
            "UPDATE item_sparse SET ModifiedCraftingReagentItemID=421 WHERE ID=238511"
        )
        self.db.execute("DELETE FROM mcr_slot_x_mcr_category WHERE ID=421")
        self.check("unmapped_published_reagent")

    def test_ambiguous_slots_reject_without_searching_assignments(self):
        self.db.execute(
            "INSERT INTO modified_crafting_spell_slot VALUES (900, ?, 900, 421, 0, 15, 8)",
            (VEST,),
        )
        self.check("ambiguous_published_reagent_slots")

    def test_slot_cannot_be_selected_twice_and_duplicate_links_are_not_ambiguity(self):
        self.db.execute("INSERT INTO mcr_slot_x_mcr_category VALUES (901, 421, 1, 421)")
        self.check()
        self.db.execute("INSERT INTO item_sparse VALUES (238512, 421)")
        self.record["items"] = ITEMS + [[238512, 15]]
        self.check("ambiguous_published_reagent_slots")


if __name__ == "__main__":
    unittest.main()
