"""Published Wowhead reagent contracts; concrete subsets of cached spell pages."""

from contextlib import closing, redirect_stderr
import hashlib
import importlib.util
import json
from pathlib import Path
import sqlite3
import sys
import tempfile
import unittest
from unittest.mock import patch
from urllib.error import HTTPError, URLError
from io import BytesIO, StringIO

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
if importlib.util.find_spec("published_reagents"):
    import published_reagents as adapter
else:
    adapter = None

# Subset of spell-1237528.html: identity + required/optional sections.
VEST_ROWS = """<tr data-icon-list-quantity="15"><th align="right" id="icon-list-icon1"></th><td><a href="/item=238511/void-tempered-leather" class="q1">Void-Tempered Leather</a><span class="icon-list-quantity-wrapper" data-icon-list-quantity="15">&nbsp;(<span class="icon-list-quantity">15</span>)</span></td></tr>
<tr data-icon-list-quantity="25"><th align="right" id="icon-list-icon2"></th><td><a href="/item=238513/void-tempered-scales" class="q1">Void-Tempered Scales</a><span class="icon-list-quantity-wrapper" data-icon-list-quantity="25">&nbsp;(<span class="icon-list-quantity">25</span>)</span></td></tr>"""
# Required Spark row from spell-1237486.xml (despite suffix, content is HTML).
SPARK_ROW = """<tr data-icon-list-quantity="2"><th align="right" id="icon-list-icon6"></th><td><a href="/items?filter=243;401;0"><span>Spark (#401)</span></a><span class="icon-list-quantity-wrapper" data-icon-list-quantity="2">&nbsp;(<span class="icon-list-quantity">2</span>)</span></td></tr>"""
OPTIONAL = """<h2 class="heading-size-3">Optional Reagents</h2><table class="icon-list"><tr data-icon-list-quantity="1"><th></th><td><a href="/items?filter=243;391;0"><span>Add Embellishment (#391)</span></a></td></tr></table>"""


# Exact required table subset of cached spell-1237550.html, including its tree.
TREE_TABLE = """<table class="icon-list" id="icon-list-reagents">
<tr id="icon-list-reagents.3-251665" data-icon-list-quantity="40"><th align="right" id="icon-list-icon1"></th><td><a class="icon-list-tree disclosure-off" style="pointer-events:none; visibility:hidden"></a><a href="/item=251665/silverleaf-thread" class="q1">Silverleaf Thread</a><span class="icon-list-quantity-wrapper" data-icon-list-quantity="40">&nbsp;(<span class="icon-list-quantity">40</span>)</span></td></tr>
<tr id="icon-list-reagents.3-236951" data-icon-list-quantity="40"><th align="right" id="icon-list-icon2"></th><td><a class="icon-list-tree disclosure-off" style="pointer-events:none; visibility:hidden"></a><a href="/item=236951/mote-of-wild-magic" class="q3">Mote of Wild Magic</a><span class="icon-list-quantity-wrapper" data-icon-list-quantity="40">&nbsp;(<span class="icon-list-quantity">40</span>)</span></td></tr>
<tr id="icon-list-reagents.3-245345" data-icon-list-quantity="20"><th align="right" id="icon-list-icon3"></th><td><a class="icon-list-tree disclosure-off" href="javascript:" id="icon-list-tree.icon-list-reagents.3-245345" onclick="WH.IconList.toggle(this)"></a><a href="/item=245345/fused-vitality" class="q3">Fused Vitality</a><span class="icon-list-quantity-wrapper" data-icon-list-quantity="20">&nbsp;(<span class="icon-list-quantity">20</span>)</span></td></tr>
<tr id="icon-list-reagents.3-245345.6-1293502" style="display:none" data-icon-list-quantity="20"><th align="right" id="icon-list-icon4"></th><td style="padding-left:1em"><a class="icon-list-tree disclosure-off" href="javascript:" id="icon-list-tree.icon-list-reagents.3-245345.6-1293502" onclick="WH.IconList.toggle(this)"></a><a href="/spell=1293502/dnt-fused-vitality" class="q">[DNT] Fused Vitality</a><span class="icon-list-quantity-wrapper" data-icon-list-quantity="20">&nbsp;(<span class="icon-list-quantity">20</span>)</span></td></tr>
<tr id="icon-list-reagents.3-245345.6-1293502.3-274267" style="display:none" data-icon-list-quantity="40"><th align="right" id="icon-list-icon5"></th><td style="padding-left:2em"><a class="icon-list-tree disclosure-off" style="pointer-events:none; visibility:hidden"></a><a href="/item=274267/fused-vitality" class="q3">Fused Vitality</a><span class="icon-list-quantity-wrapper" data-icon-list-quantity="40">&nbsp;(<span class="icon-list-quantity">40</span>)</span></td></tr>
<tr id="icon-list-reagents.3-241281" data-icon-list-quantity="5"><th align="right" id="icon-list-icon6"></th><td><a class="icon-list-tree disclosure-off" style="pointer-events:none; visibility:hidden"></a><a href="/item=241281/composite-flora" class="q2">Composite Flora</a><span class="icon-list-quantity-wrapper" data-icon-list-quantity="5">&nbsp;(<span class="icon-list-quantity">5</span>)</span></td></tr>
<tr id="icon-list-reagents.3-244633" data-icon-list-quantity="1"><th align="right" id="icon-list-icon7"></th><td><a class="icon-list-tree disclosure-off" style="pointer-events:none; visibility:hidden"></a><a href="/item=244633/infused-scalewoven-hide" class="q3">Infused Scalewoven Hide</a><span class="icon-list-quantity-wrapper" data-icon-list-quantity="1">&nbsp;(<span class="icon-list-quantity">1</span>)</span></td></tr>
<tr><td></td><td><button class="btn btn-small fa fa-plus" onclick="WH.IconList.alterAll(&quot;icon-list-reagents&quot;, true)" style="margin-right:0.5em" type="button">Expand All</button><button class="btn btn-small fa fa-minus" onclick="WH.IconList.alterAll(&quot;icon-list-reagents&quot;, false)" style="margin-right:0.5em" type="button">Collapse All</button></td></tr></table>"""
TREE_ITEMS = [[236951, 40], [241281, 5], [244633, 1], [245345, 20], [251665, 40]]


