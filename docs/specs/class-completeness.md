# Level-80 class completeness audit and shared cast behavior

Source: `spell_cast`, `auras`, `spell_info`; [implementation and proof](../class-completeness.md), [spell-casting system](../wiki/systems/spell-casting.md#level80-class-completeness).

## What it must do

- [x] Record native cast/passive-sync outcomes, warning tuples and raw resources for all 13 classes/all 40 local Retail specs, using level-80 actors and real spell rows, without writing world.db or opening a realm port.
- [x] Preserve exact missing effect/aura types and source/state prerequisites; never close unidentified Dummy fields with no-ops or guessed coefficients.
- [x] Census uses authored production destinations/maps, actor identity and group state rather than reporting absent fixture startup resources as product gaps.
- [x] Native target24 selects hostile units within the authored caster-facing cone/radius.
- [ ] Aura229 reduces only area damage whose school overlaps its native mask, multiplying independent aura factors. Direct damage and periodic damage/leech use the actual effect's selector or area-aura classification, plus native treat-as-area flags; periodic IgnoreDamageTakenModifiers excludes this mitigation. Single-target siblings and removed/expired auras do not reduce damage.
- [ ] Native429433 Blooming Infusion counts completed Regrowth and Starsurge/eligible Starfall casts independently using its authored threshold, granting the corresponding native next-spell buff. Triggered copies, ticks and rejected/cancelled casts do not count. Native class masks determine consumption, and a charged damage modifier remains with an already launched missile after its charge is consumed.
- [ ] Native434249 Control of the Dream subtracts time spent ready (at maximum charges for charge abilities), capped by its authored seconds field, from the next cooldown of the named Druid major abilities. Unavailable time, unrelated spells and absent/foreign talent do not grant credit; natural/restored full-charge readiness begins at its real completion time.
- [ ] Zero-amount native LINKED284/LINKED_2 285 casts its declared child on application, using the original caster only when that child requires it. Reapplication synchronizes child stacks without recasting; parent removal/expiry removes only the matching caster's child. Nonzero amount conflicts remain named partials without invented sign/scaling rules.
- [x] Native433748 Protective Growth reduces spell and melee damage by its authored percentage only while the learner carries their own8936 Regrowth. Removal/expiry ends protection; foreign Regrowth and healing another target do not activate it.
- [ ] Native Chrono Flames retains Living Flame's damage/heal branch and repeats caster-to-target effective health changes over its authored history window, with native spec percentage, spell-power/versatility cap and Overclock modifier. No other actor, target, miss, overkill or overheal contributes. Source-derived landing order is not an independently observed Retail timing certificate.
- [x] Native441675 Fount of Strength preserves its authored maximum Energy/Rage and increases maximum health by its own field only while the learner carries their own22842 Frenzied Regeneration. Foreign recipients/casters and absent talent do not activate the conditional field; removal/expiry restores maximum health.
- [x] Native231040 Lingering Healing consumes its authored self-Regrowth duration extension without extending another recipient/caster. Its independently masked Rejuvenation extension is preserved.
- [x] Native effect values apply authored RealPointsPerLevel only outside expected-stat/coefficient scaling, with native level bounds.
- [ ] Owned bag item-target locks require native lock eligibility and a persisted per-GUID unlock; do not consume the item or fabricate loot. Retail threshold parity remains uncertified where pinned reference behavior conflicts with caster-aware native values.
- [x] Native target119 offers resurrection to eligible party/raid corpse owners at their body positions, using authored restoration percentages.
- [x] Native targets72/86/91 use random directions and authored radii around the caster or prior destination.
- [x] Unspawned class/spec/talent summon templates/models/addons are retained from authentic TDB rows without creating world spawns.
- [x] Native spellcaster portals restrict party access and apply authored class/race conditions before casting their destination chain.
- [x] Finite harmful auras apply the receiver's native232/234 duration rule to the spell/category and actually applied effect mechanics: min(sum232, strongest negative234), with native integer AddPct. Positive/permanent auras and NoTargetDurationMod remain unchanged; positive-only232 fields and selectors outside the pinned mechanic mask remain named partials without invented extension or selector interpretations.
- [x] Native aura187 adds its victim-owned percentage to incoming melee auto-attack and non-ranged weapon-attack spell crit chance, clamped at zero. Removal/expiry restores ordinary chance; actual ranged weapon and magic spells remain unchanged. Native spell damage class and GetAttackType inputs determine the exception, not physical school alone.
- [x] Native aura263 restricts player casts at admission and completion, except ranged-slot spells, attack effects, explicit IgnoreCastingDisabled attributes or native parent-spell family flags matching caster-class metadata and the cast's flags. Its effect mask is not substituted for the parent flags; NPC/triggered casts remain exempt. Removal/expiry restores ordinary casting, and rejected casts do not pay costs or launch effects. Aura264 remains separate.
- [x] Native cast starts honor the spell's 64-bit required/excluded form masks, NotShapeshifted and AllowCasterForm attributes, and authored Stance/CanOnlyCastShapeshiftSpells form flags. Masked aura275 bypasses the form check only on its recipient; removal/expiry restores validation. Forbidden and missing-required forms produce distinct wire failures before payment. Unknown form rows retain pinned TC's explicit logged acceptance, not invented flags. Triggered casts preserve their existing form-check exemption.
- [x] Native330 permits matching-family casts and channels while moving; native Attr13 prevents that aura exception without changing intrinsically mobile spells. Gain/removal/expiry updates existing casts and the replicated permission; unrelated families remain restricted. This does not certify unrelated charged-buff or empower behavior.
- [ ] Aura305 applies the strongest positive authored minimum-speed percentage after normalization and slows, for run/flight/swim rates. It does not override rooting/stunning; removal/expiry restores ordinary rates and independent floors do not multiply.
- [ ] Native aura142's physical-school percentage multiplies player gear/base armor and pet base armor, not armor contributed afterward by stats. Independent aura factors multiply; removal/expiry restores armor, ordinary NPCs do not gain base armor, and physical spell/melee damage use the same live pet modifier. Nonphysical base-resistance modeling remains separate.
- [x] Aura466 multiplies player armor after base/gear percentages and stat-derived armor. Independent native factors multiply; removal/expiry recomputes armor, native zero values remain zero, and fractional stat armor is retained until final conversion.
- [ ] Aura467 applies its authored percentage only to gear/flat stat bonuses, preserving create/base stats. Native stat indices select individual fields; removal restores those bonuses, and ordinary total-stat percentages apply after base plus modified bonus are combined.
- [x] Aura429 multiplies direct spell and melee damage of the current owner's controlled pets and guardians, not that owner's personal damage or a stranger's summons. Removed modifiers stop affecting summons; copied source damage does not consume the owner modifier again.
- [ ] Native MasteryAffectsPoints adds raw mastery points times the authored effect coefficient before points modifiers. Self-owned flagged aura amounts follow stat/aura changes without restarting duration, stacks or tick phase; derived mastery percent is not substituted for raw points. Underivable Lycara point/percent conversion remains a gap.
- [x] Apply native aura537 damage percentages only for matching label and aura owner.
- [x] Apply aura457 haste only to its charge category; preserve running-clock progress and publish rate changes/replenishment without another cast.
- [x] Apply aura416 haste only to matching spell families, for both spell/category cooldowns, truncating native fractional milliseconds.
- [x] Preserve native signed class power costs, bounded generation, positive/negative sign constraints and atomic rejection.
- [x] Discounted primary misses consume neither runes nor negative-cost grants; school immunity is determined before payment, including composite schools covered by multiple immunity auras.

## How it works

- [Implementation/provenance and proof limits](../class-completeness.md)
- [Spell-casting system](../wiki/systems/spell-casting.md#level80-class-completeness)

## Implementation inventory

- `spell_cast/mod.rs`: cost, cooldown/charge and hit-plan payment decisions.
- `spell_cast/hit.rs`: whole-spell miss and pre-payment school immunity.
- `auras/handlers.rs`, `auras/mod.rs`: native handler dispatch, owner/label damage and school-mask union.
- `spell_cast/class_completeness_probe.rs`: opt-in controlled native-row census.
- `spell_cast/open_lock.rs`, `game_objects/locks.rs`, `persistence_inventory.rs`: native bag-item lock metadata, ordered alternatives and per-GUID unlock state.

## Immunity definition source compatibility

- [x] The creature-immunity importer reads all235 authentic TDB1210 rows, verifies the pinned full-row fingerprint, and transactionally replaces only `creature_immunities` in an existing database copy. Invalid/incomplete input changes no database state; repeated imports are idempotent. Canonical publication belongs to lead.
- [x] Native147 resolves its MiscValue0 through authentic creature-immunity definitions before using mechanic/effect immunity and the parent's authored purge flag. Definition IDs are not mistaken for masks. Missing IDs and unmodeled nonzero school/dispel/aura/other definition fields remain named partials.
- [x] Explicit compatibility requirement: the immunity loader warns and returns empty definitions when the table is absent, without crashing spell loading. Missing definition IDs remain named unsupported inputs; no immunity masks are invented from the aura147 ID itself.
- [x] Effect and mechanic immunity remove only the immune effects from a target. The spell still lands its other effects without an immune miss; the immune miss is logged only when no planned effect remains on that target (TC `Spell::AddUnitTarget`, `UpdateSpellCastDataTargets`).
- [ ] Knockback/pull effects98/124/144/145 are not executed, so the definition effect-list filter has no observable runtime boundary yet; it is not certified as knockback immunity.

## Fear control state

- [x] Native aura7 makes the unit fleeing: its cast stops, auto-attacks stop, a creature shows no target and stops chasing, and a fleeing victim cannot dodge, parry or block. Removal restores swings and the creature's target.
- [x] A fleeing caster fails with "Can't do that while fleeing" unless the spell purges every active fear effect through its own mechanic, school or definition immunity (Attr1 ImmunityPurgesEffect). Spells with Attr5 AllowWhileFleeing pass only when each fear spell's mechanics intersect the spell's allowed mechanics, including TC's Barkskin/Dispersion/Lichborne cases.
- [ ] Flee movement (TC FleeingMovementGenerator) is pending a direction decision: pinned TC adds the owner's orientation to the away-from-caster angle. Aura7 therefore remains an unhandled census type. Player client-control loss is not modelled.

## Armor percentage

- [x] Native aura101 with the physical bit multiplies final armor as TC's armor TOTAL_PCT: after base/gear and stat-derived armor, alongside aura466, with independent auras multiplying. Creatures (including level-scaled ones) multiply their armor before the attacker-level armor multiplier. Removal restores the remaining factors. Nonphysical resistance bits are logged unmodelled.

## Class trans-door portals

- [x] All 29 native class TRANS_DOOR (effect50) spells spawn their authored TDB type22 spellcaster objects; an absent template fails cast admission with its reason instead of silently succeeding.

## Dual wield and parry knowledge

- [x] Knowing a native SPELL_EFFECT_DUAL_WIELD (40) spell permits one-hand and off-hand weapons in the off hand; without it they fail with CantDualWield, and auto-equip offers only the main hand. One-hand polearms never fit the off hand.
- [ ] When a player stops knowing Dual Wield (spec/talent change), TC moves the off-hand weapon to the bags or mails it (OffhandCheckAtSpellUnlearn defaults on). Not yet modelled.
- [x] Knowing any of the six native SPELL_EFFECT_PARRY (22) spells enables parry; without one, parry chance never applies.

## Tests asserting this spec

- `scripts/tests/test_creature_immunities_import.py`: native process import, idempotence/fingerprint, unrelated-data preservation and invalid-source no-write boundary.
- `crates/server/src/spell_info/immunity_tests.rs`: authentic definition1733 root purge/admission, explicitly requested absent-table compatibility, and native1887/1733 partial versus whole-target immunity with Tail Swipe and Thunderstorm.

- `crates/server/src/spell_cast/class_completeness_tests.rs`: native owner/label, charge/cooldown, level80 resources and immune/partial-school boundaries.
- `crates/server/src/spell_cast/class_completeness_probe.rs`: executed membership receipts (manifest supplied explicitly).
- `crates/server/src/spell_cast/bonus_armor_tests.rs`: native466 factors, actual physical damage, removal/expiry, zero class fields and final rounding.
- `crates/server/src/spell_cast/cast_while_moving_tests.rs`: native330 masks, real cast/channel damage, live cast permission gain/removal/expiry and metadata veto controls.
- `crates/server/src/auras/fear_tests.rs`: native fear swings/target/removal, Slam/Berserker Rage/Will of the Forsaken/Barkskin admission against Intimidating Shout, Sleep Walk and Turn Evil, and fleeing dodge loss.
- `crates/server/src/spell_cast/resistance_pct_tests.rs`: native Armor Specialization/Reinforced Plates player armor with aura466, removal, and level-scaled creature mitigation.
- `crates/server/src/spell_cast/portal_tests.rs`: all29 class trans-door spawns and absent-template admission failure.
- `crates/server/src/inventory_tests.rs`: native Dual Wield off-hand admission, rejection and auto-equip.
- `crates/server/src/auras/defensives_tests.rs`: six native Parry spells versus none.
- `crates/server/src/spell_cast/melee_crit_taken_tests.rs`: native187 real damage, victim ownership, additive fields, expiry/removal, positive and ranged/magic controls.
- `crates/server/src/spell_cast/mechanic_duration_tests.rs`: native232/234 stacking, mixed signs, actual clocks/expiry, integer arithmetic and named source-gap exclusions.
- Existing targeted resource/regen/generator tests remain bounded proofs, not every talent combination.

## Known gaps (current cycle)

- [ ] Full legal talent purchases, race/quest/achievement/ancestor/point-budget eligibility are not certified by a granted-spell build-union probe.
- [ ] Underivable Dummy actions, resource proc/reservoir algorithms, empower input, moving fields, soft-cap curves, guardian ownership/stat inheritance and missing imported rows remain explicit table gaps.
- [ ] Whole-spec rotation, PvP and all optional talent interactions are not established by handler coverage or these fixtures.

## Continuation acceptance

- [ ] Repair every broken membership by shared root cause, then partial mechanics in descending membership count; preserve authentic unavailable-data gaps.
- [ ] Complete legal talents and all class/spec mechanics, not just handler dispatch.
- [x] Prove representative repaired mechanics live, at least one per class, on a private fb_* realm in both skins; retain requested PNGs. Current private proofs are representative only; full spell/talent acceptance remains open.
- [ ] Commit/push batches, regenerate census counts, merge origin/master and run targeted tests/check/fmt before final report.

Canonical world.db writes, UDP5000, invented coefficients, the full spell_cast suite and unfiltered server tests are prohibited. [Continuation proof](../class-completeness2.md).
