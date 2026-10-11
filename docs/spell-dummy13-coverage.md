# Class spell batch13 — 2026-10-10

Eight bounded contracts from local Mainline12.1.0.69933 DB2, read-only world.db and TDB1210. [Contract](specs/spell-dummy-scripts.md#thirteenth-batch-locally-authored-hit-rank-and-channel-contracts), [runtime](wiki/systems/spell-casting.md#thirteenth-batch-authored-hits-ranks-and-channel-clocks), [census](spell-coverage.md). No TrinityCore C++/external source, world.db writes, data/.uid commits, UDP5000 or master-destination merge.

## Capabilities / limits

| Path | Implemented / behavioral proof | Still missing |
|---|---|---|
|12294 Mortal Strike|Named115804 halves real incoming healing, expires natively.|Full talent variants.|
|772 Rend|Named388539 owned bleed, native tick amount/clock/expiry.|Crit vulnerability/talent variants.|
|163201 Execute|316405-gated10% refund of actual paid raw Rage including OptionalCost; surviving/kill/rank, partial availability, refusal and triggered-cast boundaries.|Damage interpolation/admission/talent variants.|
|32379 Shadow Word: Death|Tooltip s2 damage once, native strict20% threshold/150% bonus; active364675 raises threshold30 points and bonus100 points; one native backlash.|Legacy s1/rank variants.|
|257044 Rapid Fire|Native333ms/2000ms/EXTRA_INITIAL_PERIOD yields seven257045 shots and263585 Focus gains; cancellation stops shots and gains.|40-valued periodic mirror/Dummy2 and other talents.|
|19574 Bestial Wrath|Named344572 from the actual persistent pet at its own target; owner selection/foreign pet/no selection boundaries.|Pet buff binding, CC removal, charge ranks, periodic12 and native pet inheritance. Fixture deliberately uses non-Npc attacker/zero crit to isolate routing without RNG.|
|193530 Aspect of Wild|Initial free193455 at primary, no Focus payment; native Cobra level damage curve retained.|Additional-target geometry/count and ongoing cleave.|
|48181 Haunt|Owned mark's death resets caster cooldown; actual expiry/cancellation and foreign-owner timers preserved.|Optional periodic talent and complete vulnerability/rotation parity. Outside the fixed core/book probes: no census/script-count inflation.|

Guardian priority was investigated first. Local29264 template establishes1500ms swing time, not its owner-stat/melee damage curve.27829 binds51963, not non-demon inherited AP/SP. No inspected local scaling table/description or archive row establishes those missing formulas. Native spell/creature rows are retained in source-receipts.json; no guessed numbers or generic melee substitute. Named talent/transfer-copy contracts from prior handoffs remain separate gaps.

## Census and sources

Frozen0ac5c4d baseline exactly reproduces168 roots,88 reviewed/80 underived. After33b965c:166 roots,90 reviewed/76 underived. Execute's refund and Shadow Word: Death's threshold consume their remaining E3 numeric contracts; this is not full damage/legacy/talent certification. E3 falls119 IDs/127 memberships→117/125; aura4 stays68/76 and aura226 stays18/19. No newly exposed root. Scope remains13 classes/40 specs,630 book IDs/1551 memberships,302 core IDs/355 memberships.

151 native rows match corresponding local CSVs: zero missing rows/differences, numeric comparison tolerance1e-10, strings/newlines exact. Sources and all40 previous IDs/counts are checked by reproducible `regenerate.py`/`per_spec.py`; frozen before/after source prevents checkout drift from changing the result.

## Proof ledger

83f3f70: initial nine fixtures RED0/9. Seven failures identify missing/wrong behavior; Aspect initially lacked an explicit enemy and Bestial initially asserted an unbound pet buff (not valid final-contract REDs). ada5136 corrects that fixture; focused RED0/1 observes no Cobra.4584772 targeted GREEN9/9.2476ec8 adds partial paid-resource, refusal/trigger, persistent-pet ownership and actual Haunt death/expiry boundary fixtures: GREEN12/12, warning-free. A later source audit rejected applying owner19574 a108 to the pet: multiple reverse-description a79 candidates (186254/1285912) lack an explicit binding. Unsupported buff removed; only the named344572 strike retained. The initial pet-aura assertion is not final strike RED proof. No RNG-dependent exact damage expectations; variance bounds/zero crit/non-Npc hit source used.

3f637a3/0fa4079 Bestial focused failures exposed the fixture's pre-cast real pet melee swing, not incorrect AP inheritance. The assertion now isolates immediate cast health loss instead of including earlier melee; post-cast ordinary pet melee is not suppressed. 33b965c final focused Bestial2/2 passes. Remaining ten unchanged batch13 fixtures retain2476ec8 proof; master integration/single requested spell_cast gate pending. Parent owns independent integration acceptance; no full-crate/live/rotation claim.

Durable evidence: `game-engine/data/diagnostics/spell-dummy13-2026-10-10/` (frozen before/after sources, source receipts/equivalence, census/all40 generators and proof logs).

## All40-spec preference table

Counts mean bounded script presence, not whole spell parity. Prior40 rows/ID sets exactly reproduced; fixed catalog includes no arbitrary talent expansion.

| Spec | Book script roots before→after | Core script roots before→after | Core A before→after |
|---|---:|---:|---:|
| Mage Arcane (62) | 2→2 | 2→2 | 3→3 |
| Mage Fire (63) | 1→1 | 2→2 | 4→4 |
| Mage Frost (64) | 1→1 | 2→2 | 4→4 |
| Paladin Holy (65) | 3→3 | 2→2 | 4→4 |
| Paladin Protection (66) | 4→4 | 4→4 | 5→5 |
| Paladin Retribution (70) | 4→4 | 2→2 | 3→3 |
| Warrior Arms (71) | 1→2 | 1→4 | 5→7 |
| Warrior Fury (72) | 2→3 | 4→4 | 6→6 |
| Warrior Protection (73) | 2→3 | 3→3 | 4→4 |
| Druid Balance (102) | 5→5 | 5→5 | 5→5 |
| Druid Feral (103) | 4→4 | 4→4 | 5→5 |
| Druid Guardian (104) | 4→4 | 3→3 | 4→4 |
| Druid Restoration (105) | 4→4 | 3→3 | 3→3 |
| Death Knight Blood (250) | 2→2 | 2→2 | 3→3 |
| Death Knight Frost (251) | 2→2 | 2→2 | 4→4 |
| Death Knight Unholy (252) | 2→2 | 3→3 | 4→4 |
| Hunter Beast Mastery (253) | 2→2 | 2→4 | 3→5 |
| Hunter Marksmanship (254) | 2→2 | 3→4 | 3→4 |
| Hunter Survival (255) | 2→2 | 2→2 | 2→2 |
| Priest Discipline (256) | 2→2 | 3→3 | 6→6 |
| Priest Holy (257) | 1→1 | 4→4 | 6→6 |
| Priest Shadow (258) | 1→1 | 2→2 | 5→5 |
| Rogue Assassination (259) | 4→4 | 3→3 | 6→6 |
| Rogue Outlaw (260) | 4→4 | 3→3 | 5→5 |
| Rogue Subtlety (261) | 4→4 | 4→4 | 7→7 |
| Shaman Elemental (262) | 1→1 | 4→4 | 5→5 |
| Shaman Enhancement (263) | 2→2 | 4→4 | 6→6 |
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
