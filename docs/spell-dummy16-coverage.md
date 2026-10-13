# Class spell batch16 — 2026-10-10

Nine bounded Mainline12.1.0.69933 entries prioritize Arcane, Holy/Ret Paladin and Restoration Shaman's low script counts. [Contract](specs/spell-dummy-scripts.md#sixteenth-batch-low-count-rank-and-charge-contracts), [runtime](wiki/systems/spell-casting.md#sixteenth-batch-low-count-rank-and-charge-consumers), [census](spell-coverage.md). Guardian modules are untouched; no fabricated IDs, coefficients, actors or database imports.

## Capabilities / limits

| Entry | Implemented / behavioral proof | Missing / uncertified |
|---|---|---|
|30451 Arcane Blast|Pre-gain charge damage uses36032s1/60%; cost s5/100% per charge; cast time s4/-8% per charge. Native0/2/4 charge damage/cost/bar and unaffordable-refusal proof.|Charge-aura presentation, mastery/talents and arbitrary mid-cast charge changes. No guessed aura synchronization.|
|12042 Arcane Power|Native s2/-30% aura72 school/mana masks discount cast cost once, with or without343208; absent12042 preserves base cost and native expiry restores it. TC a352b1fa SpellInfo.cpp:4185–4194 supplies the generic rule.|Channel upkeep/other power costs and full ability parity.|
|82326 Holy Light|231642 gates named88852's one Holy Power once per completed cast, including overhealing.|Infusion/talents and all healing variants.|
|20473 Holy Shock|196926s1/-1500ms reduces only running Holy Judgment275773; ready timer untouched and owner update proved.|Offspec Judgment variants, full talent/crit semantics. Existing heal/damage bridge retained.|
|35395 Crusader Strike|196926s2/-1500ms reduces running275773; rank-absent/ready boundaries and owner update proved.|Other Judgment variants and damage/talent conversions.|
|20271 Ret Judgment|315867 gates named220637's one Holy Power on actual primary damage, not latent splash; immune primary excluded.|231663/197277 owned next-spender mark, secondary effects and other talents.|
|275773 Holy Judgment|Same named315867/220637 gain and immune-hit exclusion, through its own native root.|231644 shield, Infusion and other talents. Protection275779's existing script unchanged.|
|61295 Riptide|51564s1 grants named53390 uses; native cap2; successive Healing Wave/Chain Heal each consume one stack at native20% cast reduction, never40%. No rank gives no buff.|Other talents and full class proc semantics. Native one-charge field remains until final accumulated use.|
|77472 Healing Wave|Named native45/chainTargets0 plan admits a single validated friendly heal. Real1501 healing at100 healing power; nearby ally/enemy excluded.|Generic selector45 and talent-added jumps. Existing friendly-cast self selection is unchanged, not newly invented.|

Karma redirect, marker/talent gaps, Eye Beam native11-vs-tooltip10 discrepancy and named-guardian/stat/melee work remain unchanged. Demonology actor candidates were not substituted with Imps or unbound guardians.

## Census / authentic sources

Frozen8ff636a before and9721cec after both reproduce **163 remaining Dummy roots,98 reviewed/65 underived**. **No whole census root closes**: the new consumers improve ordinary casts and their named conditional ranks; retained census markers/variants have no newly established action. E3=114 IDs/122 memberships; aura4=66/74; aura226=18/19. No generic zero-Dummy/no-op closure. Fixed scope remains13 classes/40 specs,630 book IDs/1551 memberships,302 core IDs/355 memberships.

229 native SQLite rows match every corresponding local CSV column: zero missing/different rows, tolerance1e-10, exact strings/newlines. Description/effect/cost/cast/duration/radius/options/class-mask receipts use only local DB2 and read-only canonical world.db. No external sources or TrinityCore C++ bodies used. Every previous40-spec ID set/count is reproduced exactly. Rank fixtures restrict KnownSpells separately from loading authentic child store rows; no fabricated child fixture.

## Proof ledger

