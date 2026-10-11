# Class spell batch15 — 2026-10-10

Eight bounded Mainline12.1.0.69933 contracts. Priority: core-script counts of two for Blood, Frost DK, Holy Paladin, Fire, Windwalker and Restoration Shaman; Vengeance's delayed field supplements them. No duplicate Blade of Justice entry: batch11 already implements/proves its talent gate. [Contract](specs/spell-dummy-scripts.md#fifteenth-batch-named-health-weapon-rank-and-delayed-hit-contracts), [runtime](wiki/systems/spell-casting.md#fifteenth-batch-native-health-weapon-ranks-and-delayed-hits), [census](spell-coverage.md).

## Capabilities / limits

| Entry | Implemented / behavioral proof | Missing / uncertified |
|---|---|---|
|55233 Vampiric Blood|s4/30% max health; refresh does not stack; native expiry restores baseline.|Other non-Dummy healing/absorb fields, full ability/talents. Remaining Dummy root closes, not full parity.|
|49143 Frost Strike|Variable609 `$owb==0`: only325464 without offhand; only222026+66196 with offhand. Exact named hit counts.|Root Dummy0 Base1 marker, detailed weapon/talent formulas.|
|107428 Rising Sun Kick /128595|Actual named185099 primary damage hit applies owned115804, native50% healing reduction/expiry; absent rank, other target and immune-hit exclusions.|Other Blackout Kick clauses/normalization and talent variants.|
|122470 Touch of Karma|s3/50%-max-health shield consumed by actual damage; cap leakage, depletion and native expiry.|s4/70% redirect, accumulation, enemy Dummy marker, s5/120. No guessed damage-copy action.|
|190319 Combustion /1254512|Snapshot20% of pre-cast character crit into native direct/periodic damage modifiers, retaining native10% rank base. Fireball direct damage proves snapshot survives later stat changes.|Periodic damage proof, Mastery189/383967 and periodic226 action.|
|1064 Chain Heal /288675 High Tide|Native flat30 removes root30% jump loss. Two completed casts consume two existing native modifier charges; next cast regains falloff.|157154 mana-threshold generation, broader group/target/talent semantics.|
|53563 Beacon of Light|Single owned mark; preserves foreign marks. Named53652 copies53651s1/10% of effective direct party/raid healing, not overheal/periodic healing or another power/crit roll.|Range, other root fields/periodic226, talents and complete healing-copy semantics.|
|204596 Sigil of Flame|Native2sec delay at original destination; named204598 native8yd hit plus one targeted8sec/2sec bleed. Moving caster does not move field.|179 actor/visuals, latent duplicate bleed2/zero4 marker,389787 timing. No invented Fury timing.|

Guardian owner-stat inheritance/wolf melee and Eye Beam native11-vs-tooltip10 discrepancy remain as [batch14](spell-dummy14-coverage.md) records. Dreadstalker98035 has no canonical template/guardian binding: no synthetic actor, substituted Imp or database import added.

## Census / authentic sources

Frozen615fb90 baseline exactly reproduces **164 roots,93 reviewed/71 underived** and every previous40-spec ID set/count. At1e719bf: **163 roots,98 reviewed/65 underived**. Vampiric Blood consumes its last Dummy field. Sigil consumes its root E3 but exposes named204598's latent aura4; remaining root stays counted. Karma's cap field is consumed without pretending its redirect/markers are complete. E3=115 IDs/123 memberships→114/122; aura4=67/75→66/74; aura226=18/19 unchanged. Fixed scope:13 classes/40 specs,630 book IDs/1551 memberships and302 core IDs/355 memberships.

238 native SQLite rows match every corresponding local CSV column (zero missing/different rows, numeric tolerance1e-10, exact strings/newlines, no omitted columns). Separate local description variable609 receipt establishes Frost Strike's alternatives. Sources only local DB2/read-only canonical world.db; no external sources or local TrinityCore C++ used. Canonical database never written.

## Proof ledger

