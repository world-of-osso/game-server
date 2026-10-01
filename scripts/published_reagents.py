#!/usr/bin/env python3
"""Explicit published-page reagent evidence, never native DB2 or catalog fallback.

parse_published_reagents returns spell_id, sorted [item_id, count] pairs,
unresolved row details, and eligible (only completeness of this published table).
Invalid identity/structure/counts raise ValueError. CLI writes schema_version=1,
skillup_line, effect=288, and records containing that result plus source_url,
sha256, captured_utc, provenance. DB2 crosschecks remain the consumer's job.
"""

import argparse
from contextlib import closing
from datetime import datetime, timezone
from email.utils import parsedate_to_datetime
import hashlib
from html.parser import HTMLParser
import json
from pathlib import Path
import random
import re
import socket
import sqlite3
import time
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen

U32_MAX = 4294967295
VOID_TAGS = {
    "area",
    "base",
    "br",
    "col",
    "embed",
    "hr",
    "img",
    "input",
    "link",
    "meta",
    "param",
    "source",
    "track",
    "wbr",
}


def positive_u32(value, label):
    if not re.fullmatch(r"[0-9]+", str(value)) or not 0 < int(value) <= U32_MAX:
        raise ValueError(f"{label} must be a positive u32: {value!r}")
    return int(value)


class ReagentParser(HTMLParser):
    """Strict only inside the identified required table; unrelated HTML is ignored."""

    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.identity = None
        self.identity_text = None
        self.heading_seen = False
        self.waiting_table = False
        self.table_done = False
        self.stack = []
        self.rows = []
        self.row = None
        self.quantity_text = None
        self.html_closed = False

    def handle_starttag(self, tag, attrs):
        attributes = dict(attrs)
        if self.identity_text is not None:
            raise ValueError("Nested page identity markup")
        if tag == "script" and attributes.get("id") == "data.page.info":
            if (
                self.identity is not None
                or attributes.get("type") != "application/json"
            ):
                raise ValueError("Duplicate or non-JSON page identity")
            self.identity_text = ""
        if tag == "h2":
            if self.waiting_table:
                raise ValueError("Required heading has no table before next section")
            if attributes.get("id") == "icon-list-heading-reagents":
                if self.heading_seen:
                    raise ValueError("Duplicate required heading")
                self.heading_seen = True
                self.waiting_table = True
        if (
            tag == "table"
            and self.waiting_table
            and "icon-list" in attributes.get("class", "").split()
        ):
            self.waiting_table = False
            self.stack = ["table"]
            return
        if not self.stack:
            return
        if len(attributes) != len(attrs):
            raise ValueError("Duplicate attributes in required table")
        if tag == "table":
            raise ValueError("Nested required table")
        if tag == "tr":
            if self.row is not None or self.stack[-1] not in {"table", "tbody"}:
                raise ValueError("Malformed reagent row")
            self.row = {
                "count": positive_u32(
                    attributes.get("data-icon-list-quantity"), "row count"
                ),
                "links": [],
                "quantities": [],
            }
        elif self.row is None and tag not in {"tbody"}:
            raise ValueError("Content outside reagent row")
        self.capture_row_tag(tag, attributes)
        if tag not in VOID_TAGS:
            self.stack.append(tag)

    def capture_row_tag(self, tag, attrs):
        if self.row is None:
            return
        if tag == "a":
            self.row["links"].append({"href": attrs.get("href", ""), "text": ""})
        if tag != "tr" and "data-icon-list-quantity" in attrs:
            self.row["quantities"].append(
                positive_u32(attrs["data-icon-list-quantity"], "display count")
            )
        if "icon-list-quantity" in attrs.get("class", "").split():
            if self.quantity_text is not None:
                raise ValueError("Nested quantity display")
            self.quantity_text = ""

    def handle_data(self, data):
        if self.identity_text is not None:
            self.identity_text += data
        if self.row is not None and "a" in self.stack:
            self.row["links"][-1]["text"] += data
        if self.quantity_text is not None:
            self.quantity_text += data

    def handle_endtag(self, tag):
        if tag == "html":
            self.html_closed = True
        if tag == "script" and self.identity_text is not None:
            try:
                self.identity = json.loads(self.identity_text)
            except json.JSONDecodeError as error:
                raise ValueError("Invalid page identity JSON") from error
            self.identity_text = None
        if not self.stack:
            return
        if self.stack[-1] != tag:
            raise ValueError(f"Unbalanced required table: closing {tag}")
        if tag == "span" and self.quantity_text is not None:
            self.row["quantities"].append(
                positive_u32(self.quantity_text.strip(), "visible count")
            )
            self.quantity_text = None
        if tag == "tr":
            self.finish_row()
        self.stack.pop()
        if tag == "table":
            self.table_done = True

    def finish_row(self):
        if len(self.row["links"]) != 1:
            raise ValueError("Required row must contain exactly one reagent link")
        if any(count != self.row["count"] for count in self.row["quantities"]):
            raise ValueError("Conflicting required row counts")
        self.rows.append(self.row)
        self.row = None


