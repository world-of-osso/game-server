#!/usr/bin/env python3
"""Measure how AzerothCore (Wrath 3.3.5) content IDs map onto Retail DB2 IDs.

  content_mapping.py [--db data/world.db] [--retail DIR] [--wrath DIR]

--db must hold the imported AzerothCore content tables (import-azerothcore.sh)
and the Retail skill_line_ability table (db2.py import). --retail and --wrath are
wago.tools CSV exports (default data/db2/<build>/): Retail 12.1.0.69933 and Wrath
Classic 3.4.0.47659, the latter standing in for 3.3.5 DBCs that AzerothCore reads
from client files (its *_dbc.sql tables are empty override tables).

Each ID set is split into: same id + same name in Retail; same id, different
name (case-only or other); id absent but the name exists in Retail (unique /
ambiguous); unmatched.
"""

import argparse
import csv
import sqlite3
from collections import Counter, defaultdict
from pathlib import Path

PROJECT_DIR = Path(__file__).resolve().parent.parent
DEFAULT_RETAIL = PROJECT_DIR / "data/db2/12.1.0.69933"
DEFAULT_WRATH = PROJECT_DIR / "data/db2/3.4.0.47659"
EXAMPLES = 5


def read_csv(path, key="ID"):
    with path.open(newline="", encoding="utf-8") as handle:
        return {int(row[key]): row for row in csv.DictReader(handle)}


def classify(ids, source_names, retail_names):
    """Split ids by how their source name matches Retail; returns {category: [(id, detail)]}."""
    by_name = defaultdict(list)
    for retail_id, name in retail_names.items():
        by_name[name].append(retail_id)
    out = defaultdict(list)
    for source_id in sorted(ids):
        name = source_names.get(source_id)
        if name is None:
            out["no source name"].append((source_id, ""))
        elif retail_names.get(source_id) == name:
            out["same id, same name"].append((source_id, name))
        elif source_id in retail_names and retail_names[source_id].casefold() == name.casefold():
            out["same id, name differs only in case"].append((source_id, f"{name!r} -> {retail_names[source_id]!r}"))
        elif source_id in retail_names:
            out["same id, different name"].append((source_id, f"{name!r} -> {retail_names[source_id]!r}"))
        elif len(by_name.get(name, [])) == 1:
            out["id absent, name unique in Retail"].append((source_id, f"{name!r} -> {by_name[name][0]}"))
        elif by_name.get(name):
            out["id absent, name ambiguous in Retail"].append((source_id, f"{name!r} -> {len(by_name[name])} ids"))
        else:
            out["unmatched"].append((source_id, name))
    return out


def print_split(title, split):
    total = sum(len(rows) for rows in split.values())
    print(f"\n### {title} ({total})")
    for category, rows in split.items():
        examples = "; ".join(f"{i} {d}".strip() for i, d in rows[:EXAMPLES])
        print(f"- {category}: {len(rows)} ({100 * len(rows) / total:.1f}%) e.g. {examples}")


def column_set(conn, sql):
    return {row[0] for row in conn.execute(sql) if row[0]}


