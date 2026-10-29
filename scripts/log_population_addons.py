#!/usr/bin/env python3
"""Index addon NPC/ability associations as text; never execute addon code."""
import argparse
import json
import re
from collections import defaultdict
from pathlib import Path


def read_traits(path):
    entries = {}
    current = None
    with path.open(encoding='utf-8') as source:
        for line_number, line in enumerate(source, 1):
            match = re.fullmatch(r'\s*npcID = (\d+),\s*', line)
            if match:
                npc = int(match.group(1))
                current = {'source': str(path), 'line': line_number}
                entries.setdefault(npc, []).append(current)
                continue
            if current is None:
                continue
            if line.strip() == '},':
                current = None
                continue
            match = re.fullmatch(r'\s*(spellIDs|coPresenceNPCIDs) = (\{.*\}),\s*', line)
            if match:
                current[match.group(1)] = [int(value) for value in re.findall(r'\d+', match.group(2))]
            if re.fullmatch(r'\s*bossEncounter = true,\s*', line):
                current['bossEncounter'] = True
    return entries


def read_factory(path):
    entries = defaultdict(list)
    pending = None
    with path.open(encoding='utf-8') as source:
        for line_number, line in enumerate(source, 1):
            match = re.fullmatch(r'\s*\["sourceNPCIDs"\] = \{([\d, ]*)\},\s*', line)
            if match:
                pending = {'npc_ids': [int(value) for value in re.findall(r'\d+', match.group(1))],
                           'source': str(path), 'line': line_number}
                continue
            if pending is None:
                continue
            match = re.fullmatch(r'\s*\["sourceType"\] = "([^"]*)",\s*', line)
            if match:
                pending['source_type'] = match.group(1)
            match = re.fullmatch(r'\s*\["spellID"\] = (\d+),\s*', line)
            if match:
                pending['spell_id'] = int(match.group(1))
                for npc in pending['npc_ids']:
                    entries[npc].append({key: value for key, value in pending.items() if key != 'npc_ids'})
                pending = None
            elif line.strip() == '},':
                pending = None
    return dict(entries)


def annotate(runs, traits, factory):
    for run in runs:
        for creature in run['creatures'].values():
            npc = creature['npc_id']
            if npc in traits:
                creature['addon_traits'] = traits[npc]
            if npc in factory:
                creature['addon_factory_associations'] = factory[npc]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--census', required=True, type=Path)
    parser.add_argument('--traits', required=True, type=Path)
    parser.add_argument('--factory', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    with args.census.open(encoding='utf-8') as source:
        data = json.load(source)
    annotate(data['runs'], read_traits(args.traits), read_factory(args.factory))
    args.output.write_text(json.dumps(data, indent=2) + '\n')


if __name__ == '__main__':
    main()
