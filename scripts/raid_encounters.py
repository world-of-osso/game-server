"""Prepare dungeon and raid boss lifecycle SQL from pinned local CASC journal rows and TDB models.

No database is modified. A unique display-to-creature join on the encounter's map
is required. Multi-actor encounters and shared displays remain explicit gaps.
"""
import argparse
import json
import math
import sqlite3
from collections import defaultdict
from contextlib import closing
from pathlib import Path

from db2_casc import decode, read_header


def read_journal(path, layout, width, arrays=None):
    data = path.read_bytes()
    if read_header(data)["layout_hash"] != layout:
        raise ValueError(f"Unexpected journal layout in {path}")
    _, rows = decode(data, {0, 1}, inline_arrays=arrays)
    if any(len(row) != width for row in rows.values()):
        raise ValueError(f"Unexpected journal record width in {path}")
    return list(rows.values())


def derive_bindings(connection, encounters, creatures):
    actors = defaultdict(list)
    for row in creatures:
        journal = row[3] & 0xFFFF
        if journal != (row[8] & 0xFFFF):
            raise ValueError("JournalEncounterCreature relationship disagrees with inline ID")
        actors[journal].append(row)
    bindings, gaps = [], []
    for row in encounters:
        encounter_id = row[6] & 0xFFFF
        journal_id = row[4]
        encounter = connection.execute(
            "SELECT e.MapID,e.Bit,e.Name_lang FROM dungeon_encounter e "
            "JOIN map m ON m.ID=e.MapID WHERE e.ID=? AND m.InstanceType IN (1,2)",
            (encounter_id,),
        ).fetchone()
        if encounter is None:
            gaps.append(dict(map_id=None, encounter_id=encounter_id, journal_encounter_id=journal_id,
                             reason="no dungeon or raid DungeonEncounter"))
            continue
        map_id, bit, name = encounter
        participants = actors[journal_id]
        if len(participants) != 1:
            gaps.append(dict(map_id=map_id, encounter_id=encounter_id, name=name,
                             reason="multi-actor or absent journal creature", count=len(participants)))
            continue
        actor = participants[0]
        display = actor[4] & 0xFFFFFFFF
        entries = connection.execute(
            "SELECT DISTINCT c.id FROM tdb_creature c "
            "JOIN tdb_creature_template_model m ON m.CreatureID=c.id "
            "WHERE c.map=? AND m.CreatureDisplayID=?", (map_id, display),
        ).fetchall()
        if len(entries) != 1:
            gaps.append(dict(map_id=map_id, encounter_id=encounter_id, name=name,
                             reason="ambiguous or absent map/display join", count=len(entries)))
            continue
        bindings.append((map_id, entries[0][0], encounter_id, bit, journal_id, actor[2], display))
    # A creature must not complete two encounters just because the journal reuses a model.
    counts = defaultdict(set)
    for map_id, entry, encounter_id, *_ in bindings:
        counts[(map_id, entry)].add(encounter_id)
    ambiguous = {key for key, ids in counts.items() if len(ids) > 1}
    accepted = []
    for binding in bindings:
        if binding[:2] in ambiguous:
            gaps.append(dict(map_id=binding[0], encounter_id=binding[2],
                             reason="creature joins multiple encounters"))
        else:
            accepted.append(binding)
    unique = {}
    for binding in sorted(accepted):
        unique.setdefault(binding[:2], binding)
    return list(unique.values()), gaps


