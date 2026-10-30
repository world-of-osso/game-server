"""Resolve journal encounter participants without granting them boss credit."""
import csv
from collections import defaultdict


def encounter_actor_bits(graph, db, journal_csv):
    bits_by_journal = defaultdict(set)
    bound = {}
    for journal, bit, entry in db.execute(
        'SELECT journal_encounter_id,bit,creature_entry FROM raid_encounter_creature WHERE map_id=?',
        (graph['map_id'],),
    ):
        bits_by_journal[journal].add(bit)
        bound[entry] = bit
    displays = defaultdict(set)
    with journal_csv.open(newline='') as stream:
        for row in csv.DictReader(stream):
            journal = int(row['JournalEncounterID'])
            display = int(row['CreatureDisplayInfoID'])
            if journal in bits_by_journal and display:
                displays[display].update(bits_by_journal[journal])
    entries_by_display = defaultdict(set)
    for display, entry in db.execute(
        'SELECT DISTINCT m.CreatureDisplayID,c.id FROM tdb_creature c '
        'JOIN tdb_creature_template_model m ON m.CreatureID=c.id WHERE c.map=?',
        (graph['map_id'],),
    ):
        if display in displays:
            entries_by_display[display].add(entry)
    bits_by_entry = defaultdict(set)
    for display, bits in displays.items():
        entries = entries_by_display[display]
        if len(bits) == 1 and len(entries) == 1:
            bits_by_entry[next(iter(entries))].update(bits)
    derived = {entry: next(iter(bits)) for entry, bits in bits_by_entry.items() if len(bits) == 1}
    return derived | bound
