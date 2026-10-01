"""Offline published skinning contract; concrete observations from cached item pages."""

from contextlib import closing
import hashlib
import importlib
import json
from pathlib import Path
import sqlite3
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import economy_catalog
import test_economy_catalog

# Exact source observations from the selected public pages (not synthetic loot).
OBSERVATIONS = {
    238511: [
        {"id": 249421, "name": "Venomous Mournbat", "count": 12886, "outof": 14314},
        {"id": 243171, "name": "Gloombelly Toad", "count": 15434, "outof": 17585},
    ],
    238513: [
        {"id": 238087, "name": "Agitated Wyrm", "count": 36506, "outof": 46046},
        {"id": 249959, "name": "Arcane Wyrm", "count": 15215, "outof": 18898},
    ],
}
PROVENANCE = {
    "kind": "published-item-skinning-page",
    "capture_method": "http-get",
    "timestamp_basis": "retrieval-time",
    "native_capture": False,
}


def page(item, rows=None, template="npc", tab="skinned-from"):
    rows = OBSERVATIONS[item] if rows is None else rows
    return f"""<html><script type="application/json" id="data.page.info">
    {{"entityType":3,"entityId":{item}}}</script><script>
    new Listview({{
        template: '{template}', id: '{tab}',
        name: WH.TERMS.skinnedfrom,
        data: {json.dumps(rows)},
    }});
    </script></html>"""


class GatheringEvidenceTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.cache = Path(self.tmp.name)

    def module(self):
        return importlib.import_module("gathering_evidence")

    def cache_page(self, item=238511, html=None):
        raw = (page(item) if html is None else html).encode()
        (self.cache / f"item-{item}.html").write_bytes(raw)
        metadata = {
            "source_url": f"https://www.wowhead.com/item={item}",
            "sha256": hashlib.sha256(raw).hexdigest(),
            "captured_utc": "2026-10-01T12:00:00Z",
            "provenance": PROVENANCE,
        }
        (self.cache / f"item-{item}.json").write_text(json.dumps(metadata))
        return metadata

    def test_actual_subsets_preserve_observations_and_provenance(self):
        for item in OBSERVATIONS:
            metadata = self.cache_page(item)
        records = self.module().load_published_skinning_evidence(self.cache)
        for item in OBSERVATIONS:
            self.assertEqual(records[item]["npcs"], OBSERVATIONS[item])
            self.assertEqual(records[item]["provenance"], PROVENANCE)
            self.assertEqual(
                records[item]["source_url"], f"https://www.wowhead.com/item={item}"
            )
            self.assertEqual(records[item]["captured_utc"], metadata["captured_utc"])

    def test_other_tabs_and_zone_template_do_not_prove_skinning(self):
        for template, tab in [
            ("npc", "sold-by"),
            ("npc", "dropped-by"),
            ("zone", "skinned-from"),
        ]:
            with (
                self.subTest(template=template, tab=tab),
                self.assertRaises(ValueError),
            ):
                self.module().parse_published_skinning(
                    page(238511, template=template, tab=tab), 238511
                )

    def test_wrong_page_and_duplicate_identity_rejected(self):
        for html in [
            page(238513),
            page(238511).replace('"entityType":3', '"entityType":6'),
            page(238511)
            + '<script type="application/json" id="data.page.info">{"entityType":3,"entityId":238511}</script>',
        ]:
            with self.subTest(html=html), self.assertRaises(ValueError):
                self.module().parse_published_skinning(html, 238511)

    def test_invalid_or_zero_observations_rejected(self):
        for row in [
            {"id": 0, "count": 1, "outof": 2},
            {"id": 2**32, "count": 1, "outof": 2},
            {"id": True, "count": 1, "outof": 2},
            {"id": 249421, "count": 0, "outof": 14314},
            {"id": 249421, "count": 5, "outof": 0},
            {"id": 249421, "count": 5, "outof": 4},
            {"id": 249421, "count": 1.0, "outof": 2},
        ]:
            with self.subTest(row=row), self.assertRaises(ValueError):
                self.module().parse_published_skinning(page(238511, [row]), 238511)
        with self.assertRaises(ValueError):
            self.module().parse_published_skinning(page(238511, []), 238511)

    def test_comments_and_strings_do_not_supply_a_skinning_block(self):
        for replacement in ["/* new Listview({", "const fake = `new Listview({"]:
            html = page(238511).replace("new Listview({", replacement)
            html = html.replace(
                "});", "}); */" if replacement.startswith("/*") else "});`;"
            )
            with self.subTest(replacement=replacement), self.assertRaises(ValueError):
                self.module().parse_published_skinning(html, 238511)

    def test_overridden_skinning_identity_properties_rejected(self):
        for override in ["id: 'dropped-by',", "template: 'zone',"]:
            html = page(238511).replace("name: WH.TERMS.skinnedfrom,", override)
            with self.subTest(override=override), self.assertRaises(ValueError):
                self.module().parse_published_skinning(html, 238511)

    def test_nonfinite_json_observation_fields_rejected(self):
        html = page(238511).replace('"name": "Venomous Mournbat"', '"name": NaN')
        with self.assertRaises(ValueError):
            self.module().parse_published_skinning(html, 238511)

    def test_non_json_and_duplicate_skinning_block_rejected(self):
        for html in [
            page(238511).replace('"id": 249421', "id: 249421"),
            page(238511).replace("data: [", "data: (() => ["),
            page(238511).replace(
                "</script></html>", page(238511).split("</script><script>")[1]
            ),
        ]:
            with self.subTest(html=html), self.assertRaises(ValueError):
                self.module().parse_published_skinning(html, 238511)

    def test_missing_cache_metadata_and_raw_fail(self):
        with self.assertRaises(ValueError):
            self.module().load_published_skinning_evidence(self.cache / "missing")
        self.cache_page()
        (self.cache / "item-238511.json").unlink()
        with self.assertRaises(ValueError):
            self.module().load_published_skinning_evidence(self.cache)
        self.cache_page()
        (self.cache / "item-238511.html").unlink()
        with self.assertRaises(ValueError):
            self.module().load_published_skinning_evidence(self.cache)

    def test_bad_hash_source_timestamp_or_provenance_fail(self):
        for field, value in [
            ("sha256", "0" * 64),
            ("source_url", "https://www.wowhead.com/item=238513"),
            ("captured_utc", "not-timeZ"),
            ("captured_utc", "2026-10-01"),
            ("provenance", {**PROVENANCE, "native_capture": True}),
            ("provenance", {**PROVENANCE, "native_capture": 0}),
        ]:
            metadata = self.cache_page()
            metadata[field] = value
            (self.cache / "item-238511.json").write_text(json.dumps(metadata))
            with self.subTest(field=field, value=value), self.assertRaises(ValueError):
                self.module().load_published_skinning_evidence(self.cache)

    def test_unselected_html_is_not_used(self):
        self.cache_page()
        (self.cache / "item-238513.html").write_text(page(238513))
        records = self.module().load_published_skinning_evidence(self.cache)
        self.assertEqual(set(records), {238511})

    def test_cli_selects_skin_cache_and_reports_observations_without_db_mutation(self):
        fixture = test_economy_catalog.CatalogTests(methodName="runTest")
        fixture.setUp()
        self.addCleanup(fixture.doCleanups)
        for item in OBSERVATIONS:
            self.cache_page(item)
        fixture.recipe(1237528, 2302, [(238511, 25), (238513, 15)])
        with closing(sqlite3.connect(fixture.db)) as db, db:
            db.executemany(
                "INSERT INTO content_item(ID,ClassID,SubclassID) VALUES (?,7,6)",
                [(238511,), (238513,)],
            )
            db.executemany(
                "INSERT INTO item_sparse(ID,ExpansionID) VALUES (?,11)",
                [(238511,), (238513,)],
            )
        with fixture.items.open("a") as stream:
            stream.write(
                "238511,1000,0.125,2.5,2026-10-01T12:00:00Z\n238513,1000,0.125,2.5,2026-10-01T12:00:00Z\n"
            )
        original = fixture.db.read_bytes()
        config, report = fixture.success("--published-skinning-cache", str(self.cache))
        self.assertEqual(fixture.db.read_bytes(), original)
        self.assertTrue(set(OBSERVATIONS) <= set(config["gatherables"]))
        for item in OBSERVATIONS:
            source = report["ingredient_sources"][str(item)]
            self.assertEqual(source["route"], "gathered_auction")
            self.assertEqual(source["published_skinning"]["npcs"], OBSERVATIONS[item])
            self.assertEqual(source["published_skinning"]["provenance"], PROVENANCE)
        (fixture.root / "config.json").unlink()
        (fixture.root / "report.json").unlink()
        (self.cache / "item-238511.html").write_text(page(238513))
        result = fixture.invoke(
            "--merchant", "3532:0:165", "--published-skinning-cache", str(self.cache)
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("hash/provenance mismatch", result.stderr)
        self.assertFalse((fixture.root / "config.json").exists())
        self.assertFalse((fixture.root / "report.json").exists())
        self.assertEqual(fixture.db.read_bytes(), original)

    def test_raw_resources_gather_but_crafted_and_non_leather_do_not(self):
        fixture = test_economy_catalog.CatalogTests(methodName="runTest")
        fixture.setUp()
        self.addCleanup(fixture.doCleanups)
        for item in OBSERVATIONS:
            self.cache_page(item)
        evidence = self.module().load_published_skinning_evidence(self.cache)
        with closing(sqlite3.connect(fixture.db)) as db, db:
            db.row_factory = sqlite3.Row
            db.executemany(
                "INSERT INTO content_item(ID,ClassID,SubclassID) VALUES (?,7,6)",
                [(238511,), (238513,)],
            )
            content = {
                row["ID"]: row for row in db.execute("SELECT * FROM content_item")
            }
            sources, _ = economy_catalog.inspect_sources(
                db, content, {2304}, {238511, 238513, 2320, 2304}, evidence
            )
            for item in OBSERVATIONS:
                self.assertEqual(sources[item]["route"], "gathered_auction")
                self.assertEqual(sources[item]["direct_loot"], [])
                self.assertEqual(sources[item]["published_skinning"], evidence[item])
            self.assertEqual(sources[2320]["route"], "vendor_supplier")
            sources, _ = economy_catalog.inspect_sources(
                db, content, {238511}, {238511, 238513}, evidence
            )
            self.assertEqual(sources[238511]["route"], "crafted_auction")
            db.execute("UPDATE content_item SET SubclassID=0 WHERE ID=238513")
            content = {
                row["ID"]: row for row in db.execute("SELECT * FROM content_item")
            }
            sources, _ = economy_catalog.inspect_sources(
                db, content, set(), {238513}, evidence
            )
            self.assertNotEqual(sources[238513]["route"], "gathered_auction")
            self.assertEqual(
                db.execute(
                    "SELECT COUNT(*) FROM content_creature_template WHERE entry IN (249421,238087)"
                ).fetchone()[0],
                0,
            )


if __name__ == "__main__":
    unittest.main()