def add_threshold_bindings(connection, bindings, encounters, creatures, thresholds):
    """Retain sourced living completions even when no TDB spawn supplies a join."""
    by_key = {row[:2]: row for row in bindings}
    seen = set()
    for row in thresholds:
        binding = tuple(row[key] for key in (
            'map_id', 'creature_entry', 'encounter_id', 'bit',
            'journal_encounter_id', 'journal_creature_id', 'display_id'))
        map_id, entry, encounter, bit, journal, actor, display = binding
        percent = row['complete_at_health_pct']
        if binding[:2] in seen or not isinstance(percent, (int, float)) or not math.isfinite(percent) or not 0 <= percent <= 100:
            raise ValueError(f'invalid encounter threshold for {binding[:2]}')
        seen.add(binding[:2])
        if not row['policy'] or not row['evidence']:
            raise ValueError(f'missing threshold provenance for {binding[:2]}')
        if connection.execute('SELECT MapID,Bit FROM dungeon_encounter WHERE ID=?', (encounter,)).fetchone() != (map_id, bit):
            raise ValueError(f'threshold encounter identity changed for {binding[:2]}')
        if not any(item[4] == journal and (item[6] & 0xFFFF) == encounter for item in encounters):
            raise ValueError(f'threshold journal identity changed for {binding[:2]}')
        if not any(item[2] == actor and (item[3] & 0xFFFF) == journal and (item[4] & 0xFFFFFFFF) == display for item in creatures):
            raise ValueError(f'threshold journal actor changed for {binding[:2]}')
        if not connection.execute('SELECT 1 FROM tdb_creature_template_model WHERE CreatureID=? AND CreatureDisplayID=?', (entry, display)).fetchone():
            raise ValueError(f'threshold runtime model changed for {binding[:2]}')
        if binding[:2] in by_key and by_key[binding[:2]] != binding:
            raise ValueError(f'threshold binding conflicts for {binding[:2]}')
        by_key[binding[:2]] = binding
    return sorted(by_key.values())


def prepare(db, journal, creatures, output, report, build):
    encounters = read_journal(journal, 0xB88D11CC, 13, {2: 2})
    actors = read_journal(creatures, 0x81A3C769, 9)
    thresholds = json.loads((Path(__file__).resolve().parents[1] / 'content/encounter-thresholds.json').read_text())
    with closing(sqlite3.connect(db.resolve().as_uri() + "?mode=ro", uri=True)) as conn:
        bindings, gaps = derive_bindings(conn, encounters, actors)
        bindings = add_threshold_bindings(conn, bindings, encounters, actors, thresholds)
    completed = {row['encounter_id'] for row in thresholds}
    gaps = [gap for gap in gaps if gap['encounter_id'] not in completed]
    if not bindings:
        raise ValueError("No unambiguous instance boss bindings derived")
    with closing(sqlite3.connect(":memory:")) as staging:
        staging.execute("CREATE TABLE raid_encounter_creature (map_id INTEGER NOT NULL,"
                        "creature_entry INTEGER NOT NULL,encounter_id INTEGER NOT NULL,"
                        "bit INTEGER NOT NULL,journal_encounter_id INTEGER NOT NULL,"
                        "journal_creature_id INTEGER NOT NULL,display_id INTEGER NOT NULL,"
                        "complete_at_health_pct REAL CHECK(complete_at_health_pct IS NULL OR "
                        "(complete_at_health_pct >= 0 AND complete_at_health_pct <= 100)),"
                        "PRIMARY KEY(map_id,creature_entry))")
        staging.executemany("INSERT INTO raid_encounter_creature "
                            "(map_id,creature_entry,encounter_id,bit,journal_encounter_id,"
                            "journal_creature_id,display_id) VALUES(?,?,?,?,?,?,?)", bindings)
        staging.executemany(
            'UPDATE raid_encounter_creature SET complete_at_health_pct=? WHERE map_id=? AND creature_entry=?',
            [(row['complete_at_health_pct'], row['map_id'], row['creature_entry']) for row in thresholds],
        )
        statements = list(staging.iterdump())
    statements.insert(1, "DROP TABLE IF EXISTS raid_encounter_creature;")
    output.write_text("\n".join(statements) + "\n")
    report.write_text(json.dumps(dict(build=build, bindings=bindings, gaps=gaps), indent=2) + "\n")
    return bindings, gaps


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--db", type=Path, required=True)
    parser.add_argument("--journal", type=Path, required=True)
    parser.add_argument("--creatures", type=Path, required=True)
    parser.add_argument("--journal-build", required=True,
                        help="client build the journal binaries were extracted from")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()
    bindings, gaps = prepare(args.db, args.journal, args.creatures, args.output, args.report,
                             args.journal_build)
    print(f"Prepared {len(bindings)} bindings; {len(gaps)} underivable encounters recorded")


if __name__ == "__main__":
    main()
