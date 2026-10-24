# Class spell batch20 — 2026-10-11

**2026-10-24 correction:** Barbed's tooltip-only total inference and associated numeric-dummy closure are withdrawn; DB2/TC determine per-tick amounts and schedule. [Native-source RED/correction ledger](handoffs/bm-native-periodic-proof-ledger.md). Proof/census figures below are historical, not a refreshed current classification.

Eight previously underived core roots, prioritized around rotation/resource and defensive impact. Two numeric contracts implemented; six inspected fields explicitly deferred. [Contract](specs/spell-dummy-scripts.md#twentieth-batch-authored-totals-and-next-spell-uses), [runtime](wiki/systems/spell-casting.md#twentieth-batch-authored-totals-and-next-spell-uses), [matrix](spell-coverage.md).

## Capability matrix

| Root | Done / proof level | Missing / exact source boundary |
|---|---|---|
|217200 Barbed Shot|Native246152 Focus starts once per completed root. Damage uses generic DB2 per-tick amounts and native rolling refresh; the fixed-five damage-total inference is withdrawn. Actual geared Player per-tick/refresh RED and current implementation are tracked in the linked correction ledger.|Dummy5 has no independently sourced action; the former numeric-field closure is withdrawn. Current integration is pending; pet/talent/Latent Poison variants and whole-DPS parity remain uncertified.|
|191634 Stormkeeper|s5=2 initializes empowered-spell uses before ordinary charge modifiers, replacing options ProcCharges1 for this named aura. Real completed root, cancellation, two instant188196 and two instant188443 casts, unrelated188389 exclusion and third-bolt native cast time proved.|Elemental Overload on every Chain Lightning target, talent variants and cumulative-stack/overlap semantics remain uncertified; native CumulativeAura4 retained. Closure is the Dummy use-count only, not full spell parity.|
|47788 Guardian Spirit|Reviewed/deferred, source-level proof. Native s2=40 restores percent max HP, but description says “Cannot save the target from massive damage.”|No numerical massive-hit threshold in inspected native316 zero field, options, description-variable mappings or TDB registrations. No unconditional rescue or invented threshold implemented.|
|23922 Shield Slam|Reviewed/unidentified, source-level proof. Aura4 index3 Base15 equals the displayed Rage quantity from native energize index2 raw150.|Description refers to s3, not s4; no separate trigger/reference binds this mirror to an independent action. TDB improved_whirlwind_cleave name does not assign the15 field. No duplicate Rage grant.|
|1719 Recklessness|Reviewed/unidentified, source-level proof. E3 index1 Base0 has no target action/trigger/reference.|Rage50% and critical20% refer to separate213/107/108/218 effects. No options/PPM/variable/registration contract identifies an independent zero marker action.|
|51271 Pillar of Frost|Reviewed/unidentified, source-level proof. Aura4 indexes1/2/3 contain2/0/5, no triggers.|Description establishes index0 Strength20%; e5 describes optional Winter modifiers. No variable mapping, proc options or TDB route binds2/0/5 to rune spending or incremental Strength.|
|53385 Divine Storm|Reviewed/deferred, source-level proof. s2=5 explicitly says reduced damage beyond five targets.|No reduction curve in native row, description, variables or TDB divine_storm registration. No guessed square-root/linear curve or hard target cap.403460/407467 descendants retained.|
|19434 Aimed Shot|Reviewed/unidentified, source-level proof. E3 index1 Base50 has no target/trigger/variable mapping.|Description refers only to damage s1. TDB latent_poison_trigger name does not establish a50% action/proc for this marker. No invented damage bonus.|

Source quotations above come from local `data/world.db` `spell.Description_lang`, exactly checked against Mainline12.1.0.69933 `Spell.csv`; exact fields/absence receipts are in the durable evidence directory below. SQL registration names are route evidence only; no C++ algorithm or external source used.

## Census and source receipts

Frozen **14e4e64→57deae4** reproduces **160 roots/104 reviewed/56 underived →158/110/48**. Barbed Shot and Stormkeeper close their consumed numeric Dummy fields; six other roots move from unreviewed to reviewed/deferred, not implemented. E3 **113/121→111/119** IDs/memberships; aura4 **64/72** and aura226 **18/19** unchanged.13 classes/40 specs,630 book IDs/1551 memberships,302 core IDs/355 memberships retained. Every previous all40 script/proof ID set reproduced before additions.

**214 native SQLite rows** match pinned CSV columns with zero differences (exact strings,1e-10 numeric tolerance). Referenced duration/range/radius/cast-time rows included. All eight roots have no `SpellXDescriptionVariables` mapping in this pinned export; explicit absence and CSV hashes recorded. TDB1210 names only. Tests clone real SpellStore rows and pin only fixture Variance; actor AP/position/health are fixture inputs, not invented IDs, coefficients or clocks.

Prior Death Strike rolling-history, Karma redirect, Pandemic, Lifebloom near-expiry, Singularity area, guardian, unidentified markers and live limits remain as [batch19](spell-dummy19-coverage.md) and its linked prior batches. No generic Dummy no-op or deletion, world.db writes, external/CDN algorithms, master merge, deployment or live server/UDP5000.

## Proof ledger

| Revision | Command / result / scope |
|---|---|
|be6f091|Initial test compile blocked by shared pool QuestEntrySnapshot drift; not behavioral RED. Freeze local shared-protocol44b2b0f (last pre-quest revision) with Cargo config override; no shared checkout mutation.|
|be6f091, pinned shared|Two Stormkeeper count failures and missing Focus child are valid behavioral REDs. Initial Barbed assertion used insufficient flight/clock coverage and is not accepted rollover proof.|
|6eaaa2a|3/4 GREEN; Barbed clock investigated, not accepted full coverage. Instrumentation found native twelve-second duration, not assumed eight seconds.|
|6839bd2|Corrected Barbed RED: actual first ticks86/86 versus authored-total61/61. Native clock unchanged.|
|35ba866|4/4 GREEN, warning-free; inferred equal total distribution is explicit.|
|713c8a4|8/9 boundaries; foreign player's private combat log not delivered to this fixture's authenticated connection. Not a production damage failure.|
|7f02a25|9/9 GREEN, warning-free. Foreign-owner proof uses target health (14 independent61 hits) plus owner's received log, not another player's private inbox.|

Final code **57deae4** (pure unpaid-budget helper extraction): requested `cargo test --locked -p server --bin game-server spell_cast` ran **ONCE,427/427**, including all nine batch20 tests; exit0, compiler warning-free. `cargo fmt --all --check` and `cargo check --locked -p server --bin game-server` exit0, warning-free. All Cargo proof pins local shared44b2b0f. Later edits docs/proof only, no invalidation. Manual changed-Rust readability audit; metrics executable unavailable. Independent artifact verifier was attempted concurrently but unavailable: OAuth expired and refresh failed; no independent acceptance claimed. Parent retains integration acceptance. No full-server/live/full-rotation claim.

Durable evidence: `/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/spell-dummy20-2026-10-11/` — exact before/after source archives, pinned shared archive, native/variable receipts, census/all40 generators/results, corrected RED/GREEN logs and final proof ledger. `--config patch.\"https://github.com/world-of-osso/shared-protocol.git\".shared.path=\"<evidence>/shared-protocol\"` pins the verification dependency; current pool's quest-wire additions require unrelated server changes and are not silently integrated here.

## All40-spec preference table

Counts mean named script presence, not generic dispatch or full ability parity. Every batch19 ID set/count reproduced before additions. No generic handler relabelled as a new named script.

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
| Druid Feral (103) | 5→5 | 4→4 | 5→5 |
| Druid Guardian (104) | 5→5 | 3→3 | 4→4 |
| Druid Restoration (105) | 5→5 | 5→5 | 5→5 |
| Death Knight Blood (250) | 2→2 | 4→4 | 5→5 |
| Death Knight Frost (251) | 2→2 | 4→4 | 5→5 |
| Death Knight Unholy (252) | 2→2 | 3→3 | 4→4 |
| Hunter Beast Mastery (253) | 2→2 | 4→5 | 5→6 |
| Hunter Marksmanship (254) | 2→2 | 4→4 | 4→4 |
| Hunter Survival (255) | 2→2 | 4→4 | 4→4 |
| Priest Discipline (256) | 2→2 | 3→3 | 6→6 |
| Priest Holy (257) | 1→1 | 4→4 | 6→6 |
| Priest Shadow (258) | 3→3 | 5→5 | 8→8 |
| Rogue Assassination (259) | 5→5 | 5→5 | 8→8 |
| Rogue Outlaw (260) | 4→4 | 3→3 | 5→5 |
| Rogue Subtlety (261) | 4→4 | 4→4 | 7→7 |
| Shaman Elemental (262) | 1→1 | 4→5 | 5→6 |
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
