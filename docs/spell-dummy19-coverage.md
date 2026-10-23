# Class spell batch19 — 2026-10-11

Eight bounded spell/rank entries across low-count Frost Mage/Shadow Priest/Destruction paths and shared class mechanics. [Contract](specs/spell-dummy-scripts.md#nineteenth-batch-counts-healing-totals-and-owned-damage), [runtime](wiki/systems/spell-casting.md#nineteenth-batch-counts-healing-totals-and-owned-damage), [matrix](spell-coverage.md).

## Capability matrix

| Entry | Done / deterministic native behavioral proof | Missing / uncertified |
|---|---|---|
|44614 Flurry|Native s1 count3 selects three228596→228354 bolts instead of four stored slots; real children complete through missile path.|Freezing/talent bridges; not full spell parity. Dummy count root closes.|
|1329 Mutilate|Native exclusive-offhand AP selection halves27576 once; variable394 describes that result,5374 remains full; real root/children115+58 at fixture AP100.|Root/child zero/Base1 markers, poison and weapon variants remain unassigned.|
|34914 Vampiric Touch|Actual527 dispel applies87204 Horror to the dispeller from original owner; expiry/cancel never retaliate. Batch18 leech retained.|Aura4 Base2 and talent paths remain unidentified.|
|47585 Dispersion|Native25-percent total, seven native ticks including extra-initial; normal per-tick rounding gives252 healing at maxHP1000.|Equal tick distribution inferred from total and clock, not independently observed Retail tick amounts; aura4 Base100/Base7, movement immunity/attack restrictions remain uncertified.|
|288733 Intangibility|Native s2 adds25-percent total; seven ticks give497 after ordinary rounding. Both ranks snapshot; dropping rank later does not alter ticks.|No invented universal modifier; other talent variants uncertified.|
|453729 Altered Form|Existing native duration modifier adds2000ms; variable872 scales total by8/6; nine native ticks give333 after rounding. Combined ranks/cancellation proved.|Generic Duration machinery remains generic, not a new named script count.|
|80240 Havoc|Native50-percent effective direct single-target Warlock copy, no double source versatility/crit or recursion; immunity/full primary absorb/overkill, periodic/area/foreign-owner/replaced/expired exclusions.|Range, specialized child selectors and destination-defense/live parity uncertified. Native mark Dummy root closes, not universal spell parity.|
|31850 Ardent Defender|Native20-percent-max-health fatal-hit restoration, consumes once; nonfatal preservation, spell/direct/redirect shared health path and synchronous melee reservation before deferred removal.|Random melee outcome/live timing and aura316/118/137 semantics uncertified. Dummy field root closes, not all non-Dummy behavior.|

## Census and sources

Frozen f1b303a→173955c independently reproduces **163/103/60→160/104/56** (remaining Dummy roots/reviewed/underived). Flurry's numeric count, Havoc's numeric copy and Ardent's fatal-hit field are three whole *Dummy-census* root closures, not complete spell parity. E3 114/122→113/121; aura4 66/74→64/72; aura22618/19 unchanged.13 classes/40 specs,630 book IDs/1551 memberships,302 core IDs/355 memberships unchanged. No generic Dummy no-op or marker deletion.

**277 native SQLite rows** exactly match pinned local Mainline12.1.0.69933 CSV columns (zero differences;1e-10 numeric tolerance/exact strings). Variable394 and872 rows/mappings have separate hash receipts. TDB1210 registrations provide names/routes only, not C++ algorithms. Tests load real SpellStore child clones; only fixture Variance pinned. Positions, stats and health are fixture inputs, not fabricated runtime IDs or coefficients. No world.db write, external/C++/CDN source, live server or UDP5000.

Prior Karma redirect/Pandemic/Lifebloom near-expiry/Singularity area/unknown markers/guardian/live limits remain as [batch18](spell-dummy18-coverage.md), not silently closed. Native spell/rank counts are bounded implementations; source-derived total distribution and destination-defense limits are stated explicitly.

## Proof ledger

| Revision | Proof / limits |
|---|---|
|6be1d3a|Initial8 failures: Havoc flight too short and native extra-initial tick count wrong; not accepted as eight behavioral REDs.|
|ae4418d|Corrected **8/8 behavioral RED**: four instead of three bolts, full offhand,1750/2250 healing rather than native totals, absent Horror/Havoc/fatal rescue.|
|a83a1af|**8/8 GREEN**, compiler warning-free.|
|c9b356d|Boundaries **12/14**: Rain of Fire area-owned single-selected child incorrectly copied (genuine RED); expiry fixture stopped at12.5sec, not native15sec.|
|173955c|Area child exclusion and native-duration fixture: **14/14 GREEN**, compiler warning-free.|

Feature/master integrationf5d60ca: `git merge origin/master` completed (already up to date atf1b303a). Requested `cargo test -p server --bin game-server spell_cast` ran **ONCE:418/418**, including all14 batch19 functions, compiler warning-free. `cargo fmt --all --check` and `cargo check --locked -p server --bin game-server` pass atf5d60ca. Later edits docs/proof only, no invalidation. Manual changed-Rust readability audit; metrics executable unavailable. No full-crate/random-melee/live/full-rotation or independent verifier claim. Parent owns independent integration acceptance. Durable evidence: `game-engine/data/diagnostics/spell-dummy19-2026-10-11/` — exact source snapshots, source equivalence/metadata receipts, census/all40 generators/results and RED/GREEN logs.

## All40-spec preference table

Counts mean named script presence, not generic dispatch or full ability parity. Every batch18 ID set/count reproduced before additions. Altered Form stays generic Duration handling.

| Spec | Book script roots before→after | Core script roots before→after | Core A before→after |
|---|---:|---:|---:|
| Mage Arcane (62) | 3→3 | 4→4 | 5→5 |
| Mage Fire (63) | 1→1 | 3→3 | 5→5 |
| Mage Frost (64) | 1→1 | 4→5 | 4→5 |
| Paladin Holy (65) | 8→8 | 5→5 | 7→7 |
| Paladin Protection (66) | 6→6 | 4→5 | 5→6 |
| Paladin Retribution (70) | 6→6 | 4→4 | 5→5 |
| Warrior Arms (71) | 2→2 | 4→4 | 7→7 |
| Warrior Fury (72) | 3→3 | 4→4 | 6→6 |
| Warrior Protection (73) | 3→3 | 4→4 | 5→5 |
| Druid Balance (102) | 6→6 | 5→5 | 5→5 |
| Druid Feral (103) | 5→5 | 4→4 | 5→5 |
| Druid Guardian (104) | 5→5 | 3→3 | 4→4 |
| Druid Restoration (105) | 5→5 | 5→5 | 5→5 |
| Death Knight Blood (250) | 2→2 | 4→4 | 5→5 |
| Death Knight Frost (251) | 2→2 | 4→4 | 5→5 |
| Death Knight Unholy (252) | 2→2 | 3→3 | 4→4 |
| Hunter Beast Mastery (253) | 2→2 | 4→4 | 5→5 |
| Hunter Marksmanship (254) | 2→2 | 4→4 | 4→4 |
| Hunter Survival (255) | 2→2 | 4→4 | 4→4 |
| Priest Discipline (256) | 2→2 | 3→3 | 6→6 |
| Priest Holy (257) | 1→1 | 4→4 | 6→6 |
| Priest Shadow (258) | 1→3 | 3→5 | 7→8 |
| Rogue Assassination (259) | 4→5 | 4→5 | 7→8 |
| Rogue Outlaw (260) | 4→4 | 3→3 | 5→5 |
| Rogue Subtlety (261) | 4→4 | 4→4 | 7→7 |
| Shaman Elemental (262) | 1→1 | 4→4 | 5→5 |
| Shaman Enhancement (263) | 2→2 | 4→4 | 6→6 |
| Shaman Restoration (264) | 2→2 | 4→4 | 5→5 |
| Warlock Affliction (265) | 1→1 | 4→4 | 5→5 |
| Warlock Demonology (266) | 1→1 | 3→3 | 5→5 |
| Warlock Destruction (267) | 2→2 | 4→5 | 6→7 |
| Monk Brewmaster (268) | 3→3 | 5→5 | 6→6 |
| Monk Windwalker (269) | 4→4 | 4→4 | 5→5 |
| Monk Mistweaver (270) | 4→4 | 4→4 | 4→4 |
| Demon Hunter Havoc (577) | 6→6 | 4→4 | 5→5 |
| Demon Hunter Vengeance (581) | 5→5 | 4→4 | 4→4 |
| Evoker Devastation (1467) | 4→4 | 4→4 | 5→5 |
| Evoker Preservation (1468) | 4→4 | 3→3 | 4→4 |
| Evoker Augmentation (1473) | 6→6 | 3→3 | 4→4 |
| Demon Hunter Devourer (1480) | 7→7 | 3→3 | 4→4 |
