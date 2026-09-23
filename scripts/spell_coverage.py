#!/usr/bin/env python3
"""Class spell coverage inventory for the server spell system.

Derives the in-scope spell set for the classes offered by character creation
from the imported Retail DB2 tables in data/world.db (opened read-only), then
aggregates spell_effect Effect / EffectAura IDs, power types and implicit
targets. Names come from TrinityCore master enums saved under data/reference/.

Usage: python3 scripts/spell_coverage.py [--db PATH] [--reference DIR] [--out PATH]
"""

import argparse
import re
import sqlite3
from collections import defaultdict
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
DEFAULT_DB = REPO / "data" / "world.db"
DEFAULT_REFERENCE = REPO / "data" / "reference"
DEFAULT_OUT = REPO / "docs" / "spell-coverage.md"
DB2_BUILD = "12.1.0.69933"

# ChrClasses.ID -> (class skill line, short label). Character creation classes only.
CLASSES = {
    1: (840, "WAR"),
    2: (800, "PAL"),
    3: (795, "HUN"),
    4: (921, "ROG"),
    5: (804, "PRI"),
    6: (796, "DK"),
    7: (924, "SHA"),
    8: (904, "MAG"),
    9: (849, "WLK"),
    11: (798, "DRU"),
}
MAX_LEVEL = 90
# SkillLineAbilityAcquireMethod::NeverLearned (TrinityCore DBCEnums.h).
ACQUIRE_NEVER_LEARNED = 3
TRIGGER_DEPTH = 3
# SpellEffects whose EffectAura is an AuraType (TrinityCore SharedDefines.h names).
AURA_EFFECTS = {6, 27, 35, 65, 119, 128, 129, 143, 174, 202, 271}
CUT_SHARE = 0.95


def open_db(path):
    return sqlite3.connect(f"file:{path}?mode=ro", uri=True)


def parse_enum(text, enum_name):
    match = re.search(r"enum\s+" + enum_name + r"\b[^{]*\{(.*?)\};", text, re.DOTALL)
    if not match:
        raise ValueError(f"enum {enum_name} not found")
    return {
        int(v): k
        for k, v in re.findall(r"^\s*(\w+)\s*=\s*(-?\d+)", match.group(1), re.MULTILINE)
    }


def load_enum_names(reference_dir):
    shared = (reference_dir / "SharedDefines.h").read_text()
    auras = (reference_dir / "SpellAuraDefines.h").read_text()
    return {
        "effect": parse_enum(shared, "SpellEffects"),
        "aura": parse_enum(auras, "AuraType"),
        "target": parse_enum(shared, "Targets"),
        "power": parse_enum(shared, "Powers"),
    }


def class_bit(class_id):
    return 1 << (class_id - 1)


def baseline_spells(db, class_id, skill_line):
    """skill_line_ability rows on the class skill line applicable to the class.

    Rule: ClassMask is 0 or has the class bit; RaceMasks is ignored (any race);
    AcquireMethod != NeverLearned; spell_levels.SpellLevel (DifficultyID 0,
    missing = 0) <= MAX_LEVEL.
    """
    rows = db.execute(
        """
        SELECT a.Spell, a.ClassMask, a.AcquireMethod, COALESCE(l.SpellLevel, 0)
        FROM skill_line_ability a
        LEFT JOIN spell_levels l ON l.SpellID = a.Spell AND l.DifficultyID = 0
        WHERE a.SkillLine = ?
        """,
        (skill_line,),
    )
    return {
        spell
        for spell, mask, acquire, level in rows
        if (mask == 0 or mask & class_bit(class_id))
        and acquire != ACQUIRE_NEVER_LEARNED
        and level <= MAX_LEVEL
    }


