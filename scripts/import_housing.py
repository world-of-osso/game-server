#!/usr/bin/env python3
"""Import authenticated local housing DB2 exports without inventing gameplay rules.

Stores all authored columns, including unknown fields, as string-valued JSON.
Build-key-qualified records are catalogs, not ownership or acquisition grants.
The export report and audit ledger retain unavailable tables/encrypted counts.
Only the selected build is replaced; unrelated world tables remain untouched.
"""
import argparse
import csv
import hashlib
import json
from pathlib import Path
import re
import sqlite3


SCHEMA = (
    """CREATE TABLE IF NOT EXISTS housing_catalog_build (
        build_key TEXT PRIMARY KEY, build TEXT NOT NULL,
        report_sha256 TEXT NOT NULL, ledger_sha256 TEXT NOT NULL
    )""",
    """CREATE TABLE IF NOT EXISTS housing_catalog_source (
        build_key TEXT NOT NULL, table_name TEXT NOT NULL, fdid INTEGER NOT NULL,
        status TEXT NOT NULL, layout_hash TEXT, exported_records INTEGER NOT NULL,
        encrypted_records_dropped INTEGER NOT NULL, csv_sha256 TEXT,
        db2_sha256 TEXT, receipt_json TEXT NOT NULL,
        PRIMARY KEY (build_key, table_name),
        FOREIGN KEY (build_key) REFERENCES housing_catalog_build(build_key)
    )""",
    """CREATE TABLE IF NOT EXISTS housing_catalog_record (
        build_key TEXT NOT NULL, table_name TEXT NOT NULL, record_id INTEGER NOT NULL,
        fields_json TEXT NOT NULL,
        PRIMARY KEY (build_key, table_name, record_id),
        FOREIGN KEY (build_key, table_name)
            REFERENCES housing_catalog_source(build_key, table_name)
    )""",
)


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def validate_build(directory, build, build_key):
    if not re.fullmatch(r"[0-9a-f]{32}", build_key):
        raise ValueError("build key must be 32 lowercase hex digits")
    with (directory / "build-info-receipt.txt").open(encoding="utf-8") as source:
        reader = csv.reader(source, delimiter="|")
        columns = [field.split("!", 1)[0] for field in next(reader)]
        active = [dict(zip(columns, row, strict=True)) for row in reader if row]
    matches = [row for row in active if row.get("Active") == "1"
               and row.get("Product") == "wow" and row.get("Version") == build
               and row.get("Build Key") == build_key]
    if len(matches) != 1:
        raise ValueError(f"build receipt does not identify exactly one active wow {build}/{build_key}")


def load_export(directory, report, checks):
    name = report["table"]
    if not re.fullmatch(r"[A-Za-z][A-Za-z0-9_]*", name):
        raise ValueError(f"invalid housing table name: {name!r}")
    if report["status"] == "absent from local CASC":
        return None, []
    if report["status"] != "exported":
        raise ValueError(f"{name}: unsupported export status {report['status']!r}")
    check = checks[name]
    csv_path = directory / f"{name}.csv"
    db2_path = directory / "db2" / f"{report['fdid']}.db2"
    for path, field in [(csv_path, "csv_sha256"), (db2_path, "db2_sha256")]:
        if sha256(path) != check[field]:
            raise ValueError(f"{name}: SHA-256 mismatch for {path.name}")
    if check["layout_hash"] != report["layout"]:
        raise ValueError(f"{name}: layout receipt mismatch")
    with csv_path.open(newline="", encoding="utf-8") as source:
        reader = csv.DictReader(source)
        if reader.fieldnames != report["columns"] or "ID" not in reader.fieldnames:
            raise ValueError(f"{name}: columns differ from layout receipt or ID absent")
        rows = list(reader)
    if len(rows) != report["exported_records"] or len(rows) != check["csv_rows"]:
        raise ValueError(f"{name}: exported row count mismatch")
    records = []
    ids = set()
    for row in rows:
        if None in row or any(value is None for value in row.values()):
            raise ValueError(f"{name}: malformed CSV row")
        record_id = int(row["ID"])
        if record_id < 0 or record_id in ids:
            raise ValueError(f"{name}: invalid or duplicate ID {record_id}")
        ids.add(record_id)
        records.append((record_id, json.dumps(row, ensure_ascii=False, sort_keys=True)))
    return check, records


def load_catalog(directory, build, build_key):
    validate_build(directory, build, build_key)
    report_path = directory / "table-export-report.json"
    ledger_path = directory / "audit-proof-ledger.json"
    reports = json.loads(report_path.read_text(encoding="utf-8"))
    checks = {row["table"]: row for row in json.loads(
        ledger_path.read_text(encoding="utf-8"))["table_checks"]}
    sources = []
    names = set()
    for report in reports:
        if not report.get("housing"):
            continue
        name = report["table"]
        if name in names:
            raise ValueError(f"duplicate housing table receipt: {name}")
        names.add(name)
        check, records = load_export(directory, report, checks)
        sources.append((report, check, records))
    if not sources:
        raise ValueError("no housing table receipts")
    return sources, sha256(report_path), sha256(ledger_path)


def import_catalog(directory, database, build, build_key):
    sources, report_hash, ledger_hash = load_catalog(Path(directory), build, build_key)
    with sqlite3.connect(database) as db:
        db.execute("PRAGMA foreign_keys=ON")
        db.execute("BEGIN IMMEDIATE")
        for statement in SCHEMA:
            db.execute(statement)
        db.execute("DELETE FROM housing_catalog_record WHERE build_key=?", (build_key,))
        db.execute("DELETE FROM housing_catalog_source WHERE build_key=?", (build_key,))
        db.execute("DELETE FROM housing_catalog_build WHERE build_key=?", (build_key,))
        db.execute("INSERT INTO housing_catalog_build VALUES (?, ?, ?, ?)",
                   (build_key, build, report_hash, ledger_hash))
        for report, check, records in sources:
            name = report["table"]
            db.execute("INSERT INTO housing_catalog_source VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                       (build_key, name, report["fdid"], report["status"], report.get("layout"),
                        len(records), report.get("encrypted_records_dropped", 0),
                        check["csv_sha256"] if check else None,
                        check["db2_sha256"] if check else None,
                        json.dumps(report, sort_keys=True)))
            db.executemany("INSERT INTO housing_catalog_record VALUES (?, ?, ?, ?)",
                           ((build_key, name, record_id, fields) for record_id, fields in records))
    return {"build": build, "build_key": build_key, "tables": len(sources),
            "records": sum(len(records) for _, _, records in sources),
            "missing_tables": [report["table"] for report, _, _ in sources
                               if report["status"] != "exported"],
            "encrypted_records_dropped": sum(report.get("encrypted_records_dropped", 0)
                                             for report, _, _ in sources)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence", type=Path, required=True)
    parser.add_argument("--db", type=Path, required=True)
    parser.add_argument("--build", required=True)
    parser.add_argument("--build-key", required=True)
    args = parser.parse_args()
    print(json.dumps(import_catalog(args.evidence, args.db, args.build, args.build_key), indent=2))


if __name__ == "__main__":
    main()
