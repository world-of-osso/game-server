# Class spell batch18 — 2026-10-11

Eight bounded root entries, prioritizing Shadow/low-count Warlock, Protection Warrior and Frost Death Knight paths. [Contract](specs/spell-dummy-scripts.md#eighteenth-batch-native-leech-and-reapplication-contracts), [runtime](wiki/systems/spell-casting.md#eighteenth-batch-native-leech-and-reapplication), [matrix](spell-coverage.md).

## Capability matrix

| Entry | Done / native behavioral proof | Missing / uncertified |
|---|---|---|
|34914 Vampiric Touch|Native aura53 SP ticks; amplitude0.3 copies post-absorb damage once without double versatility. Completed root reaches real periodic damage/healing.|Aura4 Base2,87204 dispel retaliation, talents and full ability parity.|
|335467 Shadow Word: Madness|Native effect9 direct/aura53 periodic leech at amplitude0.3; immunity/full absorption/overkill exclusions. Same-owner reapplication redistributes unpaid periodic damage; foreign owners stay independent. Completed root proved.|Zero aura4 index1 remains unidentified; no guessed universal Pandemic rule.|
|234153 Drain Life|Native aura53 coefficient0.75/amplitude5; actual HostClient channel gives five1000ms ticks/5000ms and cancellation stops after one. Completed-root damage/healing proved.|334320/386619 talent multipliers and full channel variants.|
|55078 Blood Plague|Native AP leech coefficient0.33480000496/amplitude1.25; real50842 root applies disease whose ticks heal owner.|Aura4/271/381/380 descendants and212744 replacement remain uncertified.|
|205179 Phantom Singularity|Native primary aura53 coefficient0.27224999666/amplitude0.25; completed root reaches primary ticks/heals.|Area226/205246 routing, primary exclusion and eight-area-pulse parity are underivable; no duplicate guessed pulse.|
|190456 Ignore Pain|Native s2/50% per-hit absorb; explicit versatility budget; Protection73 accumulates remaining capacity up to s4/30% max health. Depletion removes aura.|Aura142, non-Protection/talent variants and full defensive parity. All this root's Dummy fields are consumed, not all non-Dummy mechanics.|
|17962/267115 Improved Conflagrate|Charges31 flat modifier gives two117828 uses per successful ranked root; native cap4 after repeated roots. Existing native30% per-use magnitude/consumption retained.|Other talents/secondary variants; batch18 does not claim a new root script merely for improving an existing binding.|
|49020 Obliterate|Local description variable608 selects physical+frost children: dual222024/66198 and1264082/1264083, or two-hand325461 and1264084. Actual roots prove child IDs/hit counts.|Dummy0/75 fields and talent variants remain underived.|

## Census and all40 method

Frozen63302bd → edfa536 independently reproduces **164 roots/100 reviewed/64 underived →163/103/60**. Ignore Pain is the single whole *Dummy-census* root closure; aura142 still blocks full spell parity. Vampiric Touch/Madness/Obliterate become reviewed partial roots without deleting their unknown markers. E3 stays114 IDs/122 memberships; aura4 goes67/75→66/74; aura226 stays18/19. Aura53's four missing core roots now dispatch native leech. No generic Dummy no-op or guardian change.

Fixed13 classes/40 specs,630 book IDs/1551 memberships and302 core IDs/355 memberships. Every previous40-spec script ID set/count is independently reproduced. Script presence adds only named335467/190456/49020 consumers: generic leech handlers are not relabelled scripts. Shadow core scripts2→3/A5→7, Protection Warrior scripts3→4/A4→5 and Frost DK scripts3→4. Shared classbook Drain Life gives one additional bounded A in each Warlock spec, not full spec parity. Blood's existing A gains real disease tick proof without inflating its count.

Karma's native70% redirect field has no locally determined period/accumulation route from124280's zero-period row; retained rather than an invented redirect timer. Lifebloom near-expiry/Pandemic threshold remains absent; Madness's explicit rollover is not generalized to it. Guardian inheritance/stamina/melee, Eye Beam11-versus-tooltip10/live facing and Deathmark live totals remain unchanged.

## Sources and proof ledger

**331 native SQLite rows** exactly match corresponding pinned local Mainline12.1.0.69933 CSV columns (zero differences, numeric tolerance1e-10, exact strings/newlines). TDB1210 registrations are names/routes only, not C++ bodies. Variable608's four child expressions are local SpellDescriptionVariables rows. Children are real SpellStore clones; only fixture Variance is pinned. Test positions, health and stats are concrete fixture inputs, not fabricated runtime coefficients/IDs. No external source, CDN, world.db write or UDP5000.

| Revision | Proof / limit |
|---|---|
|b524287|Compile-only test failure from wrong native offhand fields, not behavioral RED.|
|744a4ae|Nine failures, one vacuous0==0 rollover pass; Insanity admission failure invalid for leech RED.|
|8071b5d|Corrected deterministic **10/10 behavioral RED**: absent leech/SP snapshot,100% instead of50% soak, one instead of two Backdraft uses and all-six instead of branch-selected children.|
|fffe0b1|Production **10/10 GREEN**, compiler warning-free.|
|710ce4b|Native channel/cancellation and partial-absorb/no-double-versatility proofs pass; shield versatility capacity exposes one genuine failure: **12/13**.|
|edfa536|Native capacity factor plus completed-root fixtures: **14/14 GREEN**, compiler warning-free.|

Final origin/master integration and requested single spell_cast gate are recorded below after execution. No full-crate/live/full-rotation or independent verifier claim; parent owns integration acceptance.

Durable evidence: `game-engine/data/diagnostics/spell-dummy18-2026-10-11/` — exact before/after source snapshots, source-equivalence receipts, census/table generators, RED/GREEN logs and proof ledger.

## All40-spec preference table

Counts mean bounded script presence, not universal learning or full ability parity. Generic leech adds behavioral A evidence without inflating named-script counts. Book overlaps stay deduplicated per spec.

| Spec | Book script roots before→after | Core script roots before→after | Core A before→after |
|---|---:|---:|---:|
| Mage Arcane (62) | 3→3 | 4→4 | 5→5 |
| Mage Fire (63) | 1→1 | 3→3 | 5→5 |
| Mage Frost (64) | 1→1 | 4→4 | 4→4 |
| Paladin Holy (65) | 8→8 | 5→5 | 7→7 |
| Paladin Protection (66) | 6→6 | 4→4 | 5→5 |
| Paladin Retribution (70) | 6→6 | 4→4 | 5→5 |
| Warrior Arms (71) | 2→2 | 4→4 | 7→7 |
| Warrior Fury (72) | 3→3 | 4→4 | 6→6 |
| Warrior Protection (73) | 3→3 | 3→4 | 4→5 |
| Druid Balance (102) | 6→6 | 5→5 | 5→5 |
| Druid Feral (103) | 5→5 | 4→4 | 5→5 |
| Druid Guardian (104) | 5→5 | 3→3 | 4→4 |
| Druid Restoration (105) | 5→5 | 5→5 | 5→5 |
| Death Knight Blood (250) | 2→2 | 4→4 | 5→5 |
| Death Knight Frost (251) | 2→2 | 3→4 | 5→5 |
| Death Knight Unholy (252) | 2→2 | 3→3 | 4→4 |
| Hunter Beast Mastery (253) | 2→2 | 4→4 | 5→5 |
| Hunter Marksmanship (254) | 2→2 | 4→4 | 4→4 |
| Hunter Survival (255) | 2→2 | 4→4 | 4→4 |
| Priest Discipline (256) | 2→2 | 3→3 | 6→6 |
| Priest Holy (257) | 1→1 | 4→4 | 6→6 |
| Priest Shadow (258) | 1→1 | 2→3 | 5→7 |
| Rogue Assassination (259) | 4→4 | 4→4 | 7→7 |
| Rogue Outlaw (260) | 4→4 | 3→3 | 5→5 |
| Rogue Subtlety (261) | 4→4 | 4→4 | 7→7 |
| Shaman Elemental (262) | 1→1 | 4→4 | 5→5 |
| Shaman Enhancement (263) | 2→2 | 4→4 | 6→6 |
| Shaman Restoration (264) | 2→2 | 4→4 | 5→5 |
| Warlock Affliction (265) | 1→1 | 4→4 | 5→5 |
| Warlock Demonology (266) | 1→1 | 3→3 | 5→5 |
| Warlock Destruction (267) | 2→2 | 4→4 | 6→6 |
| Monk Brewmaster (268) | 3→3 | 5→5 | 6→6 |
| Monk Windwalker (269) | 4→4 | 4→4 | 5→5 |
| Monk Mistweaver (270) | 4→4 | 4→4 | 4→4 |
| Demon Hunter Havoc (577) | 6→6 | 4→4 | 5→5 |
| Demon Hunter Vengeance (581) | 5→5 | 4→4 | 4→4 |
| Evoker Devastation (1467) | 4→4 | 4→4 | 5→5 |
| Evoker Preservation (1468) | 4→4 | 3→3 | 4→4 |
| Evoker Augmentation (1473) | 6→6 | 3→3 | 4→4 |
| Demon Hunter Devourer (1480) | 7→7 | 3→3 | 4→4 |