- d4f5c75: initial corrected RED8/8 expected failures; first probe exposed invalid Mana-pool/travel assumptions and an already-passing Blade of Justice duplicate, which was removed. Initial Combustion expected coefficient was also wrong; do not count it as valid snapshot RED.
- 9bed228 implementation;4ee6f40 corrects native Fireball3.88125SP fixture. First GREEN6/8; remaining fixtures incorrectly used triggered Chain Heal (does not spend normal-cast charges) and25% instead of native50% Mortal Wounds.
- ca096e6 uses completed root Chain Heal and native50% wounds; adds owned reassignment, immunity and expiry boundaries. Corrected Combustion fixture uses50% initial crit then90% after cast:20% vs10% rank damage has disjoint native variance bounds. Temporarily disabling only the snapshot call reproduces that failure; new Beacon reassignment also fails. Temporary source change restored immediately; diagnostic-build unused-symbol warnings are not final-source warnings.
- 1e719bf replaces only the caster's old Beacon mark. Targeted **11/11 pass**, warning-free. Native row/census/table receipts cover this production revision.
- At6a56656, `git merge origin/master` is already up to date (615fb90). Requested `cargo test -p server --bin game-server spell_cast` ran ONCE: **360/360 pass**, warning-free. `cargo fmt --all --check` and `cargo check --locked -p server --bin game-server` pass. Later edits are proof/docs only; no redundant broad rerun. Manual changed-Rust readability audit records long explicit heal/pulse pipelines; metric CLI unavailable. No live/full-rotation or independent acceptance claim.

Durable evidence: `game-engine/data/diagnostics/spell-dummy15-2026-10-10/` — frozen source tars, generators,238-row equivalence, variable609, RED/GREEN logs. Parent owns independent integration acceptance.

## All40-spec preference table

Counts mean bounded production script presence, not whole ability parity. Improving existing Chain Heal does not inflate Restoration's count; ranked Rising Sun Kick counts as shared script presence in Mistweaver, not a claim that every build learns128595. Shared classbook Sigil bindings are also reflected in Havoc/Devourer. Previous40 rows/ID sets exactly reproduced.

| Spec | Book script roots before→after | Core script roots before→after | Core A before→after |
|---|---:|---:|---:|
| Mage Arcane (62) | 2→2 | 2→2 | 3→3 |
| Mage Fire (63) | 1→1 | 2→3 | 4→5 |
| Mage Frost (64) | 1→1 | 4→4 | 4→4 |
| Paladin Holy (65) | 3→4 | 2→3 | 4→5 |
| Paladin Protection (66) | 4→4 | 4→4 | 5→5 |
| Paladin Retribution (70) | 4→4 | 2→2 | 3→3 |
| Warrior Arms (71) | 2→2 | 4→4 | 7→7 |
| Warrior Fury (72) | 3→3 | 4→4 | 6→6 |
| Warrior Protection (73) | 3→3 | 3→3 | 4→4 |
| Druid Balance (102) | 5→5 | 5→5 | 5→5 |
| Druid Feral (103) | 4→4 | 4→4 | 5→5 |
| Druid Guardian (104) | 4→4 | 3→3 | 4→4 |
| Druid Restoration (105) | 4→4 | 3→3 | 3→3 |
| Death Knight Blood (250) | 2→2 | 2→3 | 3→4 |
| Death Knight Frost (251) | 2→2 | 2→3 | 4→5 |
| Death Knight Unholy (252) | 2→2 | 3→3 | 4→4 |
| Hunter Beast Mastery (253) | 2→2 | 4→4 | 5→5 |
| Hunter Marksmanship (254) | 2→2 | 4→4 | 4→4 |
| Hunter Survival (255) | 2→2 | 4→4 | 4→4 |
| Priest Discipline (256) | 2→2 | 3→3 | 6→6 |
| Priest Holy (257) | 1→1 | 4→4 | 6→6 |
| Priest Shadow (258) | 1→1 | 2→2 | 5→5 |
| Rogue Assassination (259) | 4→4 | 4→4 | 7→7 |
| Rogue Outlaw (260) | 4→4 | 3→3 | 5→5 |
| Rogue Subtlety (261) | 4→4 | 4→4 | 7→7 |
| Shaman Elemental (262) | 1→1 | 4→4 | 5→5 |
| Shaman Enhancement (263) | 2→2 | 4→4 | 6→6 |
| Shaman Restoration (264) | 1→1 | 2→2 | 3→3 |
| Warlock Affliction (265) | 0→0 | 3→3 | 5→5 |
| Warlock Demonology (266) | 0→0 | 2→2 | 4→4 |
| Warlock Destruction (267) | 1→1 | 3→3 | 5→5 |
| Monk Brewmaster (268) | 3→3 | 5→5 | 6→6 |
| Monk Windwalker (269) | 3→4 | 2→4 | 3→5 |
| Monk Mistweaver (270) | 4→4 | 3→4 | 3→4 |
| Demon Hunter Havoc (577) | 5→6 | 4→4 | 5→5 |
| Demon Hunter Vengeance (581) | 4→5 | 3→4 | 3→4 |
| Evoker Devastation (1467) | 4→4 | 4→4 | 5→5 |
| Evoker Preservation (1468) | 4→4 | 3→3 | 4→4 |
| Evoker Augmentation (1473) | 6→6 | 3→3 | 4→4 |
| Demon Hunter Devourer (1480) | 6→7 | 3→3 | 4→4 |
