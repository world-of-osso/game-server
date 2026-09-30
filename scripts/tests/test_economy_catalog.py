"""Concrete SQLite/CSV importer boundary tests; stdlib only."""

import csv
from contextlib import closing
import importlib.util
import json
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import threading
import unittest
import urllib.error
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

SCRIPT = Path(__file__).resolve().parents[1] / "economy_catalog.py"


class CatalogTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.db = self.root / "world.db"
        con = sqlite3.connect(self.db)
        con.executescript("""
            CREATE TABLE content_item(ID INTEGER PRIMARY KEY, ClassID INTEGER, SubclassID INTEGER,
                Name TEXT DEFAULT 'Fixture item', Quality INTEGER DEFAULT 1,
                RequiredLevel INTEGER DEFAULT 5, SellPrice INTEGER DEFAULT 25);
            CREATE TABLE item_sparse(ID INTEGER PRIMARY KEY, ExpansionID INTEGER);
            CREATE TABLE skill_line(ID INTEGER PRIMARY KEY, ParentSkillLineID INTEGER);
            CREATE TABLE skill_line_ability(Spell INTEGER, SkillLine INTEGER);
            CREATE TABLE spell_effect(SpellID INTEGER, DifficultyID INTEGER, Effect INTEGER,
                EffectItemType INTEGER, EffectBasePointsF REAL, Variance REAL,
                Coefficient REAL, EffectRealPointsPerLevel REAL, EffectPointsPerResource REAL,
                ResourceCoefficient REAL, GroupSizeBasePointsCoefficient REAL, ScalingClass INTEGER);
            CREATE TABLE content_creature_template(entry INTEGER, name TEXT, npcflag INTEGER);
            CREATE TABLE content_creature(guid INTEGER, id1 INTEGER, id2 INTEGER, id3 INTEGER, map INTEGER);
            INSERT INTO skill_line VALUES (165,0),(2532,165),(2915,165),(164,0);
            INSERT INTO content_creature_template VALUES (3532,'Pyrewood Leatherworker',128);
            INSERT INTO content_creature VALUES (1,3532,0,0,0);
        """)
        columns = ",".join(
            f"{prefix}_{i} INTEGER DEFAULT 0"
            for prefix in [
                "Reagent",
                "ReagentCount",
                "ReagentReCraftCount",
                "ReagentSource",
            ]
            for i in range(8)
        )
        con.execute(f"CREATE TABLE spell_reagents(SpellID INTEGER,{columns})")
        con.executemany(
            "INSERT INTO content_item(ID,ClassID,SubclassID) VALUES (?,?,?)",
            [(2302, 4, 1), (2318, 7, 6), (2320, 7, 0), (2304, 7, 6), (900, 4, 1)],
        )
        con.executemany(
            "INSERT INTO item_sparse VALUES (?,?)",
            [(2302, 0), (2318, 0), (2320, 0), (2304, 0), (900, 11)],
        )
        con.commit()
        con.close()
        self.recipe(2149, 2302, [(2318, 2), (2320, 1)])
        self.recipe(2152, 2304, [(2318, 1)])
        self.recipe(500, 900, [(2318, 1)], skill=2915)
        self.items = self.root / "items.csv"
        self.commodities = self.root / "commodities.csv"
        self.write_csv(
            self.items,
            ["itemId", "marketValue", "saleRate", "soldPerDay", "updatedAt"],
            [
                [i, 1000, "0.125", "2.5", "2026-09-30T08:19:11Z"]
                for i in [2302, 2318, 2320, 2304, 900]
            ],
        )
        self.write_csv(
            self.commodities,
            ["itemId", "marketValue", "minBuyout", "updatedAt"],
            [[2318, 2000, 2100, "2026-09-30T15:23:54Z"]],
        )

    def write_csv(self, path, header, rows):
        with path.open("w", newline="") as f:
            writer = csv.writer(f)
            writer.writerow(header)
            writer.writerows(rows)

    def recipe(
        self, spell, output, ingredients, skill=2532, points=1, variance=0, source=0
    ):
        with closing(sqlite3.connect(self.db)) as con, con:
            con.execute("INSERT INTO skill_line_ability VALUES (?,?)", (spell, skill))
            con.execute(
                "INSERT INTO spell_effect VALUES (?,0,24,?,?,?,0,0,0,0,1,0)",
                (spell, output, points, variance),
            )
            fields = ["SpellID"]
            values = [spell]
            for i, (item, count) in enumerate(ingredients):
                fields.extend(
                    [f"Reagent_{i}", f"ReagentCount_{i}", f"ReagentSource_{i}"]
                )
                values.extend([item, count, source])
            con.execute(
                f"INSERT INTO spell_reagents ({','.join(fields)}) VALUES ({','.join('?' for _ in fields)})",
                values,
            )

    def invoke(self, *args):
        result = subprocess.run(
            [
                sys.executable,
                str(SCRIPT),
                "--db",
                str(self.db),
                "--items",
                str(self.items),
                "--commodities",
                str(self.commodities),
                "--output",
                str(self.root / "config.json"),
                "--report",
                str(self.root / "report.json"),
                *args,
            ],
            capture_output=True,
            text=True,
        )
        return result

    def success(self, *args):
        result = self.invoke("--merchant", "3532:0:165", *args)
        self.assertEqual(result.returncode, 0, result.stderr)
        return json.loads((self.root / "config.json").read_text()), json.loads(
            (self.root / "report.json").read_text()
        )

    def test_real_recipe_quantity_expansion_and_measured_markets(self):
        config, report = self.success()
        recipes = config["merchants"][0]["recipes"]
        self.assertEqual(
            recipes,
            [
                {
                    "spell_id": 2149,
                    "expansion": 0,
                    "output_item_id": 2302,
                    "output_count": 1,
                    "ingredients": [[2318, 2], [2320, 1]],
                },
                {
                    "spell_id": 2152,
                    "expansion": 0,
                    "output_item_id": 2304,
                    "output_count": 1,
                    "ingredients": [[2318, 1]],
                },
            ],
        )
        self.assertEqual(
            config["markets"]["2318"],
            {
                "item_id": 2318,
                "anchor_price": 2000,
                "min_price": 1600,
                "max_price": 2400,
                "popularity_bps": 1250,
            },
        )
        self.assertNotIn("900", config["markets"])
        self.assertEqual(report["gathering_item_ids"], [2318])
        self.assertEqual(config["gatherables"], [2318])
        self.assertGreater(config["gatherer_capital"], 0)
        self.assertEqual(
            config["items"]["2318"],
            {
                "name": "Fixture item",
                "quality": 1,
                "required_level": 5,
                "vendor_sell_price": 25,
            },
        )
        self.assertEqual(report["coverage"]["11:165"]["eligible"], 1)
        self.assertEqual(report["assignments"][0]["spawn_count"], 1)

    def test_raw_quantity_rounds_half_up_and_clamps_to_one(self):
        for spell, points in [
            (1100, 0),
            (1101, 1.49),
            (1102, 1.5),
            (1103, 2.5),
            (1104, -2),
        ]:
            self.recipe(spell, 2302, [(2318, 1)], points=points)
        config, _ = self.success()
        counts = {
            r["spell_id"]: r["output_count"] for r in config["merchants"][0]["recipes"]
        }
        self.assertEqual(
            {spell: counts[spell] for spell in range(1100, 1105)},
            {1100: 1, 1101: 1, 1102: 2, 1103: 3, 1104: 1},
        )
        self.assertEqual(counts[2149], 1)

    def test_report_preserves_price_and_popularity_provenance(self):
        _, report = self.success()
        self.assertEqual(
            report["market_provenance"]["2318"],
            {
                "price_source": "commodities",
                "market_value": "2000",
                "price_updated_at": "2026-09-30T15:23:54Z",
                "popularity_source": "items",
                "sale_rate": "0.125",
                "sold_per_day": "2.5",
                "popularity_updated_at": "2026-09-30T08:19:11Z",
            },
        )
        self.assertEqual(report["market_provenance"]["2302"]["price_source"], "items")
        self.assertEqual(report["market_provenance"]["2302"]["market_value"], "1000")
        self.assertEqual(
            report["snapshots"]["items"]["updated_at"], ["2026-09-30T08:19:11Z"]
        )
        self.assertEqual(
            report["snapshots"]["commodities"]["updated_at"], ["2026-09-30T15:23:54Z"]
        )

    def test_midnight_missing_profession_mapping_reported(self):
        self.recipe(501, 900, [(2318, 1)], skill=2915)
        with closing(sqlite3.connect(self.db)) as con, con:
            con.execute("DELETE FROM skill_line_ability WHERE Spell=501")
        _, report = self.success()
        self.assertEqual(
            report["output_spell_coverage"]["11"],
            {
                "create_item_spells": 2,
                "profession_mapped": 1,
                "missing_profession_mapping": 1,
            },
        )
        self.assertEqual(
            report["unmapped_output_spells"],
            [{"spell_id": 501, "output_item_id": 900, "expansion": 11}],
        )

    def test_midnight_is_eleven_not_twelve(self):
        result = self.invoke("--merchant", "3532:11:165")
        self.assertEqual(result.returncode, 0, result.stderr)
        config = json.loads((self.root / "config.json").read_text())
        self.assertEqual(
            [r["spell_id"] for r in config["merchants"][0]["recipes"]], [500]
        )

    def test_excludes_missing_content_tsm_random_and_special_reagents(self):
        self.recipe(1001, 2302, [(9999, 1)])
        self.recipe(1002, 2302, [(2318, 1)], variance=0.5)
        self.recipe(1003, 2302, [(2318, 1)], source=1)
        self.recipe(1004, 2302, [(2318, 1)], points=float("inf"))
        with closing(sqlite3.connect(self.db)) as con, con:
            con.execute(
                "INSERT INTO content_item(ID,ClassID,SubclassID) VALUES (9998,7,9)"
            )
        self.recipe(1005, 2302, [(9998, 1)])
        self.recipe(1006, 9997, [(2318, 1)])
        config, report = self.success()
        self.assertEqual(len(config["merchants"][0]["recipes"]), 2)
        reasons = {r["spell_id"]: r["reason"] for r in report["excluded_recipes"]}
        self.assertEqual(reasons[1001], "missing_content_item")
        self.assertEqual(reasons[1002], "non_deterministic_output")
        self.assertEqual(reasons[1003], "special_reagents")
        self.assertEqual(reasons[1004], "non_deterministic_output")
        self.assertEqual(reasons[1005], "missing_supported_market")
        self.assertEqual(reasons[1006], "missing_output_expansion")

    def test_crafted_ingredients_never_classified_as_gathered(self):
        self.recipe(1007, 2302, [(2304, 1), (2318, 1)])
        _, report = self.success()
        self.assertEqual(report["gathering_item_ids"], [2318])
        self.assertIn(2304, report["non_gathering_ingredient_ids"])

    def test_unspawned_and_non_vendor_assignment_rejected_with_report(self):
        with closing(sqlite3.connect(self.db)) as con, con:
            con.execute("DELETE FROM content_creature")
        result = self.invoke("--merchant", "3532:0:165")
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse((self.root / "config.json").exists())
        report = json.loads((self.root / "report.json").read_text())
        self.assertEqual(report["assignments"][0]["error"], "merchant_not_spawned")
        with closing(sqlite3.connect(self.db)) as con, con:
            con.execute("INSERT INTO content_creature VALUES (1,3532,0,0,0)")
            con.execute("UPDATE content_creature_template SET npcflag=0")
        result = self.invoke("--merchant", "3532:0:165")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("merchant_not_vendor", (self.root / "report.json").read_text())

    def test_assignment_required_and_tuning_editable(self):
        self.assertNotEqual(self.invoke().returncode, 0)
        tuning = self.root / "tuning.json"
        tuning.write_text(
            json.dumps({"customer_budget": 42, "capacity": 8, "capital_limit": 900})
        )
        config, _ = self.success("--tuning", str(tuning))
        self.assertEqual(config["customer_budget"], 42)
        self.assertEqual(config["merchants"][0]["capacity"], 8)
        self.assertEqual(config["merchants"][0]["capital_limit"], 900)
        tuning.write_text('{"capacity": -1}')
        self.assertNotEqual(
            self.invoke("--merchant", "3532:0:165", "--tuning", str(tuning)).returncode,
            0,
        )

    def test_missing_popularity_schema_fails_no_vendor_price_fallback(self):
        self.write_csv(self.items, ["itemId", "marketValue"], [[2318, 0]])
        result = self.invoke("--merchant", "3532:0:165")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("saleRate", result.stderr)

    def test_zero_popularity_is_observed_not_replaced(self):
        self.write_csv(
            self.items,
            ["itemId", "marketValue", "saleRate", "soldPerDay", "updatedAt"],
            [[i, 1000, 0, 0, "now"] for i in [2302, 2318, 2320, 2304, 900]],
        )
        config, _ = self.success()
        self.assertEqual(config["markets"]["2302"]["popularity_bps"], 0)

    def test_multiple_effects_and_recraft_reagents_reported(self):
        self.recipe(1101, 2302, [(2318, 1)])
        self.recipe(1102, 2302, [(2318, 1)])
        with closing(sqlite3.connect(self.db)) as con, con:
            con.execute(
                "INSERT INTO spell_effect VALUES (1101,0,24,2304,1,0,0,0,0,0,1,0)"
            )
            con.execute(
                "UPDATE spell_reagents SET ReagentReCraftCount_0=1 WHERE SpellID=1102"
            )
        _, report = self.success()
        reasons = {r["spell_id"]: r["reason"] for r in report["excluded_recipes"]}
        self.assertEqual(reasons[1101], "multiple_or_special_effects")
        self.assertEqual(reasons[1102], "special_reagents")

    def test_metadata_invalid_template_fails_explicitly(self):
        with closing(sqlite3.connect(self.db)) as con, con:
            con.execute("UPDATE content_item SET RequiredLevel=-1 WHERE ID=2318")
        result = self.invoke("--merchant", "3532:0:165")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("invalid RequiredLevel", result.stderr)
        self.assertFalse((self.root / "config.json").exists())

    def test_bad_market_rows_excluded_zero_popularity_preserved(self):
        self.write_csv(
            self.items,
            ["itemId", "marketValue", "saleRate", "soldPerDay", "updatedAt"],
            [
                [2302, 1000, 0, 0, "now"],
                [2318, 1000, "NaN", 1, "now"],
                [2320, 1000, 0.1, 1, "now"],
                [2304, 0, 0.1, 1, "now"],
                [900, 1000, 1.1, 1, "now"],
            ],
        )
        config, report = self.success()
        self.assertEqual(config["merchants"][0]["recipes"], [])
        self.assertEqual(len(report["excluded_market_rows"]), 3)
        self.assertEqual(report["assignments"][0]["eligible_recipe_count"], 0)
        self.assertEqual(report["assignments"][0]["warning"], "no_supported_recipes")
        self.assertEqual(config["markets"], {})
        self.assertEqual(config["items"], {})
        self.assertEqual(config["gatherables"], [])

    def test_unsupported_midnight_assignment_warns_without_producing_goods(self):
        with closing(sqlite3.connect(self.db)) as con, con:
            con.execute("DELETE FROM skill_line_ability WHERE Spell=500")
        result = self.invoke("--merchant", "3532:11:165")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("no_supported_recipes", result.stderr)
        config = json.loads((self.root / "config.json").read_text())
        report = json.loads((self.root / "report.json").read_text())
        self.assertEqual(config["merchants"][0]["recipes"], [])
        self.assertEqual(config["gatherables"], [])
        self.assertEqual(config["items"], {})
        self.assertEqual(config["markets"], {})
        self.assertEqual(
            report["output_spell_coverage"]["11"],
            {
                "create_item_spells": 1,
                "profession_mapped": 0,
                "missing_profession_mapping": 1,
            },
        )

    def test_expansion_outside_runtime_i32_rejected(self):
        result = self.invoke("--merchant", "3532:2147483648:165")
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse((self.root / "config.json").exists())

    def test_invalid_commodity_price_never_uses_region_or_vendor_price(self):
        self.write_csv(
            self.commodities,
            ["itemId", "marketValue", "updatedAt"],
            [[2318, 0, "2026-09-30T15:23:54Z"]],
        )
        config, report = self.success()
        self.assertEqual(config["merchants"][0]["recipes"], [])
        self.assertEqual(
            report["excluded_market_rows"],
            [{"item_id": 2318, "reason": "invalid item ID or copper price"}],
        )
        self.assertNotIn("2318", report["market_provenance"])


