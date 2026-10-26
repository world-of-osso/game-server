# Native Devastation parity

Devastation1467 must execute its DPS rotation natively and measure within10% of pinned SimC76f8118 on fixed300s single-target and replacement-add three-target fixtures. Native70448 DB2 values and TC a352b1fa remain authoritative. [Shared measurement contract](dps-parity.md); [native empower pipeline](empowered-spells.md).

## What it must do

- [ ] Preserve strict native trait legality. Under `SIMC_PROFILE_TRAIT_EDGE_DIVERGENCE`, omit only node98931/entry122279 from both native inputs and the derived SimC profile (`hero_talents=122279:0`), with no replacement. Preserve all remaining allocated/effective ranks, gear and APL text.
- [ ] Apply native362980's actual mastery amount once to direct and periodic Evoker-family224 damage. Reuse ordinary damage-modifier exclusions and never inherit owner mastery onto a pet.
- [ ] Under `DEVASTATION_GIANTKILLER_SIMC_POLICY`, scale mastery by target health fraction with a30% floor. Native Tyranny376888 gives full mastery during Dragonrage375087; the named secondary damage spells353759,441172 and1292322 retain their pinned full-mastery exception when Tyranny is active. Removing mastery/Tyranny or using another class's spell preserves the appropriate unmodified path. Healing receives no damage multiplier.
- [ ] Interpret the literal reference APL through the shared `simc_apl`/`simc_apl_native` path. Unsupported facts/options remain cited policy gaps, never substitute a local priority list or invented facts. Fire Breath refreshability uses fresh native release duration for the proposed eligible empower rank and current Blast Furnace, including ordinary duration modifiers—not the raw child spell duration or a carried refresh duration.
- [ ] `SIMC_DEVASTATION_EMPOWER_MASS_DISINTEGRATE`: successful native Fire Breath and Eternity Surge releases grant one application of native436336 only with active native436335. Preserve the native stack cap, duration and modifier amounts; a failed or unselected release cannot grant it. SimC76f8118 `sc_evoker.cpp4703–4704` supplies the producer relationship, not values. `SIMC_DEVASTATION_MASS_DISINTEGRATE_CONSUMER` spends exactly one earned use after a successful Disintegrate. Native ChainTargets operation17 applies the buff's flat additional targets before chain search; stacked uses do not multiply that amount. Preserve native chain geometry and per-recipient damage. `SIMC_DEVASTATION_MASS_DISINTEGRATE_REDISTRIBUTION` snapshots native436335 effect1 per missing native effect0 recipient from the admitted launch plan, excluding the caster-only helper aura. Retain that factor in the real periodic payload after use consumption (`sc_evoker.cpp6560–6567`); a full pack gains no missing-recipient bonus.
- [ ] `SIMC_DEVASTATION_MANEUVERABILITY_ACTION` resolves learned357210 to native433874 only with learned433871 through shared native action knowledge (`sc_evoker.cpp6323–6327`). Resolve the native caster-only endpoint's cast unit to the actor while retaining the selected hostile primary for its payload. `SIMC_DEVASTATION_DEEP_BREATH_PAYLOAD`: ordinary native357210 and selected Maneuverability433874 dispatch native353759 at the retained hostile primary. Preserve native periodic amounts, duration, area recipients and Giantkiller behavior. Pinned `sc_evoker.cpp6241–6245` supplies the missing payload relationship; `SIMC_DEVASTATION_MANEUVERABILITY_MELT_ARMOR` applies native441172 to the actual353759 impact recipients with live433871; only live441176 enables the payload's debuff effect. Native amounts, period and ordinary full-mastery policy remain unchanged. Overlapping burns use sourced extension behavior (`sc_evoker.cpp5712–5728/6219–6222/6252–6253`). Command Squadron remains separate qualification.
- [ ] `SIMC_DEVASTATION_SURGE_AZURE_SWEEP`: starting native Eternity Surge359073/382411 with selected1265867 grants native1265871. Its native aura332 exposes1265872 through known Azure Strike; retain native modifiers, charges and area damage. Pinned `sc_evoker.cpp5592–5599` supplies the producer relationship. Native MID1 four-piece1264822 supplies its additional use through label5988/Charges31 and its effect1 milliseconds reduce the running Surge cooldown after each Sweep. Each cast spends one use without multiplying the buff's damage modifier; the last use restores Azure Strike. Pinned `sc_evoker.cpp5702–5708/10853–10856` supplies those relationships. `SIMC_DEVASTATION_MID1_SWEEP_PRIMARY` applies native two-piece1264821 effect1 only to the retained hostile primary, in addition to effect0's ordinary all-recipient modifier. Missing primary context never selects a substitute. Pinned `sc_evoker.cpp5691–5698` supplies this conditional application, not a coefficient override.
- [ ] `SIMC_DEVASTATION_CAUSALITY`: actual primary Disintegrate ticks with live native375777 reduce running Fire Breath and Eternity Surge recovery by that aura's native effect0 milliseconds (`sc_evoker.cpp6694–6699`). Retain the chosen primary through aura application, refresh and periodic reconstruction; secondary recipients, absent primary context and absent passive do not repeat or invent reductions.
- [ ] `SIMC_DEVASTATION_STRAFING_RUN`: successful native Deep Breath357210/433874 with active1266151 grants native1266165 only when no prior window exists; the next actual cast spends the old window instead of renewing it (`sc_evoker.cpp6344–6353/6384–6398`). Native aura411 grants the extra charge and aura500 preserves the running charge recovery. Do not reset, refill or replace the original recovery timer.
- [ ] `SIMC_DEVASTATION_SCINTILLATION`: each real Disintegrate recipient tick with live native370821 rolls its effect1 chance and launches a rank-one native359077 release at that tick recipient, at native effect0 effectiveness (`sc_evoker.cpp6669–6677`). Reuse ordinary rank/Span recipient selection, coefficients, flight and damage; do not spend another empower or grant its foreground-only buffs. `SIMC_DEVASTATION_SHATTERING_STARS` launches native1265804 from an actual Surge damage impact only with live1265802, at the retained primary unless live1265826 admits secondary recipients. Its effect0 scales the completed native empower rank; Scintillation's actual child effectiveness carries through (`sc_evoker.cpp5550–5573`). No primary or completed rank is invented.
- [ ] `SIMC_DEVASTATION_POWER_SWELL`: a successful native empower release with live370839 grants native376850 (`sc_evoker.cpp4684/11504–11507`). Its actual aura110/Essence19 percentage changes ordinary regeneration; do not refill resources or fabricate gains. Absent talent grants no buff.
- [ ] Qualify every reference damage channel and achieve±10% native DPS on both scenarios, preserving per-ability/stat/buff and per-target receipts.

