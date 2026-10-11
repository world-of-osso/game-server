# Class spell batch21 — 2026-10-11

Eight previously underived core roots, selected for rotation, tank and healing impact. One authored numeric contract implemented; seven source reviews distinguish unidentified fields from missing runtime prerequisites. [Contract](specs/spell-dummy-scripts.md#twenty-first-batch-native-combo-builders), [runtime](wiki/systems/spell-casting.md#twenty-first-batch-native-combo-builders), [matrix](spell-coverage.md).

## Capability matrix

| Root | Done / proof level | Missing / exact boundary |
|---|---|---|
|106951 Berserk|Native aura4 index1 supplies s2=1 additional combo point. Active same-family, positive native self-directed ENERGIZE builders gain it once per grant. Real completed-root casts prove Shred, Rake and multi-enemy Thrash; native cap/overflow, expiry, independent1500ms343216 periodic point, unrelated family and rejected friendly cast covered. Targeted8/8 at1aef6ea.|Numeric Dummy-field closure only.405189 over-cap storage/finisher recovery, script-only grants and full Feral rotation remain uncertified. No new periodic clock or duplicated bonus on343216.|
|113656 Fists of Fury|Reviewed/deferred, source-level proof: s6=60 is secondary-hit percentage; s1=5 is reduced-damage boundary.|Tooltip authors five117418 hits, but root aura226 periods are166ms over4sec, plus an independent AP effect4. Children117418/120086 have60-degree cones but radius indexes0. Variable251 supplies weapon/stance arithmetic, not the missing dispatch/clock/radius rule or beyond-five reduction curve. No guessed five-tick schedule, radius or hard cap.|
|119582 Purifying Brew|Reviewed/runtime prerequisite: s1=50-percent Stagger clearance with s2=8-percent-max-health minimum.|No server Stagger reservoir/delayed-damage tick representation in this snapshot. Numbers are known; which pending records to clear and remaining-payment behavior are not established locally. No fabricated pool or immediate healing substitute.|
|228477 Soul Cleave|Reviewed/runtime prerequisite: s3=2 consumed Soul Fragments; child228478s3 gives damage bonus per fragment. Existing native child/heal retained.|No server Soul Fragment actor/reservoir/collection-ownership representation or local eligible-record binding. Variable327's legacy weapon/heal formulas do not identify consumed fragments. Soft-cap curve remains unidentified. No invented fragment count or collection action.|
|322101 Expel Harm|Reviewed/deferred: s2=10 converts healing to115129 Nature damage.|Child115129 targets18/16, both radius indexes0, MaxTargets1. Tooltip's115129A1 resolves to absent/zero radius; no description-variable mapping supplies range or enemy selection. Local wording does not settle total versus effective healing/overheal. No guessed10yd/nearest-enemy or overheal rule.|
|288613 Trueshot|Reviewed/unidentified: aura4 index4 Base0, no trigger or tooltip/variable reference.|Cooldown/crit amounts belong to separate effects0/1/2/3. No local route establishes an independent zero-marker action; existing modifiers are not duplicated or counted as marker closure.|
|357208 Fire Breath|Reviewed/runtime prerequisite: variable688 supplies stage-dependent357209 direct/DoT amounts/durations; native SpellEmpower1 stages1000/750/750ms exist.|Server lacks empower-release rank input/state/resolution. Root zero4/E3 does not independently bind a released rank to its child. No default-rank/instant replacement. Cone and beyond-five reduction remain separate gaps.|
|367226 Spiritbloom|Reviewed/runtime prerequisite: description specifies one/two/three allies by rank; variable703 explicitly supplies30yd jump range; SpellEmpower4 stages1000/750/750ms exist.|Server lacks empower-release rank input/state/resolution and child367230 rank-dependent dispatch/selection. Range is known, not missing. No rank-I fallback or invented injury tie-break rule.|

The seven reviews are not seven implementations. Runtime prerequisite deferrals do not claim native amounts/stage durations are underivable. TDB1210 registration names establish routes only, not algorithms.

## Census and sources

Frozen **d2796d9→1aef6ea** reproduces **158 roots/110 reviewed/48 underived →157/117/40**. Delta **-1 root/+7 reviewed/-8 underived**: Berserk's only counted Dummy field is consumed; seven retained roots now have exact review reasons. E3 **111/119** and aura226 **18/19** unchanged; aura4 **64/72→63/71** IDs/memberships.13 classes/40 specs,630 book IDs/1551 memberships and302 core IDs/355 memberships retained. Previous all40 script/proof ID sets reproduced before one named addition; Feral core named scripts4→5 and positive proof5→6. No generic no-op relabeling.

**338 native SQLite rows exactly match** Mainline12.1.0.69933 CSV columns, zero differences (exact strings,1e-10 numeric tolerance). Included referenced durations/ranges/radii/cast times and child rows. Description-variable mappings:113656→251,228477→327,357208→688,367226→703; explicit absence for106951,119582,322101,288613. SpellEmpower/SpellEmpowerStage rows and hashes recorded separately. Fixtures clone real SpellStore rows; only Variance is zeroed. AP/health/positions/power pools are ordinary actor fixture inputs, not invented spell data.

Source roots: read-only `data/world.db`; `/home/osso/Projects/world-of-osso/game-engine/data/db2/12.1.0.69933/{Spell,SpellEffect,SpellXDescriptionVariables,SpellDescriptionVariables,SpellEmpower,SpellEmpowerStage}.csv`; local `data/reference/tdb1210/TDB_full_world_1210.26091_2026_09_09.sql` registration rows. Berserk description directly assigns s2 to additional points. Local route receipts include Cat Form, Rake, Thrash, Sinister Strike, Feast of Souls, Fire Breath/damage and Scouring Flame; no C++ algorithm or external/CDN source used.

Prior Death Strike history, Karma, Pandemic, Lifebloom near-expiry, Singularity area, guardians, unidentified markers and live limits remain as [batch20](spell-dummy20-coverage.md) and linked prior batches. No world.db writes, data/.uid commits, master merge, deployment or live server/UDP5000.

## Proof ledger

| Revision | Command / result / scope |
|---|---|
|5125ee0|Initial8-test run:4 builder/log failures expose missing bonus, but boundary assertions mix independent native periodic points. Those boundary failures are not accepted RED evidence.|
|2e9c530|Corrected native fixture isolates already-paid343216 and clears its inbox. RED: four builder/amount/overflow failures; four negative/expiry controls pass.|
|4e394a0|First GREEN attempt blocked by helper visibility compilation; not behavioral proof.|
|1aef6ea|Visibility corrected; targeted `cargo test --locked -p server --bin game-server dummy21`:8/8 GREEN, warning-free.|

Final requested `spell_cast` gate, fmt/check and independent artifact verification are pending. Do not infer full-server/live/full-rotation acceptance from targeted tests.

All Cargo verification pins the previous batch's immutable local shared-protocol44b2b0f with `--config patch."https://github.com/world-of-osso/shared-protocol.git".shared.path="/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/spell-dummy20-2026-10-11/shared-protocol"`. Pool quest-wire drift is not silently integrated or fixed here.

Durable evidence: `/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/spell-dummy21-2026-10-11/`: before/after source archives,338-row equivalence receipts, variable and empower receipts, exact seven reviewed reasons, frozen census/all40 generators/results, RED/GREEN full stdout/stderr and command receipts.

## All40-spec preference table

Counts mean named script presence, not generic dispatch or full ability parity. Every batch20 ID set/count reproduced before additions. No generic handler relabelled as a new named script.

| Spec | Book script roots before→after | Core script roots before→after | Core A before→after |
|---|---:|---:|---:|
| Mage Arcane (62) | 3→3 | 4→4 | 5→5 |
| Mage Fire (63) | 1→1 | 3→3 | 5→5 |
| Mage Frost (64) | 1→1 | 5→5 | 5→5 |
| Paladin Holy (65) | 8→8 | 5→5 | 7→7 |
| Paladin Protection (66) | 6→6 | 5→5 | 6→6 |
| Paladin Retribution (70) | 6→6 | 4→4 | 5→5 |
| Warrior Arms (71) | 2→2 | 4→4 | 7→7 |
| Warrior Fury (72) | 3→3 | 4→4 | 6→6 |
| Warrior Protection (73) | 3→3 | 4→4 | 5→5 |
| Druid Balance (102) | 6→6 | 5→5 | 5→5 |
| Druid Feral (103) | 5→5 | 4→5 | 5→6 |
| Druid Guardian (104) | 5→5 | 3→3 | 4→4 |
| Druid Restoration (105) | 5→5 | 5→5 | 5→5 |
| Death Knight Blood (250) | 2→2 | 4→4 | 5→5 |
| Death Knight Frost (251) | 2→2 | 4→4 | 5→5 |
| Death Knight Unholy (252) | 2→2 | 3→3 | 4→4 |
| Hunter Beast Mastery (253) | 2→2 | 5→5 | 6→6 |
| Hunter Marksmanship (254) | 2→2 | 4→4 | 4→4 |
| Hunter Survival (255) | 2→2 | 4→4 | 4→4 |
| Priest Discipline (256) | 2→2 | 3→3 | 6→6 |
| Priest Holy (257) | 1→1 | 4→4 | 6→6 |
| Priest Shadow (258) | 3→3 | 5→5 | 8→8 |
| Rogue Assassination (259) | 5→5 | 5→5 | 8→8 |
| Rogue Outlaw (260) | 4→4 | 3→3 | 5→5 |
| Rogue Subtlety (261) | 4→4 | 4→4 | 7→7 |
| Shaman Elemental (262) | 1→1 | 5→5 | 6→6 |
| Shaman Enhancement (263) | 2→2 | 4→4 | 6→6 |
| Shaman Restoration (264) | 2→2 | 4→4 | 5→5 |
| Warlock Affliction (265) | 1→1 | 4→4 | 5→5 |
| Warlock Demonology (266) | 1→1 | 3→3 | 5→5 |
| Warlock Destruction (267) | 2→2 | 5→5 | 7→7 |
| Monk Brewmaster (268) | 3→3 | 5→5 | 6→6 |
| Monk Windwalker (269) | 4→4 | 4→4 | 5→5 |
| Monk Mistweaver (270) | 4→4 | 4→4 | 4→4 |
| Demon Hunter Havoc (577) | 6→6 | 4→4 | 5→5 |
| Demon Hunter Vengeance (581) | 5→5 | 4→4 | 4→4 |
| Evoker Devastation (1467) | 4→4 | 4→4 | 5→5 |
| Evoker Preservation (1468) | 4→4 | 3→3 | 4→4 |
| Evoker Augmentation (1473) | 6→6 | 3→3 | 4→4 |
| Demon Hunter Devourer (1480) | 7→7 | 3→3 | 4→4 |