def spec_spells(db, class_id):
    """specialization_spells plus ChrSpecialization mastery spells, per spec."""
    specs = {}
    for spec_id, name, m0, m1 in db.execute(
        "SELECT ID, Name_lang, MasterySpellID_0, MasterySpellID_1 FROM chr_specialization WHERE ClassID = ?",
        (class_id,),
    ):
        spells = {
            s
            for (s,) in db.execute(
                "SELECT SpellID FROM specialization_spells WHERE SpecID = ?", (spec_id,)
            )
        }
        spells |= {m for m in (m0, m1) if m}
        specs[name] = spells
    return specs


def talent_spells(db, skill_line):
    """trait_definition.SpellID for every node of the class trait tree, hero subtrees included."""
    rows = db.execute(
        """
        SELECT DISTINCT d.SpellID
        FROM skill_line_x_trait_tree x
        JOIN trait_node n ON n.TraitTreeID = x.TraitTreeID
        JOIN trait_node_x_trait_node_entry ne ON ne.TraitNodeID = n.ID
        JOIN trait_node_entry e ON e.ID = ne.TraitNodeEntryID
        JOIN trait_definition d ON d.ID = e.TraitDefinitionID
        WHERE x.SkillLineID = ? AND d.SpellID != 0
        """,
        (skill_line,),
    )
    return {s for (s,) in rows}


def trigger_closure(db, roots):
    """Spells reached via spell_effect.EffectTriggerSpell (DifficultyID 0), up to TRIGGER_DEPTH hops."""
    seen = set(roots)
    frontier = set(roots)
    for _ in range(TRIGGER_DEPTH):
        found = set()
        for spell in frontier:
            for (trigger,) in db.execute(
                """
                SELECT DISTINCT e.EffectTriggerSpell FROM spell_effect e
                JOIN spell_name n ON n.ID = e.EffectTriggerSpell
                WHERE e.SpellID = ? AND e.DifficultyID = 0 AND e.EffectTriggerSpell != 0
                """,
                (spell,),
            ):
                found.add(trigger)
        frontier = found - seen
        seen |= frontier
    return seen - set(roots)


def existing_spells(db, spells):
    known = {s for (s,) in db.execute("SELECT ID FROM spell_name")}
    return spells & known


def derive_class_sets(db):
    """Returns {class_id: {"baseline", "spec", "specs", "talent", "triggered", "all"}}."""
    result = {}
    for class_id, (skill_line, _) in CLASSES.items():
        base = existing_spells(db, baseline_spells(db, class_id, skill_line))
        specs = {
            k: existing_spells(db, v) for k, v in spec_spells(db, class_id).items()
        }
        spec = set().union(*specs.values())
        talent = existing_spells(db, talent_spells(db, skill_line))
        direct = base | spec | talent
        triggered = trigger_closure(db, direct)
        result[class_id] = {
            "baseline": base,
            "spec": spec,
            "specs": specs,
            "talent": talent,
            "triggered": triggered,
            "direct": direct,
            "all": direct | triggered,
        }
    return result


def spell_class_index(class_sets):
    index = defaultdict(set)
    for class_id, sets in class_sets.items():
        for spell in sets["all"]:
            index[spell].add(class_id)
    return index


def aggregate(db, class_sets):
    index = spell_class_index(class_sets)
    direct = set().union(*(s["direct"] for s in class_sets.values()))
    tallies = {
        k: defaultdict(lambda: {"spells": set(), "instances": 0})
        for k in ("effect", "aura", "target", "power")
    }

    def add(kind, key, spell):
        entry = tallies[kind][key]
        entry["spells"].add(spell)
        entry["instances"] += 1

    for spell, effect, aura, t0, t1 in db.execute(
        "SELECT SpellID, Effect, EffectAura, ImplicitTarget_0, ImplicitTarget_1 FROM spell_effect WHERE DifficultyID = 0"
    ):
        if spell not in index:
            continue
        add("effect", effect, spell)
        if effect in AURA_EFFECTS:
            add("aura", aura, spell)
        for target in (t0, t1):
            if target:
                add("target", target, spell)
    for spell, power in db.execute("SELECT SpellID, PowerType FROM spell_power"):
        if spell in index:
            add("power", power, spell)
    return tallies, index, direct


