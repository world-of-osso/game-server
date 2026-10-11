# Class spell batch17 — 2026-10-11

Eight source-backed low-count roots: Demonology2 and Restoration Druid/Blood/Destruction3 baseline core scripts. [Contract](specs/spell-dummy-scripts.md#seventeenth-batch-named-resources-and-restorative-hooks), [runtime](wiki/systems/spell-casting.md#seventeenth-batch-named-resources-and-restorative-hooks), [census](spell-coverage.md). No guardian edits, fabricated rows/numbers, DB imports or realm claim.

## Capability matrix

| Entry | Done / native behavioural proof | Missing / uncertified |
|---|---|---|
|686 Shadow Bolt|One194192/10 raw shard grant at completed cast only in actual Demonology266; all other/no specs excluded; native cancellation/completion proved.|Shadow Embrace32388 and full talent/proc variants. Shared classbook counts are conditional script presence, not offspec shard generation.|
|17962 Conflagrate|245330/5 fragments on actual damage;196406-gated117828 use, native cap2 after three casts. Two real completed Incinerates each take one use at30%, not60%; third returns native2000ms. Immune hit grants no fragments/buff.|267115 additional grant/cap4 and other talents; Soul Fire/Chaos Bolt matching-mask consumption not separately certified.|
|6353 Soul Fire|281490/10 raw units once on completed cast; base157736 periodic mark or445474 with445465, never both. Periodic damage sampled; cancellation excluded; immune hit still grants completion resource but applies neither mark.|Own Dummy2000/10000 fields; optional vulnerabilities/tick resources and any additional direct Immolate/Wither hit. Only named base periodic effects are bridged.|
|50842 Blood Boil|Owned55078 base disease on each damage-hit enemy; allies/far/immune excluded. Native description's learned212744 replacement branch excludes it without synthesizing its absent local row.|Periodic leech53 and zero4/271/381/380 markers. Application is not tick/heal parity. New dependency adds a retained census root.|
|195182 Marrowrend|Native s3 grants three195181 stacks; repeated real roots reach3/6/9/10, never above native cap; immune primary excluded.|Own Dummy0 Base1, Bone Shield melee charge depletion/haste/markers. Root is outside frozen core probes.|
|33763 Lifebloom|33778/219 heal on expiry/dispel, never cancellation/death/replacement. Natural15sec expiry after periodic ticks blooms once. New target removes only the same owner's previous aura; foreign owner survives.|Near-expiration refresh threshold absent from inspected local rows; no guessed30%/4.5sec rule. Full talent/refresh parity unclaimed.|
|8936 Improved Regrowth|231032's native40% probability only with an existing HoT on the target, including foreign-owner HoTs; deterministic60+40=100 critical initial heal, absent-rank/no-HoT0% exclusions.|Tree of Life/other talents and full periodic normalization. No invented ownership restriction.|
|102351 Cenarion Ward|Real incoming Fireball consumes native charge and applies102352 from owner onto protected ally, not attacker. Heals/fully absorbed damage leave ward intact. Native8000ms/2000ms + EXTRA_INITIAL_PERIOD512 gives five heals, expiry/no retrigger.|Melee/periodic/triggered incoming boundaries not separately cast-certified; no tooltip/UI total certificate. Root's sole4 field has its named action, but this root is outside frozen core probes.|

## Census / all40 scope

Frozen592cf28 ->76c4591: **163 roots/99 reviewed/64 underived ->164/100/64**. No whole *existing core-census* root closes. Cenarion Ward's named damage-to-HoT script is resolved outside the frozen core set; Blood Boil introduces55078's still-unimplemented Dummy descendant, which must not be hidden. E3 stays114 IDs/122 memberships; aura4 increases66/74 ->67/75; aura226 stays18/19. Fixed13 classes/40 specs,630 book IDs/1551 memberships and302 core IDs/355 memberships retained.

Every prior40-spec ID set/count is independently reproduced, with already-integrated guardian205180 included in Affliction baseline4. New core script presence: Demonology2->3, Restoration Druid3->5, Blood3->4, Destruction3->4. Lowest-count Shadow retains its underivable marker/leech/variant gaps; no guessed closure. Existing Demonbolt/Voidform already implement proposed consumers and are not duplicated. Heart Strike's316575 and negative power-cost/modifier34 interaction remains underived, not given a second guessed resource grant.

Karma redirect, guardian inheritance/stamina/melee, Eye Beam11-vs-tooltip10/uncertified live facing and Deathmark80-Energy live total remain unchanged. No local TrinityCore C++ body, external sources, CDN or fabricated guardian used.

## Sources / proof ledger

429 native SQLite rows exactly match all corresponding pinned Mainline12.1.0.69933 CSV columns; zero differences, numeric tolerance1e-10, exact strings/newlines. Native names, effects, costs, cast/duration/radius/options/restrictions/masks/specs and TDB1210 spell_script_names are receipted. Archive registrations establish names/routes, not missing C++ bodies or coefficients. Child spells come from the real store; only fixture Variance is zeroed for deterministic amounts, never IDs/coefficients/clocks/costs.

| Revision | Proof / limit |
|---|---|
|e11bc1e|Initial RED compile errors in combat-log fixture fields; not behavioural RED.|
|8b9e8eb|10 failures; one probabilistic Regrowth pass invalid for RED, and two fixtures had missing-row/PvP assumptions.|
|3926de7|Corrected deterministic **11/11 RED**: genuine absent grants/marks/blooms, probability60 vs100, ward incorrectly consumed by a heal.|
|a17570e|Production10/11 GREEN; remaining bloom expected219 was affected by native random variance.|
|8933e46|Variance-only fixture pins/real charged cast checks;11/13 GREEN. New immune-hit proof exposes absent ledger checks; guessed four Ward ticks contradict native extra-initial attribute.|
|76c4591|Root-specific landed-hit predicates and native initial+four clock: **13/13 GREEN**, compiler warning-free. No suppressed unsupported leech claim.|
|5f8ee8e|Added actual HostClient completion/cancellation fixture: **1/1 GREEN**, compiler warning-free; previous13 production proofs remain valid.|

Final integration gate pending before publication. Manual changed-Rust readability audit: named short helpers, shared stacked-use path, no warning suppressions; metric executable unavailable. No broad/full-crate/live/independent acceptance claim; parent owns integration acceptance.

Durable evidence: `game-engine/data/diagnostics/spell-dummy17-2026-10-11/` — frozen sources, source/equivalence receipts, reproducible census/table generators, RED/GREEN/completion logs and proof ledger.

## All40-spec preference table

Counts mean bounded production script presence, not full ability parity or universal rank learning. Shared classbook roots can be counted outside their script's enabled spec. No additional probes added merely to inflate a spec's count.

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
| Warrior Protection (73) | 3→3 | 3→3 | 4→4 |
| Druid Balance (102) | 5→6 | 5→5 | 5→5 |
| Druid Feral (103) | 4→5 | 4→4 | 5→5 |
| Druid Guardian (104) | 4→5 | 3→3 | 4→4 |
| Druid Restoration (105) | 4→5 | 3→5 | 3→5 |
| Death Knight Blood (250) | 2→2 | 3→4 | 4→5 |
| Death Knight Frost (251) | 2→2 | 3→3 | 5→5 |
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
| Shaman Restoration (264) | 2→2 | 4→4 | 5→5 |
| Warlock Affliction (265) | 0→1 | 4→4 | 5→5 |
| Warlock Demonology (266) | 0→1 | 2→3 | 4→5 |
| Warlock Destruction (267) | 1→2 | 3→4 | 5→6 |
| Monk Brewmaster (268) | 3→3 | 5→5 | 6→6 |
| Monk Windwalker (269) | 4→4 | 4→4 | 5→5 |
| Monk Mistweaver (270) | 4→4 | 4→4 | 4→4 |
| Demon Hunter Havoc (577) | 6→6 | 4→4 | 5→5 |
| Demon Hunter Vengeance (581) | 5→5 | 4→4 | 4→4 |
| Evoker Devastation (1467) | 4→4 | 4→4 | 5→5 |
| Evoker Preservation (1468) | 4→4 | 3→3 | 4→4 |
| Evoker Augmentation (1473) | 6→6 | 3→3 | 4→4 |
| Demon Hunter Devourer (1480) | 7→7 | 3→3 | 4→4 |