-7152816 initial fixture did not compile (wrong SpellHistory field);6d1e1cf fixes field and records9 failures. Arcane Power's initial expected Fireball cost was wrong, not valid rank RED.
-5173154 uses native1% Fireball mana cost and native2sec Shaman cast duration. Corrected Arcane Power rank case fails100 vs70. Other REDs fail absent native consumers.
-5a4e48a implementation:8/9 targeted pass.64fb234 diagnostic shows Healing Wave45 fails at cast admission, not Tidal consumption.
-d060007 adds named Healing Wave single-primary plan, needed for genuine Tidal use testing.9/10 pass; remaining fixture wrongly expected hostile friendly-cast selection to reject rather than existing self selection.
-9721cec corrects that observable selection assertion and adds native expiry/immune boundaries. **12/12 targeted pass**, warning-free. Current production/source/census proof covers this revision. Manual readability audit: short named new functions, no suppressions; expanded existing cost pipeline remains long, no unrelated refactor. Automated metric tool unavailable.
-e0fa018 merges origin/master into this feature branch (wiki index conflict resolved by preserving both entries). Requested `cargo test -p server --bin game-server spell_cast` runs ONCE: **372/372 pass**, warning-free. `cargo fmt --all --check` and `cargo check --locked -p server --bin game-server` pass. A later pre-push merge imports origin/master c644eaa's three unrelated toy visual IDs128327/196783/318452. At156cd37, fresh bounded12/12 and fmt/check pass. No repeated broad gate or fresh372/372 claim on this later revision; no live/full-rotation/independent acceptance claim. Final changes after156cd37 are docs/proof only.

Durable evidence: `game-engine/data/diagnostics/spell-dummy16-2026-10-10/` — source tars/generators, native row/equivalence receipts, exact census/all40 before/after, RED/GREEN logs and proof ledger. Parent owns independent integration acceptance.

## All40-spec preference table

Counts mean bounded production script presence, not whole ability parity. Existing Holy Shock is improved without inflating its script count; shared classbook bindings can appear outside the owning spec. No count claims that every character learns a rank.

| Spec | Book script roots before→after | Core script roots before→after | Core A before→after |
|---|---:|---:|---:|
| Mage Arcane (62) | 2→3 | 2→4 | 3→5 |
| Mage Fire (63) | 1→1 | 3→3 | 5→5 |
| Mage Frost (64) | 1→1 | 4→4 | 4→4 |
| Paladin Holy (65) | 4→8 | 3→5 | 5→7 |
| Paladin Protection (66) | 4→6 | 4→4 | 5→5 |
| Paladin Retribution (70) | 4→6 | 2→4 | 3→5 |
| Warrior Arms (71) | 2→2 | 4→4 | 7→7 |
| Warrior Fury (72) | 3→3 | 4→4 | 6→6 |
| Warrior Protection (73) | 3→3 | 3→3 | 4→4 |
| Druid Balance (102) | 5→5 | 5→5 | 5→5 |
| Druid Feral (103) | 4→4 | 4→4 | 5→5 |
| Druid Guardian (104) | 4→4 | 3→3 | 4→4 |
| Druid Restoration (105) | 4→4 | 3→3 | 3→3 |
| Death Knight Blood (250) | 2→2 | 3→3 | 4→4 |
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
| Shaman Restoration (264) | 1→2 | 2→4 | 3→5 |
| Warlock Affliction (265) | 0→0 | 3→3 | 5→5 |
| Warlock Demonology (266) | 0→0 | 2→2 | 4→4 |
| Warlock Destruction (267) | 1→1 | 3→3 | 5→5 |
| Monk Brewmaster (268) | 3→3 | 5→5 | 6→6 |
| Monk Windwalker (269) | 4→4 | 4→4 | 5→5 |
| Monk Mistweaver (270) | 4→4 | 4→4 | 4→4 |
| Demon Hunter Havoc (577) | 6→6 | 4→4 | 5→5 |
| Demon Hunter Vengeance (581) | 5→5 | 4→4 | 4→4 |
| Evoker Devastation (1467) | 4→4 | 4→4 | 5→5 |
| Evoker Preservation (1468) | 4→4 | 3→3 | 4→4 |
| Evoker Augmentation (1473) | 6→6 | 3→3 | 4→4 |
| Demon Hunter Devourer (1480) | 7→7 | 3→3 | 4→4 |
