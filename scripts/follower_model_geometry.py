#!/usr/bin/env python3
"""Emit only missing, used native TDB model-info rows. Input world.db stays read-only."""
import argparse
import hashlib
import json
from pathlib import Path
import sqlite3

from encounter_model_geometry import SCHEMA, read_model_info


def plan(database, source_rows):
    used = {row[0] for row in database.execute('SELECT DISTINCT CreatureDisplayID FROM content_creature_template_model WHERE CreatureDisplayID != 0')}
    existing = {row[0] for row in database.execute('SELECT DisplayID FROM encounter_creature_model_info')}
    source = {row[0]: row for row in source_rows}
    missing = used - existing
    return [source[display] for display in sorted(missing & source.keys())], sorted(missing - source.keys())


def render_sql(rows):
    inserts = ['INSERT INTO encounter_creature_model_info VALUES (' + ','.join(str(value) for value in row) + ');' for row in rows]
    return '\n'.join(['BEGIN;', *inserts, 'COMMIT;', ''])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--database', type=Path, required=True)
    parser.add_argument('--source-dump', type=Path, required=True)
    parser.add_argument('--sql-output', type=Path, required=True)
    parser.add_argument('--receipt', type=Path, required=True)
    args = parser.parse_args()
    inputs = [args.database.resolve(strict=True), args.source_dump.resolve(strict=True)]
    outputs = [args.sql_output.resolve(), args.receipt.resolve()]
    if len(set(outputs)) != 2 or any(path in inputs or path.exists() for path in outputs):
        parser.error('outputs must be new, distinct files, never either source input')
    rows = read_model_info(inputs[1])
    with sqlite3.connect(f'file:{inputs[0]}?mode=ro', uri=True) as database:
        additions, gaps = plan(database, rows)
        followers = [dict(spec=spec, entry=entry, display=display) for spec, entry, display in database.execute('SELECT p.spec_id,p.entry,m.CreatureDisplayID FROM follower_profiles p JOIN content_creature_template_model m ON m.CreatureID=p.entry') if display in gaps]
    with inputs[1].open('rb') as stream:
        digest = hashlib.file_digest(stream, 'sha256').hexdigest()
    receipt = dict(source=str(inputs[1]), sha256=digest, source_rows=len(rows), additions=additions,
                   missing_used_displays_without_source=gaps, follower_source_gaps=followers,
                   policy='No invented model info. FOLLOWER_PLAYER_RACE_MODEL_REACH_TC_PLAYER_DEFAULTS is a separate production decision, not a TDB row.')
    for path in outputs:
        path.parent.mkdir(parents=True, exist_ok=True)
    args.sql_output.write_text(render_sql(additions))
    args.receipt.write_text(json.dumps(receipt, indent=2) + '\n')
    print(json.dumps(dict(additions=len(additions), used_source_gaps=len(gaps), follower_source_gaps=followers)))


if __name__ == '__main__':
    main()