def cut_line(tally):
    """Number of IDs (by instance count, descending) covering CUT_SHARE of instances."""
    counts = sorted((v["instances"] for v in tally.values()), reverse=True)
    total = sum(counts)
    running = 0
    for n, count in enumerate(counts, 1):
        running += count
        if running >= CUT_SHARE * total:
            return n, total
    return len(counts), total


def class_labels(spells, index):
    ids = set().union(*(index[s] for s in spells))
    return " ".join(CLASSES[c][1] for c in sorted(ids))


def render_table(tally, names, index, direct, header):
    ranked = sorted(tally.items(), key=lambda kv: (-kv[1]["instances"], kv[0]))
    rank = {key: n for n, (key, _) in enumerate(ranked, 1)}
    cut, _ = cut_line(tally)
    rows = sorted(tally.items(), key=lambda kv: (-len(kv[1]["spells"]), kv[0]))
    lines = [
        f"| {header} ID | Name | Spells | Direct spells | Instances | 95% set | Classes |",
        "|---:|---|---:|---:|---:|:---:|---|",
    ]
    for key, v in rows:
        name = names.get(key, "UNKNOWN")
        in_cut = "yes" if rank[key] <= cut else ""
        lines.append(
            f"| {key} | {name} | {len(v['spells'])} | {len(v['spells'] & direct)} | "
            f"{v['instances']} | {in_cut} | {class_labels(v['spells'], index)} |"
        )
    return "\n".join(lines)


def render_class_totals(db, class_sets):
    names = dict(db.execute("SELECT ID, Name_lang FROM chr_classes"))
    lines = [
        "| Class | Baseline | Spec | Talent | Direct (union) | Triggered only | Total |",
        "|---|---:|---:|---:|---:|---:|---:|",
    ]
    for class_id, s in class_sets.items():
        lines.append(
            f"| {class_id} {names[class_id]} | {len(s['baseline'])} | {len(s['spec'])} | {len(s['talent'])} | "
            f"{len(s['direct'])} | {len(s['triggered'])} | {len(s['all'])} |"
        )
    union = set().union(*(s["all"] for s in class_sets.values()))
    union_direct = set().union(*(s["direct"] for s in class_sets.values()))
    lines.append(
        f"| All (distinct) | | | | {len(union_direct)} | {len(union - union_direct)} | {len(union)} |"
    )
    return "\n".join(lines)


def render_spec_totals(db, class_sets):
    names = dict(db.execute("SELECT ID, Name_lang FROM chr_classes"))
    lines = ["| Class | Spec | Spec spells |", "|---|---|---:|"]
    for class_id, s in class_sets.items():
        for spec, spells in sorted(s["specs"].items()):
            lines.append(
                f"| {names[class_id]} | {spec or '(unnamed)'} | {len(spells)} |"
            )
    return "\n".join(lines)