class DownloadTests(unittest.TestCase):
    def test_http_retry_after_then_saved_snapshot_and_permanent_error(self):
        spec = importlib.util.spec_from_file_location("catalog", SCRIPT)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)

        class Handler(BaseHTTPRequestHandler):
            attempts = 0

            def log_message(self, *args):
                return

            def do_GET(self):
                Handler.attempts += 1
                if self.path == "/permanent":
                    self.send_error(404)
                    return
                if self.path == "/retry" and Handler.attempts == 1:
                    self.send_response(429)
                    self.send_header("Retry-After", "0")
                    self.end_headers()
                    return
                if self.path == "/exhaust":
                    self.send_response(503)
                    self.send_header("Retry-After", "0")
                    self.end_headers()
                    return
                if self.path == "/dated" and Handler.attempts == 5:
                    self.send_response(429)
                    self.send_header("Retry-After", "Wed, 30 Sep 2020 00:00:00 GMT")
                    self.end_headers()
                    return
                if self.path == "/bounded":
                    self.send_response(503)
                    self.send_header("Retry-After", "999")
                    self.end_headers()
                    return
                self.send_response(200)
                self.end_headers()
                self.wfile.write(b"itemId,marketValue\n2318,2000\n")

        server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        self.addCleanup(server.server_close)
        self.addCleanup(server.shutdown)
        with tempfile.TemporaryDirectory() as tmp:
            target = Path(tmp) / "items.csv"
            url = f"http://127.0.0.1:{server.server_port}"
            module.fetch_snapshot(url + "/retry", target)
            self.assertEqual(target.read_bytes(), b"itemId,marketValue\n2318,2000\n")
            self.assertEqual(Handler.attempts, 2)
            with self.assertRaises(urllib.error.HTTPError):
                module.fetch_snapshot(url + "/permanent", target)
            self.assertEqual(Handler.attempts, 3)
            with self.assertRaises(ValueError):
                module.fetch_snapshot(url + "/bounded", target)
            self.assertEqual(Handler.attempts, 4)
            module.fetch_snapshot(url + "/dated", target)
            self.assertEqual(Handler.attempts, 6)
            with self.assertRaises(urllib.error.HTTPError):
                module.fetch_snapshot(url + "/exhaust", target)
            self.assertEqual(Handler.attempts, 10)
            self.assertEqual(target.read_bytes(), b"itemId,marketValue\n2318,2000\n")


if __name__ == "__main__":
    unittest.main()