## How it works

- [Shared APL interpreter and native facts](../wiki/systems/simc-apl.md).
- [Damage and passive-aura pipeline](../wiki/systems/auras.md).

## Implementation inventory

- `crates/server-spell-scripts/src/modules/giantkiller.rs` — native mastery amount and bounded target-health policy.
- `crates/server-spell-scripts/src/modules/evoker_tier.rs` — native Azure Sweep uses and MID1 Surge recovery.
- `crates/server-combat/src/spell_cast/effects.rs` — ordinary direct/periodic source-damage integration, excluding already-resolved copies.
- `crates/server-combat/src/followers/giantkiller_tests.rs` — real level90 Kybel damage and native passive lifetime controls.
- `crates/server-simulation/src/dungeon_simulator/dps_parity/` — exact-gear, strict-trait300s measurement.

## Tests asserting this spec

Four ignored `giantkiller_native_*` cases were pushed at3caafcc03 before implementation. Frozen0d07469b8 execution produced genuine4RED (22.30s): actual Flame2995 versus3354.4, Disintegrate2090 versus2340.8 and Deep Breath DoT2598 versus2909.76 with actual12% native mastery. These failures establish absent mastery consequences. Frozen f50689cfa execution passed all four cases in25.40s: health100/50/10% and30%floor, ordinary Tyranny/Dragonrage controls, other-family exclusion, and all three secondary exceptions. Tests remain opt-in because they require frozen native inputs. Evidence: `game-engine/data/diagnostics/dev-evoker-parity-2026-10-22/giantkiller-green-f50689cfa.log`.

## Known gaps (current cycle)

- [x] Giantkiller targeted GREEN and full floor/Tyranny/exclusion qualification atf50689cfa (4/4,25.40s).
- [ ] Literal APL dispatch, remaining native damage channels, consumables and final300s parity. Historical pre-Giantkiller1cf83f408 measures15650.57ST/20146.43three-target versus113727.53/182444.53 (−86.24%/−88.96%); these are diagnosis, not acceptance.

## Native build divergences

`DEVASTATION_REFINED_ESSENCE_BUILD_DIVERGENCE`: native70448 `SpellEffect` for1261452/EffectIndex1 is aura108, operation22 (`PeriodicHealingAndDamage`), base+15%. Pinned SimC70391 applies+25% to Disintegrate. Preserve native+15%; never tune it to the oracle. Evidence: `game-engine/data/diagnostics/dev-evoker-parity-2026-10-22/disintegrate-factor-source-receipt.json` records read-only native rows and the no-raid oracle factors/hash. This delta is data provenance, not a missing consequence or a parity-band waiver. Retire when the authenticated source builds agree.

Arcane Intensity375618's base8% versus SimC16% is not this divergence: selected rank2 requires native rank-derived effect points. The shared trait-rank lifecycle owns that correction; Dev must re-check its deployed data receipt before attributing a remaining numerical difference.

## Out of scope

No coefficient/stat tuning, canonical data changes, unrelated specialization scripts, alternate APL evaluator or claim that a SimC policy is an implemented TC script. TC a352b1fa has no362980 handler. DB2 supplies the mastery amount; pinned `sc_evoker.cpp4613–4629,5721,6165,7449` supplies only the explicit health/Tyranny application policy. Retire this policy when authenticated native behavior replaces those formula assumptions.