def report_items(conn, retail):
    sparse = read_csv(retail / "ItemSparse.csv")
    item = read_csv(retail / "Item.csv")
    retail_names = {i: row["Display_lang"] for i, row in sparse.items()}
    source = {row[0]: row for row in conn.execute(
        "SELECT entry, name, class, subclass, InventoryType, Quality, displayid, BuyPrice, SellPrice, ItemLevel, RequiredLevel"
        " FROM item_template")}
    names = {i: row[1] for i, row in source.items()}
    reward_cols = [f"RewardItem{n}" for n in range(1, 5)] + [f"RewardChoiceItemID{n}" for n in range(1, 7)]
    subsets = {
        "all item_template": set(source),
        "npc_vendor items": column_set(conn, "SELECT DISTINCT item FROM npc_vendor WHERE item > 0"),
        "quest reward items": set().union(*(column_set(conn, f"SELECT DISTINCT {c} FROM quest_template") for c in reward_cols)),
        "creature loot items": column_set(conn, "SELECT DISTINCT Item FROM creature_loot_template WHERE Reference = 0"),
    }
    print("\n## Items (item_template.entry vs Retail ItemSparse.ID)")
    for title, ids in subsets.items():
        print_split(title, classify(ids & set(source), names, retail_names))

    appearance = read_csv(retail / "ItemAppearance.csv")
    display_by_item = {}
    with (retail / "ItemModifiedAppearance.csv").open(newline="", encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            if row["ItemAppearanceModifierID"] == "0":
                look = appearance.get(int(row["ItemAppearanceID"]))
                if look:
                    display_by_item[int(row["ItemID"])] = int(look["ItemDisplayInfoID"])
    same = [i for i in source if retail_names.get(i) == names[i]]
    checks = Counter()
    mismatches = defaultdict(list)
    for i in same:
        (_, _, cls, sub, inv, quality, display, buy, sell, ilvl, req) = source[i]
        s, it = sparse[i], item.get(i)
        fields = {
            "class/subclass": it is not None and (int(it["ClassID"]), int(it["SubclassID"])) == (cls, sub),
            "InventoryType": int(s["InventoryType"]) == inv,
            "Quality": int(s["OverallQualityID"]) == quality,
            "displayid (via ItemModifiedAppearance mod 0)": display_by_item.get(i) == display,
            "BuyPrice": int(s["BuyPrice"]) == buy,
            "SellPrice": int(s["SellPrice"]) == sell,
            "ItemLevel": int(s["ItemLevel"]) == ilvl,
            "RequiredLevel": int(s["RequiredLevel"]) == req,
        }
        for field, ok in fields.items():
            checks[field] += ok
            if not ok and len(mismatches[field]) < EXAMPLES:
                mismatches[field].append(i)
    print(f"\n### Field agreement for the {len(same)} same-id-same-name items")
    for field, ok in checks.items():
        print(f"- {field}: {ok} ({100 * ok / len(same):.1f}%) agree; mismatch e.g. {mismatches[field]}")


def report_spells(conn, retail, wrath):
    retail_names = {i: row["Name_lang"] for i, row in read_csv(retail / "SpellName.csv").items()}
    wrath_names = {i: row["Name_lang"] for i, row in read_csv(wrath / "SpellName.csv").items()}
    learnable = column_set(conn, "SELECT DISTINCT Spell FROM skill_line_ability")
    class_ability_ids = defaultdict(set)  # (name, class id) -> Retail spell ids a class skill line grants
    for spell, mask in conn.execute("SELECT Spell, ClassMask FROM skill_line_ability WHERE ClassMask > 0"):
        for class_id in range(1, 32):
            if mask & (1 << (class_id - 1)) and spell in retail_names:
                class_ability_ids[(retail_names[spell], class_id)].add(spell)
    class_of = defaultdict(set)
    for spell, class_id in conn.execute(
            "SELECT DISTINCT s.SpellId, t.Requirement FROM trainer_spell s JOIN trainer t ON t.Id = s.TrainerId WHERE t.Type = 0"):
        class_of[spell].add(class_id)
    trainer = column_set(conn, "SELECT DISTINCT SpellId FROM trainer_spell")
    class_trainer = column_set(
        conn, "SELECT DISTINCT s.SpellId FROM trainer_spell s JOIN trainer t ON t.Id = s.TrainerId WHERE t.Type = 0")
    reward = column_set(conn, "SELECT DISTINCT RewardSpell FROM quest_template WHERE RewardSpell > 0")
    display = column_set(conn, "SELECT DISTINCT RewardDisplaySpell FROM quest_template")
    print("\n## Spells (AzerothCore spell ids, names from Wrath Classic SpellName, vs Retail SpellName)")
    for title, ids in {
        "trainer_spell (all trainers)": trainer,
        "trainer_spell (class trainers, trainer.Type=0)": class_trainer,
        "quest_template.RewardSpell": reward,
        "quest_template.RewardDisplaySpell": display,
    }.items():
        split = classify(ids, wrath_names, retail_names)
        print_split(title, split)
        same = [i for i, _ in split.get("same id, same name", [])]
        in_sla = [i for i in same if i in learnable]
        if "class trainers" in title:
            ambiguous = [i for i, _ in split.get("id absent, name ambiguous in Retail", [])]
            resolved = {i: next(iter(ids_)) for i in ambiguous
                        for ids_ in [set().union(*(class_ability_ids[(wrath_names[i], c)] for c in class_of[i]))]
                        if len(ids_) == 1}
            print(f"- of name-ambiguous, resolved to one Retail spell by (name, trainer class) in skill_line_ability:"
                  f" {len(resolved)}/{len(ambiguous)} e.g. "
                  + "; ".join(f"{i} {wrath_names[i]!r} -> {r}" for i, r in list(resolved.items())[:EXAMPLES]))
        print(f"- of same-id-same-name, in Retail skill_line_ability: {len(in_sla)}/{len(same)}"
              f"; not, e.g. {[(i, retail_names[i]) for i in same if i not in learnable][:EXAMPLES]}")


def report_displays(conn, retail, wrath):
    retail_cdi = read_csv(retail / "CreatureDisplayInfo.csv")
    wrath_cdi = read_csv(wrath / "CreatureDisplayInfo.csv")
    retail_model = {i: row["FileDataID"] for i, row in read_csv(retail / "CreatureModelData.csv").items()}
    wrath_model = {i: row["FileDataID"] for i, row in read_csv(wrath / "CreatureModelData.csv").items()}
    ids = column_set(conn, "SELECT DISTINCT CreatureDisplayID FROM creature_template_model")
    split = defaultdict(list)
    for i in sorted(ids):
        if i not in retail_cdi:
            split["absent in Retail"].append((i, ""))
        elif i not in wrath_cdi:
            split["in Retail, absent in Wrath Classic (no model to compare)"].append((i, ""))
        else:
            wrath_file = wrath_model.get(int(wrath_cdi[i]["ModelID"]))
            retail_file = retail_model.get(int(retail_cdi[i]["ModelID"]))
            if wrath_file == retail_file:
                split["same id, same model FileDataID"].append((i, wrath_file))
            else:
                split["same id, different model FileDataID"].append((i, f"{wrath_file} -> {retail_file}"))
    print("\n## Creature displays (creature_template_model.CreatureDisplayID vs Retail CreatureDisplayInfo -> CreatureModelData.FileDataID)")
    print_split("distinct display ids", split)


def report_quests(conn, retail):
    quest_v2 = read_csv(retail / "QuestV2.csv")
    questline = {int(row["QuestID"]) for row in read_csv(retail / "QuestLineXQuest.csv").values()}
    print("\n## Quests (quest_template.ID vs Retail QuestV2.ID; Retail DB2 carries no quest titles)")
    for title, sql in {
        "all quest_template": "SELECT ID FROM quest_template",
        "creature-started quests": "SELECT DISTINCT quest FROM creature_queststarter",
    }.items():
        ids = column_set(conn, sql)
        present = sorted(ids & set(quest_v2))
        absent = sorted(ids - set(quest_v2))
        print(f"- {title}: {len(ids)}; id in QuestV2 {len(present)} ({100 * len(present) / len(ids):.1f}%)"
              f", in a Retail QuestLine {len(ids & questline)}; absent {len(absent)} e.g. "
              + "; ".join(f"{q} {conn.execute('SELECT LogTitle FROM quest_template WHERE ID=?', (q,)).fetchone()[0]!r}"
                          for q in absent[:EXAMPLES]))


def report_taxi(retail, wrath):
    retail_nodes = {i: row["Name_lang"] for i, row in read_csv(retail / "TaxiNodes.csv").items()}
    wrath_nodes = {i: row["Name_lang"] for i, row in read_csv(wrath / "TaxiNodes.csv").items()}
    print("\n## Taxi (Wrath Classic TaxiNodes/TaxiPath vs Retail)")
    print_split("TaxiNodes", classify(set(wrath_nodes), wrath_nodes, retail_nodes))
    retail_paths = read_csv(retail / "TaxiPath.csv")
    wrath_paths = read_csv(wrath / "TaxiPath.csv")
    same = [i for i, row in wrath_paths.items() if i in retail_paths
            and (row["FromTaxiNode"], row["ToTaxiNode"]) == (retail_paths[i]["FromTaxiNode"], retail_paths[i]["ToTaxiNode"])]
    print(f"- TaxiPath: {len(wrath_paths)} Wrath paths; same id + same endpoints in Retail {len(same)}"
          f" ({100 * len(same) / len(wrath_paths):.1f}%)")


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--db", type=Path, default=PROJECT_DIR / "data/world.db")
    parser.add_argument("--retail", type=Path, default=DEFAULT_RETAIL)
    parser.add_argument("--wrath", type=Path, default=DEFAULT_WRATH)
    args = parser.parse_args()
    conn = sqlite3.connect(f"file:{args.db}?mode=ro", uri=True)
    report_items(conn, args.retail)
    report_spells(conn, args.retail, args.wrath)
    report_displays(conn, args.retail, args.wrath)
    report_quests(conn, args.retail)
    report_taxi(args.retail, args.wrath)
    conn.close()


if __name__ == "__main__":
    main()
