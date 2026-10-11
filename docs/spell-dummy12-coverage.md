# Class spell batch12 — 2026-10-10

Eight bounded local Mainline12.1.0.69933 paths. [Contract](specs/spell-dummy-scripts.md#twelfth-batch-actorresourcesnapshot-consumers), [runtime](wiki/systems/spell-casting.md#twelfth-batch-actor-resource-and-snapshot-consumers), [full census](spell-coverage.md). Sources: local DB2/CASC, read-only canonical world.db, TDB1210 archive creature bindings.134 native rows match local CSVs exactly; no semantic or newline differences. Canonical guardian_spells has9586 rows, supplied by parent; worker writes no database.

## Capability matrix

| Path | Implemented / behavioral proof | Remaining boundary |
|---|---|---|
|51533 Feral Spirit|Named228562's two29264 wolves; native15sec lifecycle, archive58875 Spirit Walk cast and expiry.|Root Dummy retained for movement immunity, physical buff/Maelstrom/elemental variants; no inferred wolf melee or leap movement parity.|
|49206 Gargoyle|Native pet-title3238 temporary actor, authentic27829→51963.1242147 guarantees crit; successful owner Runic Power costs accumulate211947's1% per displayed point using native49206s4 and PowerType DisplayModifier10. Foreign/refused/triggered exclusions and expiry.|Tests set actor AP100 to isolate script and observe its actual connection (generic SummonedCreature combat logs do not route to owner). Non-demon inherited stats, movement and Army-of-the-Dead entry are not certified. Native51963 cast2500ms/summon25000ms, not invented attack cadence.|
|2098 Dispatch|Native damage multiplied by1/3/5 actual paid combo points at launch.|Rank6/7 and other talent semantics.|
|196819 Eviscerate|Same native1/3/5 expenditure snapshot, no later pool read.|Find Weakness extra Shadow damage and rank6/7.|
|1822 Rake|Named155722 bleed freezes the native60% stealth/rank bonus. Later rank/stealth removal leaves ticks; reapplication recomputes, later stealth cannot retroactively improve an ordinary snapshot.|Child155722 zero271/other talent interactions remain un-certified; not complete Feral rotation proof.|
|30108 Unstable Affliction|Native s3 adds10% to owner spell/periodic damage while own233490 exists, once rather than per instance; foreign/expired exclusions.|Multi-instance, dispel/death refund and melee/direct damage vulnerability remain open; root Dummy1 retained.|
|53600 Shield of Righteous|Own native6yd/120-degree front cone; primary-stat armor132403 effect1 once per successful cast, even empty; rear/side/far/ally exclusion and expiry.|132403 Dummy mirror/optional fields, root heal triggers and duration-extension/talent semantics.|
|212436 Butchery|459868 gates named459870 bleed on each landed enemy; native ticks/expiry and ally/far exclusion. Existing294029 cooldown reduction remains unchanged.|Secondary soft-cap curve and other talent interactions.|

## Census / proof

Baseline a8d22f9 exactly reproduces168 roots,87 reviewed/81 underived. After168 roots,88 reviewed/80 underived: Rake closes its complete numeric E3 contract; newly reachable132403 exposes a latent aura4 mirror. Feral Spirit retains its incomplete root contract. E3 falls120 IDs/128 memberships→119/127; aura4 rises67/75→68/76; aura226 remains18/19. Scope unchanged:13 classes/40 specs,630 book IDs/1551 memberships,302 core IDs/355 memberships. Script presence and A classification mean bounded proof, not whole-spell parity.

Durable evidence: `game-engine/data/diagnostics/spell-dummy12-2026-10-10/` (frozen before/after sources, reproducible generator, source receipts/equivalence, per-spec baseline assertions and proof ledger). Ten focused functions pass at60ce7ac; initial suite0/7 includes invalid Gargoyle missing-dependency and Rake passive-activation assumptions, corrected before final proof. Six spell paths have valid initial behavioral RED; no complete eight-path RED claim. Inherited Rapture's missing30108 fixture dependency/obsolete damage expectation was reproduced and corrected (focused1/1 at16b79bc). After merging origin/master ata995632, the requested `cargo test -p server --bin game-server spell_cast` ran ONCE:325/325, warning-free. `cargo fmt --all -- --check` and `cargo check --locked -p server --bin game-server` pass ata995632. Later edits are proof/docs only; parent owns independent acceptance. No realm/network/server/live client, world.db writes, invented IDs/numbers or master-destination merge.

## All40-spec preference table

| Spec | Book script roots before→after | Core script roots before→after | Core A before→after |
|---|---:|---:|---:|
| Mage Arcane (62) | 2→2 | 2→2 | 3→3 |
| Mage Fire (63) | 1→1 | 2→2 | 4→4 |
| Mage Frost (64) | 1→1 | 2→2 | 4→4 |
| Paladin Holy (65) | 2→3 | 2→2 | 4→4 |
| Paladin Protection (66) | 3→4 | 3→4 | 4→5 |
| Paladin Retribution (70) | 3→4 | 2→2 | 3→3 |
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
| Rogue Assassination (259) | 3→4 | 3→3 | 6→6 |
| Rogue Outlaw (260) | 2→4 | 2→3 | 4→5 |
| Rogue Subtlety (261) | 3→4 | 3→4 | 6→7 |
| Shaman Elemental (262) | 1→1 | 4→4 | 5→5 |
| Shaman Enhancement (263) | 2→2 | 3→4 | 5→6 |
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
