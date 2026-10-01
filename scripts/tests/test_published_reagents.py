"""Published Wowhead reagent contracts; concrete subsets of cached spell pages."""

import hashlib
import importlib.util
import json
from pathlib import Path
import sqlite3
import sys
import tempfile
import unittest
from unittest.mock import patch
from urllib.error import HTTPError
from io import BytesIO

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


def page(rows=VEST_ROWS, spell=1237528):
    return f"""<html><body><script type="application/json" id="data.page.info">{{"commentsLocked":false,"entityId":{spell},"entityType":6}}</script><h2 id="icon-list-heading-reagents" class="heading-size-3">Reagents</h2><table class="icon-list">{rows}</table>{OPTIONAL}</body></html>"""


class PublishedReagentTests(unittest.TestCase):
    def setUp(self):
        self.assertIsNotNone(adapter, "published_reagents adapter not implemented")

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
            with sqlite3.connect(db) as con:
                con.executescript(
                    "CREATE TABLE skill_line_ability(ID,Spell,SkillupSkillLineID); CREATE TABLE spell_effect(SpellID,Effect); INSERT INTO skill_line_ability VALUES(7,1237528,2915),(8,1237486,2915),(9,999,165),(10,1237528,2915),(11,555,2915); INSERT INTO spell_effect VALUES(1237528,288),(1237486,288),(999,288),(555,24);"
                )
            before = db.read_bytes()
            self.assertEqual(adapter.find_spell_ids(db, 2915), [1237486, 1237528])
            self.assertEqual(db.read_bytes(), before)

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

    def test_cli_requires_explicit_arguments_and_rejects_tmp_cache(self):
        with self.assertRaises(SystemExit):
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
