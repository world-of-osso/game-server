# Guardian and damage-transfer coverage — 2026-10-10

Native Mainline12.1.0.69933, read-only world.db and local TDB1210.26091 archive. [Contract](specs/spell-guardians.md); [runtime](wiki/systems/spell-casting.md#guardian-actors-and-damage-transfer). No C++ bodies or guessed actor spell IDs.

## Capability matrix

| Path | Implemented / evidence | Remaining boundary |
|---|---|---|
| Generic effect28 guardian | Existing summon lifecycle now carries creature spells; target follows owner selection; normal creature cast pipeline pays native costs and observes cast/cooldown timing. Authentic111859→416→3110 fixture proves delay, retarget and native27s expiry. | Root111859 Dummy0/cleanse is unbound; fixture directly invokes native effect28. Index order is an engine scheduling policy, not Retail rotation proof. |
| Generic native aura81 | School-mask percent splitting after protected-target mitigation/absorbs and before health loss. Spell, periodic, real melee and direct damage paths covered; recipient absorbs, same-map/living/immune checks; no recursive splitting. | Other damage-copy/aura300 mechanics are not inferred from81. Per-recipient integer health truncation retains existing pipeline behavior. |
| Blessing of Sacrifice6940 | Native30% share and20% caster-health floor; spell/melee/periodic/direct fixtures and lifetime boundaries. | Rank/talent variants and comprehensive proc-event parity are not certified. |
| Local creature binding importer | 9586 exact TDB creature_template_spell rows loaded into an in-memory test DB only. Duplicate spell3110 at distinct416 indices deduplicates in runtime; duplicate-key import rolls back. | Production guardian_spells table is not imported: importer must run on a prepared DB copy. No world.db writes. |
| Named Wild Imp/Dreadstalker/Xuen/Rune Weapon/Darkglare/Tyrant | Authentic summon entries/metadata inspected and retained, never replaced by Imp416. | Their archive creature_template_spell and smart-script rows are absent; empty AIName/ScriptName supplies no bolt count/energy budget/cadence. Non-demon power inheritance is uncertified; existing demon pet power coefficients are reused, not new Retail scaling proof. |

No whole Dummy root closes. Census remains **168 roots,87 reviewed/81 underived**; E3 **120 IDs/128 memberships**, aura4 **67/75**, aura226 **18/19**. Generic actors/81 do not count as named scripts. Native6940 effect2 is a named consumed numeric field; Paladin book script presence increases by one per spec; core totals stay unchanged. Blade Flurry geometry, Atonement damage-to-healing curve, Touch of Karma and Spirit Link remain un-derived.

Durable receipts, frozen before/after source tars, generator, per-spec assertions and proof logs: `game-engine/data/diagnostics/spell-guardians-2026-10-10/`. Base dbe51fd; production snapshot a3e3762. Import still required: `python3 scripts/import-guardian-spells.py --archive data/reference/tdb1210/TDB_full_world_1210.26091_2026_09_09.sql --db data/world.db.new`; do not invoke against live world.db.

## All40-spec preference table

| Spec | Book script roots before→after | Core script roots before→after | Core A before→after |
|---|---:|---:|---:|
| Mage Arcane (62) | 2→2 | 2→2 | 3→3 |
| Mage Fire (63) | 1→1 | 2→2 | 4→4 |
| Mage Frost (64) | 1→1 | 2→2 | 4→4 |
| Paladin Holy (65) | 1→2 | 2→2 | 4→4 |
| Paladin Protection (66) | 2→3 | 3→3 | 4→4 |
| Paladin Retribution (70) | 2→3 | 2→2 | 3→3 |
| Warrior Arms (71) | 1→1 | 1→1 | 5→5 |
| Warrior Fury (72) | 2→2 | 4→4 | 6→6 |
| Warrior Protection (73) | 2→2 | 3→3 | 4→4 |
| Druid Balance (102) | 5→5 | 5→5 | 5→5 |
| Druid Feral (103) | 4→4 | 4→4 | 5→5 |
| Druid Guardian (104) | 4→4 | 3→3 | 4→4 |
| Druid Restoration (105) | 4→4 | 3→3 | 3→3 |
| Death Knight Blood (250) | 2→2 | 2→2 | 3→3 |
| Death Knight Frost (251) | 2→2 | 2→2 | 4→4 |
| Death Knight Unholy (252) | 2→2 | 3→3 | 4→4 |
| Hunter Beast Mastery (253) | 2→2 | 2→2 | 3→3 |
| Hunter Marksmanship (254) | 2→2 | 3→3 | 3→3 |
| Hunter Survival (255) | 2→2 | 2→2 | 2→2 |
| Priest Discipline (256) | 2→2 | 3→3 | 6→6 |
| Priest Holy (257) | 1→1 | 4→4 | 6→6 |
| Priest Shadow (258) | 1→1 | 2→2 | 5→5 |
| Rogue Assassination (259) | 3→3 | 3→3 | 6→6 |
| Rogue Outlaw (260) | 2→2 | 2→2 | 4→4 |
| Rogue Subtlety (261) | 3→3 | 3→3 | 6→6 |
| Shaman Elemental (262) | 1→1 | 4→4 | 5→5 |
| Shaman Enhancement (263) | 2→2 | 3→3 | 5→5 |
| Shaman Restoration (264) | 1→1 | 2→2 | 3→3 |
| Warlock Affliction (265) | 0→0 | 3→3 | 5→5 |
| Warlock Demonology (266) | 0→0 | 2→2 | 4→4 |
| Warlock Destruction (267) | 1→1 | 3→3 | 5→5 |
| Monk Brewmaster (268) | 3→3 | 5→5 | 6→6 |
| Monk Windwalker (269) | 3→3 | 2→2 | 3→3 |
| Monk Mistweaver (270) | 4→4 | 3→3 | 3→3 |
| Demon Hunter Havoc (577) | 4→4 | 2→2 | 3→3 |
| Demon Hunter Vengeance (581) | 3→3 | 3→3 | 3→3 |
| Evoker Devastation (1467) | 4→4 | 4→4 | 5→5 |
| Evoker Preservation (1468) | 4→4 | 3→3 | 4→4 |
| Evoker Augmentation (1473) | 6→6 | 3→3 | 4→4 |
| Demon Hunter Devourer (1480) | 4→4 | 2→2 | 3→3 |