def tree_page(table=TREE_TABLE):
    return page("", 1237550).replace('<table class="icon-list"></table>', table, 1)


def page(rows=VEST_ROWS, spell=1237528):
    return f"""<html><body><script type="application/json" id="data.page.info">{{"commentsLocked":false,"entityId":{spell},"entityType":6}}</script><h2 id="icon-list-heading-reagents" class="heading-size-3">Reagents</h2><table class="icon-list">{rows}</table>{OPTIONAL}</body></html>"""


class PublishedReagentTests(unittest.TestCase):
    def setUp(self):
        self.assertIsNotNone(adapter, "published_reagents adapter not implemented")

    def test_crafting_tree_charges_only_five_primary_items(self):
        parsed = adapter.parse_published_reagents(tree_page(), 1237550)
        self.assertEqual(
            parsed,
            {
                "spell_id": 1237550,
                "items": TREE_ITEMS,
                "unresolved": [],
                "eligible": True,
            },
        )
        self.assertNotIn(274267, [item for item, _count in parsed["items"]])

    def test_tree_does_not_omit_hidden_root_or_required_spark(self):
        table = TREE_TABLE.replace(
            'id="icon-list-reagents.3-251665" data-',
            'id="icon-list-reagents.3-251665" style="display:none" data-',
        ).replace("<tr><td></td>", SPARK_ROW + "<tr><td></td>")
        parsed = adapter.parse_published_reagents(tree_page(table), 1237550)
        self.assertEqual(parsed["items"], TREE_ITEMS)
        self.assertFalse(parsed["eligible"])
        self.assertEqual(
            parsed["unresolved"],
            [
                {
                    "row": 6,
                    "href": "/items?filter=243;401;0",
                    "text": "Spark (#401)",
                    "count": 2,
                    "reason": "non-item link",
                }
            ],
        )

    def test_tree_rejects_malformed_hierarchy_and_required_rows(self):
        lines = TREE_TABLE.splitlines()
        variants = [
            TREE_TABLE.replace('id="icon-list-reagents"', 'id="another-table"', 1),
            TREE_TABLE.replace('id="icon-list-reagents"', "", 1),
            TREE_TABLE.replace(".6-1293502.3-274267", ".6-999.3-274267"),
            TREE_TABLE.replace(".3-245345.6-1293502", ".3-245345.3-1293502"),
            TREE_TABLE.replace("/item=274267/", "/item=999/"),
            TREE_TABLE.replace("/spell=1293502/", "/currency=1293502/"),
            TREE_TABLE.replace('quantity="40">&nbsp;', 'quantity="41">&nbsp;', 1),
            TREE_TABLE.replace(
                'style="display:none" data-icon-list-quantity="20"',
                'style="display:none"',
            ),
            TREE_TABLE.replace('style="display:none"', "", 1),
            "\n".join(lines[:3] + lines[4:]),  # Missing intermediate root.
            "\n".join(lines[:4] + lines[5:]),  # Missing spell ancestor.
            "\n".join(lines[:4] + [lines[5], lines[4]] + lines[6:]),
            TREE_TABLE.replace(lines[4], lines[4] + lines[4]),
            TREE_TABLE.replace('data-icon-list-quantity="5"', "", 1),
            TREE_TABLE.replace("/item=241281/", "/item=0/"),
        ]
        for table in variants:
            with self.subTest(table=table), self.assertRaises(ValueError):
                adapter.parse_published_reagents(tree_page(table), 1237550)

    def test_tree_rejects_unknown_or_malformed_controls(self):
        variants = [
            TREE_TABLE.replace("WH.IconList.toggle(this)", "unknown(this)", 1),
            TREE_TABLE.replace(
                "icon-list-tree.icon-list-reagents.3-245345",
                "icon-list-tree.icon-list-reagents.3-999",
                1,
            ),
            TREE_TABLE.replace("visibility:hidden", "visibility:visible", 1),
            TREE_TABLE.replace(
                'visibility:hidden"></a>', 'visibility:hidden">item</a>', 1
            ),
            TREE_TABLE.replace("WH.IconList.alterAll", "unknown", 1),
            TREE_TABLE.replace("Expand All", "Unknown control"),
            TREE_TABLE.replace(
                "Collapse All</button>",
                'Collapse All</button><a href="/item=999/test">extra</a>',
            ),
            TREE_TABLE.replace("Collapse All</button>", "Collapse All</button>extra"),
            TREE_TABLE.replace(
                "Collapse All</button>", "Collapse All</button><span></span>"
            ),
            TREE_TABLE.replace(
                "<tr><td></td>", '<tr data-icon-list-quantity="1"><td></td>'
            ),
            TREE_TABLE.replace("</table>", VEST_ROWS + "</table>", 1),
        ]
        for table in variants:
            with self.subTest(table=table), self.assertRaises(ValueError):
                adapter.parse_published_reagents(tree_page(table), 1237550)

    def test_required_items_sorted_optional_ignored(self):
        parsed = adapter.parse_published_reagents(page(), 1237528)
        self.assertEqual(
            parsed,
            {
                "spell_id": 1237528,
                "items": [[238511, 15], [238513, 25]],
                "unresolved": [],
                "eligible": True,
            },
        )
        self.assertEqual(
            adapter.parse_published_reagents(
                page("\n".join(reversed(VEST_ROWS.splitlines()))), 1237528
            ),
            parsed,
        )

    def test_required_placeholder_is_not_silently_omitted(self):
        parsed = adapter.parse_published_reagents(
            page(VEST_ROWS + SPARK_ROW, 1237486), 1237486
        )
        self.assertFalse(parsed["eligible"])
        self.assertEqual(parsed["items"], [[238511, 15], [238513, 25]])
        self.assertEqual(
            parsed["unresolved"],
            [
                {
                    "row": 3,
                    "href": "/items?filter=243;401;0",
                    "text": "Spark (#401)",
                    "count": 2,
                    "reason": "non-item link",
                }
            ],
        )

    def test_non_item_link_is_unresolved(self):
        result = adapter.parse_published_reagents(
            page(SPARK_ROW.replace("/items?filter=243;401;0", "/currency=123/test")),
            1237528,
        )
        self.assertFalse(result["eligible"])
        self.assertEqual(len(result["unresolved"]), 1)
        self.assertEqual(result["items"], [])

    def test_rejects_wrong_missing_or_duplicate_identity(self):
        original = page()
        for html in [
            page(spell=1237486),
            original.replace('"entityType":6', '"entityType":3'),
            original.replace('id="data.page.info"', 'id="other"'),
            original.replace('"entityId":1237528', '"entityId":"1237528"'),
            original.replace(
                "</body>",
                '<script type="application/json" id="data.page.info">{}</script></body>',
            ),
        ]:
            with self.subTest(html=html), self.assertRaises(ValueError):
                adapter.parse_published_reagents(html, 1237528)

    def test_rejects_invalid_or_duplicate_counts_and_ids(self):
        variants = [
            VEST_ROWS + VEST_ROWS,
            VEST_ROWS.replace('quantity="15"', 'quantity="0"'),
            VEST_ROWS.replace('quantity="15"', 'quantity="-1"'),
            VEST_ROWS.replace('quantity="15"', 'quantity="4294967296"'),
            VEST_ROWS.replace('quantity="15"', 'quantity="1.5"'),
            VEST_ROWS.replace('quantity="15"', 'quantity="abc"'),
            VEST_ROWS.replace('quantity="15"', 'quantity="16"', 1),
            VEST_ROWS.replace(">15</span>", ">16</span>"),
            VEST_ROWS.replace("/item=238511/", "/item=0/"),
            VEST_ROWS.replace("/item=238511/", "/item=4294967296/"),
            VEST_ROWS.replace("/item=238511/", "/item=-1/"),
            VEST_ROWS.replace('data-icon-list-quantity="15"', "", 1),
            VEST_ROWS.replace("</td>", '<a href="/item=999/test">extra</a></td>', 1),
        ]
        for rows in variants:
            with self.subTest(rows=rows), self.assertRaises(ValueError):
                adapter.parse_published_reagents(page(rows), 1237528)

    def test_rejects_empty_missing_partial_or_malformed_table(self):
        original = page()
        variants = [
            page(""),
            original.replace('id="icon-list-heading-reagents"', 'id="other"'),
            original[: original.index("</table>")],
            original.removesuffix("</html>"),
            original.replace("</tr>", "", 1),
            original.replace("</td>", "</span>", 1),
            original.replace(
                '<table class="icon-list">',
                '<h2>Optional Reagents</h2><table class="icon-list">',
                1,
            ),
            original.replace(
                '<tr data-icon-list-quantity="15">',
                '<tr data-icon-list-quantity="15" data-icon-list-quantity="25">',
            ),
        ]
        for html in variants:
            with self.subTest(html=html), self.assertRaises(ValueError):
                adapter.parse_published_reagents(html, 1237528)

    def test_first_table_only(self):
        html = page().replace(
            OPTIONAL, '<table class="icon-list">' + SPARK_ROW + "</table>" + OPTIONAL
        )
        self.assertTrue(adapter.parse_published_reagents(html, 1237528)["eligible"])

    def test_candidate_query_uses_actual_spell_not_ability_id(self):
        with tempfile.TemporaryDirectory() as root:
            db = Path(root) / "fixture.db"
            with closing(sqlite3.connect(db)) as con:
                con.executescript(
                    "CREATE TABLE skill_line_ability(ID,Spell,SkillupSkillLineID); CREATE TABLE spell_effect(SpellID,Effect); INSERT INTO skill_line_ability VALUES(7,1237528,2915),(8,1237486,2915),(9,999,165),(10,1237528,2915),(11,555,2915); INSERT INTO spell_effect VALUES(1237528,288),(1237486,288),(999,288),(555,24);"
                )
                con.commit()
            before = db.read_bytes()
            self.assertEqual(adapter.find_spell_ids(db, 2915), [1237486, 1237528])
            self.assertEqual(db.read_bytes(), before)

    def test_fetch_identifies_ordinary_public_http_requests(self):
        from http.server import BaseHTTPRequestHandler, HTTPServer
        from threading import Thread

        class PublicPage(BaseHTTPRequestHandler):
            def do_GET(self):
                identified = self.headers.get("User-Agent", "").startswith(
                    "world-of-osso-"
                )
                html_requested = self.headers.get("Accept") == "text/html"
                self.send_response(200 if identified and html_requested else 403)
                self.end_headers()
                if identified and html_requested:
                    self.wfile.write(page().encode())

            def log_message(self, *_args):
                return

        server = HTTPServer(("127.0.0.1", 0), PublicPage)
        thread = Thread(target=server.serve_forever, daemon=True)
        thread.start()
        try:
            raw = adapter.fetch_page(
                f"http://127.0.0.1:{server.server_port}/spell=1237528"
            )
            self.assertEqual(
                adapter.parse_published_reagents(raw.decode(), 1237528)["items"],
                [[238511, 15], [238513, 25]],
            )
        finally:
            server.shutdown()
            server.server_close()
            thread.join()

    def test_fetch_retries_transient_but_not_forbidden(self):
        url = "https://www.wowhead.com/spell=1237528"
        transient = HTTPError(url, 503, "temporary", {"Retry-After": "2"}, None)
        with (
            patch.object(
                adapter, "urlopen", side_effect=[transient, BytesIO(page().encode())]
            ),
            patch.object(adapter.time, "sleep") as sleep,
        ):
            self.assertEqual(adapter.fetch_page(url), page().encode())
            self.assertGreaterEqual(sleep.call_args.args[0], 2)
        with (
            patch.object(
                adapter,
                "urlopen",
                side_effect=HTTPError(url, 403, "forbidden", {}, None),
            ),
            patch.object(adapter.time, "sleep") as sleep,
        ):
            with self.assertRaises(HTTPError):
                adapter.fetch_page(url)
            sleep.assert_not_called()
        with (
            patch.object(adapter, "urlopen", side_effect=transient) as opener,
            patch.object(adapter.time, "sleep"),
        ):
            with self.assertRaises(HTTPError):
                adapter.fetch_page(url)
            self.assertEqual(opener.call_count, 3)

    def test_saved_wrapper_cache_reuse_and_hash_integrity(self):
        with tempfile.TemporaryDirectory() as root:
            cache = Path(root)
            with patch.object(
                adapter, "fetch_page", return_value=page().encode()
            ) as fetch:
                first = adapter.load_record(cache, 1237528)
                second = adapter.load_record(cache, 1237528)
                self.assertEqual(first, second)
                self.assertEqual(fetch.call_count, 1)
            self.assertEqual(
                first["source_url"], "https://www.wowhead.com/spell=1237528"
            )
            self.assertEqual(
                first["sha256"], hashlib.sha256(page().encode()).hexdigest()
            )
            self.assertTrue(first["captured_utc"].endswith("Z"))
            self.assertEqual(first["provenance"]["kind"], "published-spell-page")
            (cache / "spell-1237528.html").write_text(page(SPARK_ROW))
            with self.assertRaises(ValueError):
                adapter.load_record(cache, 1237528)

    def test_rejects_cache_without_acquisition_metadata(self):
        with tempfile.TemporaryDirectory() as root:
            cache = Path(root)
            (cache / "spell-1237528.html").write_text(page())
            with (
                self.assertRaises(ValueError),
                patch.object(adapter, "fetch_page") as fetch,
            ):
                adapter.load_record(cache, 1237528)
            fetch.assert_not_called()

    def test_does_not_retry_permanent_transport_error(self):
        with (
            patch.object(
                adapter, "urlopen", side_effect=URLError("unknown URL scheme")
            ) as opener,
            patch.object(adapter.time, "sleep") as sleep,
        ):
            with self.assertRaises(URLError):
                adapter.fetch_page("https://www.wowhead.com/spell=1237528")
            self.assertEqual(opener.call_count, 1)
            sleep.assert_not_called()

    def test_cli_cached_records_preserve_db_and_wrapper_contract(self):
        with tempfile.TemporaryDirectory(
            dir=Path(__file__).resolve().parents[2]
        ) as root:
            root = Path(root)
            db = root / "fixture.db"
            cache = root / "cache"
            output = root / "evidence.json"
            with closing(sqlite3.connect(db)) as con:
                con.executescript(
                    "CREATE TABLE skill_line_ability(Spell,SkillupSkillLineID); CREATE TABLE spell_effect(SpellID,Effect); INSERT INTO skill_line_ability VALUES(1237528,2915),(1237486,2915); INSERT INTO spell_effect VALUES(1237528,288),(1237486,288);"
                )
                con.commit()
            before = db.read_bytes()
            with patch.object(
                adapter,
                "fetch_page",
                side_effect=[
                    page().encode(),
                    page(VEST_ROWS + SPARK_ROW, 1237486).encode(),
                ],
            ):
                adapter.load_record(cache, 1237528)
                adapter.load_record(cache, 1237486)
            argv = [
                "--db",
                str(db),
                "--skillup-line",
                "2915",
                "--cache-dir",
                str(cache),
                "--output",
                str(output),
            ]
            with patch.object(adapter, "fetch_page") as fetch:
                self.assertEqual(adapter.main(argv), 0)
                fetch.assert_not_called()
            self.assertEqual(db.read_bytes(), before)
            wrapper = json.loads(output.read_text())
            self.assertEqual(wrapper["schema_version"], 1)
            self.assertEqual(wrapper["skillup_line"], 2915)
            self.assertEqual(wrapper["effect"], 288)
            self.assertEqual(
                [r["spell_id"] for r in wrapper["records"]], [1237486, 1237528]
            )
            self.assertEqual([r["eligible"] for r in wrapper["records"]], [False, True])
            with self.assertRaises(ValueError):
                adapter.main(argv[:-1] + [str(db)])
            self.assertEqual(db.read_bytes(), before)

    def test_cli_requires_explicit_arguments_and_rejects_tmp_cache(self):
        with self.assertRaises(SystemExit), redirect_stderr(StringIO()):
            adapter.main([])
        with self.assertRaises(ValueError):
            adapter.main(
                [
                    "--db",
                    "/unused.db",
                    "--skillup-line",
                    "2915",
                    "--cache-dir",
                    "/tmp/published-cache",
                    "--output",
                    "/unused.json",
                ]
            )


if __name__ == "__main__":
    unittest.main()