HEADER = """# Class spell coverage

Generated by `scripts/spell_coverage.py`. Do not edit by hand.

- DB2 build: {build} (`data/world.db`, opened read-only).
- Enum names: TrinityCore master (`data/reference/SOURCE.txt` records the commit).
- Regenerate: `python3 scripts/spell_coverage.py` (from the game-server repo root).

## Derivation rules

- Classes: character-creation classes 1-9 and 11 (no Monk, Demon Hunter, Evoker).
- **Baseline:** `skill_line_ability` rows on the class skill line (CategoryID 7) where `ClassMask` is 0 or has the class bit, `AcquireMethod != 3` (NeverLearned), and `spell_levels.SpellLevel` (DifficultyID 0, missing = 0) <= {max_level}. `RaceMasks` is ignored. No spec skill lines exist in this build.
- **Spec:** `specialization_spells.SpellID` for every `chr_specialization` of the class, plus `MasterySpellID_0/1`.
- **Talent:** `trait_definition.SpellID` for every node of the class trait tree (`skill_line_x_trait_tree` -> `trait_node` -> `trait_node_x_trait_node_entry` -> `trait_node_entry` -> `trait_definition`). The class tree holds the class, spec and hero-talent nodes (hero subtrees are `trait_node.TraitSubTreeID`). Talents are attributed to the class, not to a spec.
- **Triggered:** closure over `spell_effect.EffectTriggerSpell` (DifficultyID 0, any Effect) up to {depth} hops from the direct set. Triggered-only spells are counted separately.
- Only spells present in `spell_name` are kept.
- **Instances:** `spell_effect` rows (DifficultyID 0) of in-scope spells. Aura rows are the Effect IDs {aura_effects}. Target instances count `ImplicitTarget_0` and `_1` separately (non-zero only).
- **95% set:** IDs ranked by instances; the smallest prefix whose instances reach 95% of the total.
- **Direct spells:** spells counted that are baseline, spec or talent (not triggered-only).

## Caveats

- `RaceMasks` is ignored, so race-specific class mounts (e.g. Paladin chargers) are in the baseline set.
- Talent nodes are not filtered by spec (`trait_node_x_trait_cond` is not evaluated); every node of the class tree counts, including removed or hidden nodes still present in the DB2.
- Trigger closure follows every non-zero `EffectTriggerSpell`, including LEARN_SPELL and proc auras; it does not follow spells cast from scripts, `EffectMiscValue` or `spell_class_options` masks.
- SPELL_EFFECT_APPLY_AURA is {apply_aura_share:.0%} of effect instances, so the 95% effect cut is small; the aura table is where handler coverage is decided.
- Enum sources are not tracked in git. Fetch them into `data/reference/` from `https://raw.githubusercontent.com/TrinityCore/TrinityCore/<commit>/src/server/game/` (`Miscellaneous/SharedDefines.h`, `Spells/Auras/SpellAuraDefines.h`).

## Summary

- Distinct effect IDs: {effects}. 95% of {effect_total} effect instances: top {effect_cut} effect IDs.
- Distinct aura IDs: {auras}. 95% of {aura_total} aura instances: top {aura_cut} aura IDs.
- Distinct power types: {powers}. Distinct implicit targets: {targets}.
"""

SECTIONS = [
    ("Effects", "effect", "Effect"),
    ("Auras", "aura", "Aura"),
    ("Power types", "power", "Power"),
    ("Implicit targets", "target", "Target"),
]


def render(db, class_sets, tallies, index, direct, enum_names):
    effect_cut, effect_total = cut_line(tallies["effect"])
    aura_cut, aura_total = cut_line(tallies["aura"])
    header = HEADER.format(
        build=DB2_BUILD,
        max_level=MAX_LEVEL,
        depth=TRIGGER_DEPTH,
        aura_effects=sorted(AURA_EFFECTS),
        apply_aura_share=tallies["effect"][6]["instances"] / effect_total,
        effects=len(tallies["effect"]),
        effect_total=effect_total,
        effect_cut=effect_cut,
        auras=len(tallies["aura"]),
        aura_total=aura_total,
        aura_cut=aura_cut,
        powers=len(tallies["power"]),
        targets=len(tallies["target"]),
    )
    parts = [
        header,
        "## Per-class totals\n",
        render_class_totals(db, class_sets),
        "\n## Per-spec spell counts\n",
        render_spec_totals(db, class_sets),
    ]
    for title, kind, label in SECTIONS:
        parts.append(f"\n## {title}\n")
        parts.append(
            render_table(tallies[kind], enum_names[kind], index, direct, label)
        )
    return "\n".join(parts) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--db", type=Path, default=DEFAULT_DB)
    parser.add_argument("--reference", type=Path, default=DEFAULT_REFERENCE)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT)
    args = parser.parse_args()
    db = open_db(args.db)
    class_sets = derive_class_sets(db)
    tallies, index, direct = aggregate(db, class_sets)
    args.out.write_text(
        render(db, class_sets, tallies, index, direct, load_enum_names(args.reference))
    )
    print(f"wrote {args.out}")


if __name__ == "__main__":
    main()
