"""Publish selected prepared kit intervals through stable native keys, COPY only.

The prepared DB must come from follower_spec_kits.py. This delta updates existing
intervals only; changed spellbook interval identities require requalification.
"""
import argparse
from contextlib import closing
import hashlib
import json
from pathlib import Path
import sqlite3


COLUMNS = ('spec_id', 'spell_id', 'min_level', 'max_level', 'role_slot',
           'priority', 'conditions', 'policy')


def digest(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def open_readonly(path):
    return sqlite3.connect(path.resolve().as_uri() + '?mode=ro', uri=True)


def read_rows(db):
    return {row[:3]: row for row in db.execute(
        'SELECT ' + ','.join(COLUMNS) + ' FROM follower_spec_kit '
        'ORDER BY spec_id,spell_id,min_level')}


def make_delta(db, original, desired, specs):
    before = {key: row for key, row in original.items() if key[0] in specs}
    after = {key: row for key, row in desired.items() if key[0] in specs}
    if before.keys() != after.keys():
        raise ValueError('kit interval keys differ; requalify the source spellbook')
    if {key[0] for key in after} != set(specs):
        raise ValueError('requested specialization has no prepared kit intervals')
    quote = lambda value: db.execute('SELECT quote(?)', (value,)).fetchone()[0]
    statements = []
    changed_keys = []
    for key, row in after.items():
        if before[key] == row:
            continue
        values = ','.join(name + '=' + quote(value)
                          for name, value in zip(COLUMNS[3:], row[3:]))
        where = ' AND '.join(name + '=' + quote(value)
                             for name, value in zip(COLUMNS[:3], key))
        statements.append(f'UPDATE follower_spec_kit SET {values} WHERE {where};')
        changed_keys.append(key)
    return 'BEGIN IMMEDIATE;\n' + '\n'.join(statements) + '\nCOMMIT;\n', changed_keys


def validate_paths(args):
    inputs = [args.world_db.resolve(), args.prepared_db.resolve()]
    outputs = [args.output_db.resolve(), args.sql.resolve(), args.receipt.resolve()]
    if len(set(outputs)) != 3 or any(path in inputs or path.exists() for path in outputs):
        raise ValueError('refusing existing, overlapping or input output path')
    return outputs


def backup_and_patch(source, path, sql, expected):
    with closing(sqlite3.connect(path)) as output:
        source.backup(output)
        output.executescript(sql)
        if read_rows(output) != expected:
            raise ValueError('delta replay did not preserve the expected complete kit state')


def prepare(args):
    outputs = validate_paths(args)
    specs = sorted(set(args.specs))
    before_digest = digest(args.world_db)
    with closing(open_readonly(args.world_db)) as source, closing(open_readonly(args.prepared_db)) as prepared:
        original, desired = read_rows(source), read_rows(prepared)
        sql, changed = make_delta(source, original, desired, specs)
        expected = {key: desired[key] if key[0] in specs else row
                    for key, row in original.items()}
        for path in outputs:
            path.parent.mkdir(parents=True, exist_ok=True)
        backup_and_patch(source, args.output_db, sql, expected)
    if digest(args.world_db) != before_digest:
        raise ValueError('source database changed; discard the output copy')
    args.sql.write_text(sql)
    receipt = dict(producer=str(Path(__file__).resolve()), producer_sha256=digest(__file__),
                   source_database=str(args.world_db.resolve()), source_database_sha256=before_digest,
                   prepared_database=str(args.prepared_db.resolve()), prepared_database_sha256=digest(args.prepared_db),
                   output_database=str(args.output_db.resolve()), output_database_sha256=digest(args.output_db),
                   sql=str(args.sql.resolve()), sql_sha256=digest(args.sql), specs=specs,
                   changed_specs=sorted({key[0] for key in changed}), changed_interval_keys=changed,
                   changed_tables=['follower_spec_kit'] if changed else [],
                   unchanged_other_specs=True)
    args.receipt.write_text(json.dumps(receipt, indent=2, sort_keys=True) + '\n')
    print(f'Prepared {len(changed)} stable-key kit updates in COPY {args.output_db}')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('world-db', 'prepared-db', 'output-db', 'sql', 'receipt'):
        parser.add_argument('--' + name, type=Path, required=True)
    parser.add_argument('--specs', type=int, nargs='+', required=True)
    prepare(parser.parse_args())


if __name__ == '__main__':
    main()
