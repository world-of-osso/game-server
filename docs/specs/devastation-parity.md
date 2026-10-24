# Native Devastation parity

Devastation1467 must execute its DPS rotation natively and measure within10% of pinned SimC76f8118 on fixed300s single-target and replacement-add three-target fixtures. Native70448 DB2 values and TC a352b1fa remain authoritative. [Shared measurement contract](dps-parity.md); [native empower pipeline](empowered-spells.md).

## What it must do

- [ ] Preserve strict native trait legality. Under `SIMC_PROFILE_TRAIT_EDGE_DIVERGENCE`, omit only node98931/entry122279 from both native inputs and the derived SimC profile (`hero_talents=122279:0`), with no replacement. Preserve all remaining allocated/effective ranks, gear and APL text.
- [ ] Apply native362980's actual mastery amount once to direct and periodic Evoker-family224 damage. Reuse ordinary damage-modifier exclusions and never inherit owner mastery onto a pet.
- [ ] Under `DEVASTATION_GIANTKILLER_SIMC_POLICY`, scale mastery by target health fraction with a30% floor. Native Tyranny376888 gives full mastery during Dragonrage375087; the named secondary damage spells353759,441172 and1292322 retain their pinned full-mastery exception when Tyranny is active. Removing mastery/Tyranny or using another class's spell preserves the appropriate unmodified path. Healing receives no damage multiplier.
- [ ] Interpret the literal reference APL through the shared `simc_apl`/`simc_apl_native` path. Unsupported facts/options remain cited policy gaps, never substitute a local priority list or invented facts. Generic Fire Breath DoT refreshability uses its fresh-state native rank1-equivalent duration and current Blast Furnace, including ordinary duration modifiers—not raw child duration, last-cast carry or a proposed higher rank. The distinct `action.fire_breath.release.dot_duration` operand uses the proposed eligible empower rank. Pinned `dot.cpp458–477` creates a fresh state; `sc_evoker.cpp204–217,5291–5300` leaves its zero empower with no duration reduction, while5450–5492 explicitly seeds the release-expression rank.
- [ ] Qualify every reference damage channel and achieve±10% native DPS on both scenarios, preserving per-ability/stat/buff and per-target receipts.

## How it works

- [Shared APL interpreter and native facts](../wiki/systems/simc-apl.md).
- [Damage and passive-aura pipeline](../wiki/systems/auras.md).

## Implementation inventory

- `crates/server/src/spell_cast/scripts/giantkiller.rs` — native mastery amount and bounded target-health policy.
- `crates/server/src/spell_cast/effects.rs` — ordinary direct/periodic source-damage integration, excluding already-resolved copies.
- `crates/server/src/followers/giantkiller_tests.rs` — real level90 Kybel damage and native passive lifetime controls.
- `crates/server/src/dungeon_simulator/dps_parity/` — exact-gear, strict-trait300s measurement.
- `dps_parity/devastation_rotation/76f8118.simc` — all89 pinned action rows, unchanged order/options; APL-text SHA25648b025417cf6cd1c94416c247e8fe2c8b995f1a14f300d7d91046a6b94be4af3. This is preserved source data, not execution qualification.
- `crates/server-simulation/src/dungeon_simulator/dps_parity/devastation_rotation/{bindings,facts,tests}.rs` — native alias registry and shared action/target-context forwarding. The focused Living Flame leaf case checks real native context, CastSuccess and landed damage; it is not the full precombat/controller or89-row proof. Shared trinket/set/ilvl gaps remain explicit errors until their owner supplies the native facts.
- `auras/causality.rs` — TC's application-time load condition, live caster effect0 and actual empower recovery modification on Disintegrate periodic damage; no invented reduction.

## Tests asserting this spec

Four ignored `giantkiller_native_*` cases were pushed at3caafcc03 before implementation. Frozen0d07469b8 execution produced genuine4RED (22.30s): actual Flame2995 versus3354.4, Disintegrate2090 versus2340.8 and Deep Breath DoT2598 versus2909.76 with actual12% native mastery. These failures establish absent mastery consequences. Frozen f50689cfa execution passed all four cases in25.40s: health100/50/10% and30%floor, ordinary Tyranny/Dragonrage controls, other-family exclusion, and all three secondary exceptions. Tests remain opt-in because they require frozen native inputs. Evidence: `game-engine/data/diagnostics/dev-evoker-parity-2026-10-22/giantkiller-green-f50689cfa.log`.

