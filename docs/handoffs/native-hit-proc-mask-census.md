# Native hit proc-mask census — 2026-10-22

Requested for shared `spell_hit_masks` root fix b382b9d28. TC a352b1fa
`Spell.cpp:2343–2400` classifies melee/ranged by defense, then non-weapon hits by
TreatAsPeriodic/IsAbility attributes, not DefenseType0 versus1. Native Word of
Glory85673 DefenseType0 is not an ability; its helpful hit was incorrectly0x400,
preventing native378405's0x4000 proc admission.

## Catalog diff (counts and exact IDs)

Read-only local70448 frozen world DB SHA256
`6b8b5e96b2a1f0caac05df74abce559516cacae9279d85e669d97edfd0dd7f2c`.
No duplicate ordinary difficulty0 scalar rows in this catalog. Reproduction:
`python3 <D>/proc-mask-census.py`, where D is game-engine
`data/diagnostics/healer-ai-2026-10-21/light-of-the-titans/`.

**334969 spell records change possible helpful/harmful hit classification.**
The inventory includes dormant/reference/generic spell records, not334969 newly
castable spells. All exact spell IDs/names, before/after actor/target masks,
family/masks and source attributes are in `proc-mask-changed-spells.csv`.

| Prior non-weapon class | New native class | Records |
|---|---|---:|
| Ability | Spell | 332646 |
| Spell | Ability | 1658 |
| Spell | Periodic | 512 |
| Ability | Periodic | 153 |

Actor/target masks by class: helpful ability0x400/0x800, harmful
ability0x1000/0x2000; helpful spell0x4000/0x8000, harmful spell0x10000/0x20000;
helpful periodic0x200000/0x80000000, harmful periodic0x40000/0x80000.
Melee/ranged remain unchanged.

**5670 proc-aura records have changed static mask/family candidate sets:**3015
have gains,4950 have losses (categories overlap). Exact aura IDs and their source
proc masks/family unions plus per-side gained/lost spell counts are in
`proc-aura-mask-candidate-diff.jsonl`. To avoid repeating millions of spell IDs,
each side references disjoint group IDs in `proc-mask-spell-groups.json`, which
contains every exact spell ID. `proc-mask-census-summary.json` lists all changed
spell IDs, gain/loss aura IDs and transition totals. This is a normalized exact
ID inventory, not a sample or a list truncated to common spells.

## What those proc counts do NOT prove

These are static mask+native effect-family candidate changes. Both possible
helpful/harmful relations are listed; the census does not claim each spell
actually resolves both ways. Runtime script checks, explicitly scripted native
`spell_proc` restrictions, chance/RPPM/cooldowns, reflect-only gating, trigger
ownership/depth, damage/heal eligibility, known spells and real targets still
apply. Therefore do **not** describe3015 gains as3015 newly observed procs.
This census intentionally includes aura-options rows without generic trigger
types so explicitly scripted auras are not omitted; `has_trigger_type` is shown.
The lead's full-workspace integration gate remains required.

## Non-Paladin TC spot-check

Native Hellfire5857 has Attr3 `TreatAsPeriodic=0x02000000`. Its harmful hit changes
from spell0x10000 to periodic0x40000. Earthen Rage170374 has native100% chance and
proc mask0x10000:5857 is consequently removed from its static harmful-hit
candidates, not spuriously admitted as a direct spell. TC `Spell.cpp:2381–2386`
requires the periodic flags; TC `spell_shaman.cpp:861–886` checks for a spell other
than170379 and does not override that mask to accept periodic damage. This is a
source/native-eligibility spot-check, not a real Shaman/Hellfire learned-kit claim.
The changed non-Paladin Healing Stream5394 (Attr0 IsAbility) likewise becomes a
helpful ability rather than a spell per `Spell.cpp:2388–2393`.

Native mask-output controls use these unmodified rows plus Word85673, melee1464,
ranged19434 and ordinary Flash2061 in `auras/proc_mask_tests.rs`. Actual native
Word→Titans tests exercise the proc execution boundary, not only mask output.
