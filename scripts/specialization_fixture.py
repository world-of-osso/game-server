#!/usr/bin/env python3
"""Copy the authentic Leather Armor Crafting tree into a hermetic SQL fixture.

Usage: python3 scripts/specialization_fixture.py --world data/world.db \
    --reference data/diagnostics/crafting-2026-10-11/reference.sqlite
Only regeneration needs the private source databases; tests use the tracked SQL.
Rows and schemas are copied without editing values or synthesizing bindings.
"""

import argparse
from contextlib import closing
from pathlib import Path
import re
import sqlite3

ROOT = Path(__file__).resolve().parent.parent
OUTPUT = ROOT / "crates/server/src/profession/specializations/fixture.sql"
TREE = 350


def read_ids(db, table, column, predicate):
    return {row[0] for row in db.execute(f'SELECT "{column}" FROM "{table}" WHERE {predicate}')}


def in_ids(column, ids):
    return f'"{column}" IN ({",".join(map(str, sorted(ids))) or "NULL"})'


def copy_rows(source, target, table, predicate):
    schema, = source.execute(
        "SELECT sql FROM sqlite_master WHERE type='table' AND name=?", (table,)
    ).fetchone()
    target.execute(schema)
    rows = source.execute(f'SELECT * FROM "{table}" WHERE {predicate} ORDER BY rowid').fetchall()
    if rows:
        placeholders = ",".join("?" for _ in rows[0])
        target.executemany(f'INSERT INTO "{table}" VALUES ({placeholders})', rows)


def copy_tree(world, fixture):
    nodes = read_ids(world, "trait_node", "ID", f"TraitTreeID={TREE}")
    groups = read_ids(world, "trait_node_group", "ID", f"TraitTreeID={TREE}")
    entries = read_ids(world, "trait_node_x_trait_node_entry", "TraitNodeEntryID", in_ids("TraitNodeID", nodes))
    definitions = read_ids(world, "trait_node_entry", "TraitDefinitionID", in_ids("ID", entries))
    currencies = read_ids(world, "trait_tree_x_trait_currency", "TraitCurrencyID", f"TraitTreeID={TREE}")
    conditions, costs = set(), set()
    for owner, ids in [("node", nodes), ("node_entry", entries), ("node_group", groups)]:
        column = "Trait" + "".join(word.title() for word in owner.split("_")) + "ID"
        for kind, selected in [("cond", conditions), ("cost", costs)]:
            table = f"trait_{owner}_x_trait_{kind}"
            predicate = in_ids(column, ids)
            selected.update(read_ids(world, table, f"Trait{kind.title()}ID", predicate))
            copy_rows(world, fixture, table, predicate)
    selections = {
        "trait_tree": f"ID={TREE}",
        "skill_line_x_trait_tree": f"TraitTreeID={TREE}",
        "trait_node": in_ids("ID", nodes),
        "trait_node_group": in_ids("ID", groups),
        "trait_node_group_x_trait_node": in_ids("TraitNodeID", nodes),
        "trait_node_x_trait_node_entry": in_ids("TraitNodeID", nodes),
        "trait_node_entry": in_ids("ID", entries),
        "trait_definition": in_ids("ID", definitions),
        "trait_edge": in_ids("LeftTraitNodeID", nodes),
        "trait_cond": in_ids("ID", conditions),
        "trait_cost": in_ids("ID", costs),
        "trait_currency": in_ids("ID", currencies),
        "trait_tree_x_trait_currency": f"TraitTreeID={TREE}",
        "trait_currency_source": in_ids("TraitCurrencyID", currencies),
        "trait_sub_tree": f"TraitTreeID={TREE}",
    }
    for table, predicate in selections.items():
        copy_rows(world, fixture, table, predicate)
    spec_sets = read_ids(fixture, "trait_cond", "SpecSetID", "SpecSetID<>0")
    copy_rows(world, fixture, "spec_set_member", in_ids("SpecSet", spec_sets))
    spells = read_ids(fixture, "trait_definition", "SpellID", "SpellID<>0")
    copy_rows(world, fixture, "spell_misc", in_ids("SpellID", spells) + " AND DifficultyID=0")
    for text, in fixture.execute("SELECT OverrideDescription_lang FROM trait_definition"):
        spells.update(int(match) for match in re.findall(r"\$@spellname(\d+)", text, re.IGNORECASE))
    copy_rows(world, fixture, "spell_name", in_ids("ID", spells))
    copy_rows(world, fixture, "tdb_quest_template", "ID=70191")
    return nodes, definitions, currencies


def copy_reference(reference, fixture, nodes, definitions, currencies):
    selections = {
        "prof_trait_tree": f"TraitTreeID={TREE}",
        "prof_trait_path_node": in_ids("TraitNodeID", nodes),
        "prof_trait_perk_node": in_ids("TraitNodeID", nodes),
        "profession_trait": in_ids("TraitDefinitionID", definitions),
    }
    for table, predicate in selections.items():
        copy_rows(reference, fixture, table, predicate)
    traits = read_ids(fixture, "profession_trait", "ID", "1")
    copy_rows(reference, fixture, "profession_trait_x_effect", in_ids("ProfessionTraitID", traits))
    effects = read_ids(fixture, "profession_trait_x_effect", "ProfessionEffectID", "1")
    copy_rows(reference, fixture, "profession_effect", in_ids("ID", effects))
    types = read_ids(fixture, "profession_effect", "ProfessionEffectTypeEnumID", "1")
    copy_rows(reference, fixture, "profession_effect_type", in_ids("EnumID", types))
    wallet_ids = read_ids(fixture, "trait_currency", "CurrencyTypesID", in_ids("ID", currencies)) - {0}
    copy_rows(reference, fixture, "currency_types", in_ids("ID", wallet_ids))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--world", type=Path, required=True)
    parser.add_argument("--reference", type=Path, required=True)
    parser.add_argument("--output", type=Path, default=OUTPUT)
    args = parser.parse_args()
    with closing(sqlite3.connect(args.world.resolve().as_uri() + "?mode=ro", uri=True)) as world, closing(
        sqlite3.connect(args.reference.resolve().as_uri() + "?mode=ro", uri=True)
    ) as reference, closing(sqlite3.connect(":memory:")) as fixture:
        nodes, definitions, currencies = copy_tree(world, fixture)
        copy_reference(reference, fixture, nodes, definitions, currencies)
        args.output.write_text(
            "-- Generated by scripts/specialization_fixture.py; do not edit.\n"
            "-- Authentic world.db + crafting-2026-10-11/reference.sqlite rows.\n"
            "-- Tree 350 / skill line 2830; quest 70191. No runtime source DB required.\n"
            + "\n".join(fixture.iterdump()) + "\n"
        )


if __name__ == "__main__":
    main()
