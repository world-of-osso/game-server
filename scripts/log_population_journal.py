#!/usr/bin/env python3
"""Associate observed encounter actors with local Journal DB2 and TDB display rows."""
import argparse
import csv
import importlib.util
import json
import sqlite3
from collections import defaultdict
from pathlib import Path

spec = importlib.util.spec_from_file_location('log_population_census', Path(__file__).with_name('log_population_census.py'))
census = importlib.util.module_from_spec(spec)
spec.loader.exec_module(census)


def journal_actors(directory):
    encounters = {}
    encounters_path = directory / 'JournalEncounter.csv'
    actors_path = directory / 'JournalEncounterCreature.csv'
    with encounters_path.open(encoding='utf-8', newline='') as source:
        reader = csv.DictReader(source)
        previous_line = 1
        for row in reader:
            line = previous_line + 1
            previous_line = reader.line_num
            encounters[row['ID']] = (int(row['DungeonEncounterID']), line)
    actors = defaultdict(list)
    with actors_path.open(encoding='utf-8', newline='') as source:
        reader = csv.DictReader(source)
        previous_line = 1
        for row in reader:
            line = previous_line + 1
            previous_line = reader.line_num
            if row['JournalEncounterID'] not in encounters:
                continue
            encounter_id, encounter_line = encounters[row['JournalEncounterID']]
            actors[encounter_id].append({'name': row['Name_lang'], 'display_id': int(row['CreatureDisplayInfoID']),
                'actor_id': int(row['ID']), 'journal_encounter_id': int(row['JournalEncounterID']),
                'actor_file': str(actors_path), 'actor_line': line,
                'encounter_file': str(encounters_path), 'encounter_line': encounter_line})
    return actors


def annotate(runs, actors, db):
    with sqlite3.connect(db.resolve().as_uri() + '?mode=ro', uri=True) as conn:
        for run in runs:
            for encounter in run['encounters']:
                evidence = []
                for actor in actors.get(encounter['id'], []):
                    candidates = sorted({unit['npc_id'] for unit in encounter['units'].values()
                        if unit['name'] == actor['name'] and conn.execute(
                            'SELECT 1 FROM tdb_creature_template_model WHERE CreatureID=? AND CreatureDisplayID=?',
                            (unit['npc_id'], actor['display_id'])).fetchone()})
                    if candidates:
                        evidence.append({**actor, 'candidate_npc_ids': candidates})
                encounter['journal_actor_evidence'] = evidence
                ids = {npc for actor in evidence for npc in actor['candidate_npc_ids']}
                for cast in encounter['casts']:
                    cast['journal_actor'] = encounter['units'][cast['guid']]['npc_id'] in ids


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--census', required=True, type=Path)
    parser.add_argument('--journal-dir', required=True, type=Path)
    parser.add_argument('--world-db', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--summary', required=True, type=Path)
    args = parser.parse_args()
    with args.census.open(encoding='utf-8') as source:
        data = json.load(source)
    annotate(data['runs'], journal_actors(args.journal_dir), args.world_db)
    args.output.write_text(json.dumps(data, indent=2) + '\n')
    args.summary.write_text(json.dumps(census.summarize_encounters(data['runs']), indent=2) + '\n')


if __name__ == '__main__':
    main()