def parse_published_reagents(html, spell_id) -> dict:
    spell_id = positive_u32(spell_id, "spell ID")
    parser = ReagentParser()
    parser.feed(html)
    parser.close()
    info = parser.identity
    if (
        not isinstance(info, dict)
        or type(info.get("entityType")) is not int
        or type(info.get("entityId")) is not int
        or info.get("entityType") != 6
        or info.get("entityId") != spell_id
    ):
        raise ValueError(f"Page does not identify spell {spell_id}")
    if (
        not parser.html_closed
        or parser.identity_text is not None
        or parser.stack
        or not parser.table_done
        or not parser.rows
    ):
        raise ValueError("Missing, empty or partial required reagent table/page")
    items = {}
    unresolved = []
    for index, row in enumerate(parser.rows, 1):
        link = row["links"][0]
        match = re.fullmatch(r"/item=([0-9]+)(?:/[^?#]*)?", link["href"])
        if match:
            item_id = positive_u32(match[1], "item ID")
            if item_id in items:
                raise ValueError(f"Duplicate required item {item_id}")
            items[item_id] = row["count"]
        elif link["href"].startswith("/item="):
            raise ValueError(f"Invalid item link: {link['href']}")
        else:
            unresolved.append(
                {
                    "row": index,
                    "href": link["href"],
                    "text": link["text"].strip(),
                    "count": row["count"],
                    "reason": "non-item link",
                }
            )
    return {
        "spell_id": spell_id,
        "items": [[item, items[item]] for item in sorted(items)],
        "unresolved": unresolved,
        "eligible": not unresolved,
    }


def find_spell_ids(db, skillup_line):
    skillup_line = positive_u32(skillup_line, "skillup line")
    uri = Path(db).resolve().as_uri() + "?mode=ro"
    with closing(sqlite3.connect(uri, uri=True)) as con:
        rows = con.execute(
            """SELECT DISTINCT a.Spell FROM skill_line_ability a
            JOIN spell_effect e ON e.SpellID = a.Spell
            WHERE e.Effect = 288 AND a.SkillupSkillLineID = ? ORDER BY a.Spell""",
            (skillup_line,),
        )
        return [positive_u32(row[0], "candidate spell ID") for row in rows]


def retry_delay(attempt, retry_after):
    delay = 2**attempt + random.uniform(0, 0.5)
    if not retry_after:
        return delay
    try:
        seconds = int(retry_after)
    except ValueError:
        seconds = (
            parsedate_to_datetime(retry_after) - datetime.now(timezone.utc)
        ).total_seconds()
    return max(delay, seconds)