Four ignored `native_causality_*` cases use ordinary native empower casts to start recovery, then actual Disintegrate periodic events and the selected Causality375777 effect0 amount. They cover tick-zero/later ticks, unrelated Dragonrage recovery, passive absence/removal, and TC's application-time `Load` boundary across late restoration and the next channel. No cooldown history writes. The first cbbed51bf run reached four invalid setup failures before Disintegrate: Eternity Surge was supplied self instead of its actual enemy target. That caller is corrected without changing native validation. At61218bfda, three cases reach genuine missing-reduction RED and passive-absence control passes (30.33s): Fire Breath25.65s versus25.15s after the actual native500ms effect, and the next loaded channel misses2.5s across five ticks. The minimal TC consequence and periodic-damage callback pass all four cases atbf0fd2c21 in26.44s (compile22m54, zero compiler warnings) on the unchanged paired native inputs. This qualifies the Causality consequence, not literal APL dispatch or DPS parity. Evidence: `game-engine/data/diagnostics/dev-evoker-parity-2026-10-22/dev-simulation-mixed-bf0fd2c21-native_causality.log`. TC a352b1fa `spell_evoker.cpp215–239` supplies the consequence.

Two ignored `literal_dev_native_*` cases run the unchanged native300s ST/3T fixture and inspect actual caster CastSuccess ordering: the source's unconditional precombat Living Flame must complete before native Fire Breath/Eternity Surge. They require a real Fire Breath cast too; labels or a lone synthetic precombat receipt cannot satisfy them. At cbbed51bf both reach genuine ordering RED in20.87s: native357208 precedes361469 on ST and3T. Native empower execution and follower identity premises pass. Dev controller implementation is still open.

## Known gaps (current cycle)

- [x] Giantkiller targeted GREEN and full floor/Tyranny/exclusion qualification atf50689cfa (4/4,25.40s).
- [ ] Literal APL dispatch, remaining native damage channels, consumables and final300s parity. Historical pre-Giantkiller1cf83f408 measures15650.57ST/20146.43three-target versus113727.53/182444.53 (−86.24%/−88.96%); these are diagnosis, not acceptance.

## Native build divergences

`DEVASTATION_REFINED_ESSENCE_BUILD_DIVERGENCE`: native70448 `SpellEffect` for1261452/EffectIndex1 is aura108, operation22 (`PeriodicHealingAndDamage`), base+15%. Pinned SimC70391 applies+25% to Disintegrate. Preserve native+15%; never tune it to the oracle. Evidence: `game-engine/data/diagnostics/dev-evoker-parity-2026-10-22/disintegrate-factor-source-receipt.json` records read-only native rows and the no-raid oracle factors/hash. This delta is data provenance, not a missing consequence or a parity-band waiver. Retire when the authenticated source builds agree.

Arcane Intensity375618's base8% versus SimC16% is not this divergence: selected rank2 requires native rank-derived effect points. The shared trait-rank lifecycle owns that correction. Post-tp1034 regressions in `dps_parity/trait_rechecks.rs` assert actual selected rank2 and effect0=16% (Set curves54429/64806); effect1 remains its native8% because it has no point row. A separate opt-in Feral witness uses native Tireless Energy383352/definition108190/Multiply curve58943 to assert the40-point bonus and actual140 Energy. Both rechecks pass at235a31cdf (13.44s) on source56820001/private backupdd1de231. They require current canonical trait-point rows; historical pre-import snapshots are not a substitute.

## Out of scope

No coefficient/stat tuning, canonical data changes, unrelated specialization scripts, alternate APL evaluator or claim that a SimC policy is an implemented TC script. TC a352b1fa has no362980 handler. DB2 supplies the mastery amount; pinned `sc_evoker.cpp4613–4629,5721,6165,7449` supplies only the explicit health/Tyranny application policy. Retire this policy when authenticated native behavior replaces those formula assumptions.
