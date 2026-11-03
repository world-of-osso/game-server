#!/usr/bin/env python3
"""Emit guarded SQL for reviewed WCL creature abilities and empirical timer samples."""
import argparse
import json
import re
from pathlib import Path


def integer_delay(value, *, repeat):
    if isinstance(value, bool) or not isinstance(value, (int, float)) or int(value) != value:
        raise ValueError(f'non-integral delay: {value!r}')
    result = int(value)
    if result < (1 if repeat else 0) or result > 0xffffffff:
        raise ValueError(f'invalid {"successive_cast" if repeat else "first_activity_proxy"} delay_ms={result}')
    return result


def render_sql(map_id, abilities, samples):
    indexed = {(row['entry'], row['spell']): row for row in samples}
    lines = [
        '-- INFERRED_WCL; first-activity delay is a proxy, not proven combat entry.',
        '-- Stop and roll back on any failure.',
        'BEGIN IMMEDIATE;',
        'CREATE TEMP TABLE _wcl_guard(ok INTEGER NOT NULL CHECK(ok=1));',
        "CREATE TEMP TABLE _wcl_sample(map INTEGER NOT NULL,difficulty INTEGER NOT NULL,entry INTEGER NOT NULL,spell INTEGER NOT NULL,sample_id INTEGER NOT NULL,timing_kind TEXT NOT NULL CHECK(timing_kind IN ('first_activity_proxy','successive_cast')),delay_ms INTEGER NOT NULL CHECK(delay_ms>=0 AND delay_ms<=4294967295 AND (timing_kind!='successive_cast' OR delay_ms>0)),report TEXT NOT NULL,fight INTEGER NOT NULL,source_actor INTEGER NOT NULL,source_instance INTEGER NOT NULL);",
    ]
    seen = set()
    for ability in abilities:
        entry, spell, target = (int(ability[key]) for key in ('entry', 'spell', 'target_type'))
        if (entry, spell) in seen or target not in (1, 2, 5, 6):
            raise ValueError(f'duplicate ability or unsupported target: {entry}/{spell}/{target}')
        seen.add((entry, spell))
        row = indexed[(entry, spell)]
        if not row['delay_proxy_ms'] or not row['interval_ms']:
            raise ValueError(f'ability {entry}/{spell} requires first and repeat samples')
        lines.append(f"INSERT INTO _wcl_guard SELECT CASE WHEN EXISTS(SELECT 1 FROM content_creature_template WHERE entry={entry} AND AIName='' AND ScriptName='') AND NOT EXISTS(SELECT 1 FROM tdb_smart_scripts WHERE entryorguid={entry} AND source_type=0) AND EXISTS(SELECT 1 FROM spell_effect WHERE SpellID={spell}) AND NOT EXISTS(SELECT 1 FROM content_inferred_creature_ability WHERE map={map_id} AND difficulty IN (8,23) AND entry={entry} AND spell={spell}) THEN 1 ELSE 0 END;")
        for difficulty in (8, 23):
            lines.append(f"INSERT INTO content_inferred_creature_ability(map,difficulty,entry,spell,target_type,provenance,retirement_condition) VALUES({map_id},{difficulty},{entry},{spell},{target},'INFERRED_WCL','Replace with authoritative current-build creature behavior when validated.');")
            sample_id = 0
            for kind, rows, field in [('first_activity_proxy', row['delay_proxy_ms'], 'delay_ms'), ('successive_cast', row['interval_ms'], 'interval_ms')]:
                for sample in rows:
                    sample_id += 1
                    delay = integer_delay(sample[field], repeat=kind == 'successive_cast')
                    report, fight = sample['report'].split('/')
                    if not re.fullmatch(r'[A-Za-z0-9]+', report):
                        raise ValueError(f'invalid report code {report!r}')
                    actor, instance = sample['actor']
                    lines.append(f"INSERT INTO _wcl_sample VALUES({map_id},{difficulty},{entry},{spell},{sample_id},'{kind}',{delay},'{report}',{int(fight)},{int(actor)},{int(instance)});")
    lines.extend(['INSERT INTO content_inferred_creature_ability_sample SELECT * FROM _wcl_sample;', 'DROP TABLE _wcl_sample;', 'DROP TABLE _wcl_guard;', 'COMMIT;'])
    return '\n'.join(lines) + '\n'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--map', type=int, required=True)
    parser.add_argument('--abilities', type=Path, required=True)
    parser.add_argument('--samples', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    abilities = json.loads(args.abilities.read_text())
    samples = json.loads(args.samples.read_text())['spells']
    args.output.write_text(render_sql(args.map, abilities, samples))


if __name__ == '__main__':
    main()