def fetch_page(url):
    """Three ordinary GET attempts; no 403 retries or access-control evasion."""
    for attempt in range(3):
        try:
            with urlopen(Request(url, method="GET"), timeout=30) as response:
                return response.read()
        except HTTPError as error:
            if error.code not in {408, 429, 500, 502, 503, 504} or attempt == 2:
                raise
            delay = retry_delay(attempt, error.headers.get("Retry-After"))
        except URLError as error:
            reason = error.reason
            transient = isinstance(reason, (TimeoutError, ConnectionError)) or (
                isinstance(reason, socket.gaierror) and reason.errno == socket.EAI_AGAIN
            )
            if not transient or attempt == 2:
                raise
            delay = retry_delay(attempt, None)
        except (TimeoutError, ConnectionError):
            if attempt == 2:
                raise
            delay = retry_delay(attempt, None)
        # Do not retry earlier than Retry-After or block indefinitely on it.
        if delay > 120:
            raise ValueError(f"Retry-After exceeds bounded wait: {delay} seconds")
        time.sleep(delay)
    raise AssertionError("Unreachable retry state")


def utc_timestamp():
    return datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")


def load_record(cache_dir, spell_id):
    """Cache HTML + acquisition metadata; never invent provenance for raw files."""
    spell_id = positive_u32(spell_id, "spell ID")
    cache = Path(cache_dir)
    cache.mkdir(parents=True, exist_ok=True)
    page_path = cache / f"spell-{spell_id}.html"
    metadata_path = cache / f"spell-{spell_id}.json"
    source_url = f"https://www.wowhead.com/spell={spell_id}"
    if page_path.exists():
        raw = page_path.read_bytes()
        if not metadata_path.exists():
            raise ValueError(f"Missing acquisition metadata: {metadata_path}")
        metadata = json.loads(metadata_path.read_text())
    else:
        raw = fetch_page(source_url)
        metadata = {
            "source_url": source_url,
            "sha256": hashlib.sha256(raw).hexdigest(),
            "captured_utc": utc_timestamp(),
            "provenance": {
                "kind": "published-spell-page",
                "capture_method": "http-get",
                "timestamp_basis": "retrieval-time",
                "native_capture": False,
            },
        }
        page_path.write_bytes(raw)
        metadata_path.write_text(json.dumps(metadata, indent=2) + "\n")
    if (
        metadata["sha256"] != hashlib.sha256(raw).hexdigest()
        or metadata["source_url"] != source_url
    ):
        raise ValueError(f"Cache source/hash mismatch for spell {spell_id}")
    parsed = parse_published_reagents(raw.decode("utf-8"), spell_id)
    timestamp = metadata["captured_utc"]
    if not isinstance(timestamp, str) or not timestamp.endswith("Z"):
        raise ValueError("Cache requires a UTC acquisition timestamp")
    datetime.fromisoformat(timestamp)
    provenance = metadata["provenance"]
    if (
        provenance.get("kind") != "published-spell-page"
        or provenance.get("native_capture") is not False
    ):
        raise ValueError("Cache is not published-page evidence")
    return {
        **parsed,
        "source_url": source_url,
        "sha256": metadata["sha256"],
        "captured_utc": timestamp,
        "provenance": provenance,
    }


def main(argv=None):
    args_parser = argparse.ArgumentParser(description=__doc__)
    args_parser.add_argument("--db", required=True, type=Path)
    args_parser.add_argument("--skillup-line", required=True, type=int)
    args_parser.add_argument("--cache-dir", required=True, type=Path)
    args_parser.add_argument("--output", required=True, type=Path)
    args = args_parser.parse_args(argv)
    cache = args.cache_dir.resolve()
    if cache.is_relative_to("/tmp") or cache.is_relative_to("/var/tmp"):
        raise ValueError("Cache must be persistent, outside /tmp and /var/tmp")
    if args.output.resolve() == args.db.resolve() or (
        args.output.exists() and args.output.samefile(args.db)
    ):
        raise ValueError("Output must not overwrite the source database")
    spell_ids = find_spell_ids(args.db, args.skillup_line)
    records = [load_record(cache, spell_id) for spell_id in spell_ids]
    wrapper = {
        "schema_version": 1,
        "skillup_line": args.skillup_line,
        "effect": 288,
        "records": records,
    }
    args.output.write_text(json.dumps(wrapper, indent=2, sort_keys=True) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
