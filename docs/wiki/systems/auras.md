# Auras

## Prayer of Mending baseline — 2026-10-18

`spell_cast/scripts/prayer_of_mending.rs` handles33076 and155793's native effect0 DUMMY and41635's on-damage proc. Root native jump count4 creates5 initial stacks; existing owned wards add the authored4 through normal refresh, capped by native max10. A proc casts real33110 from the original Priest, launches real155793 from the current holder with remaining stacks/original caster, then removes that holder's ward. The actual40yd/s missile keeps its per-cast SpellInfo clone, so delayed arrival can apply41635 with the original Priest and remaining count. Healing coefficient0.732/variance0.05 and RNG are unchanged. Ward tooltip-point/custom-visual projection and talent custom arguments are outside this baseline.

`SpellInfo` stores two private, non-serialized per-cast arguments: `cast_aura_stacks` and `cast_original_caster`. Aura construction consumes the stack argument before stacking/log/view publication; no post-hoc mutation of a published one-stack ward. Normal applications still request one, and Agony retains its special initial1/3 floor and refresh behavior. The triggered helper carries these arguments through existing planning and missile snapshots; old effect0-amount callers use the same helper with the two new arguments absent.

Source: pinned TC a352b1fa `spell_priest.cpp3636–3806`, `Spell.cpp3247/9516–9581`, `Unit.cpp3403–3458`; unchanged native70448 rows. Random selection follows injury/player priority buckets, retaining full-health targets when no higher-priority candidate exists and excluding the holder through the native selector. **PRAYER_OF_MENDING_NPC_TREAT_AS_RAID_UNIT_STATIC_FLAG_ABSENT** is a named producer gap: NPC static-flag4 raid-unit equivalence is not represented. Tests qualify player recipients, not that NPC classification; no flags/importers are invented. [Contract](../../specs/class-completeness.md#prayer-of-mending-baseline-active70448).

Ignored tests-only cbf816aee RED passes the native800-damage/no-heal control and fails ordinary33076→41635: the cast succeeds with no failure packet but creates no ward (1PASS/1FAIL19.15s). The delayed-jump/original-owner and no-override Agony controls were added before implementation. Atb94cd697a (runtime3ce692202 plus naming-only5d9895d5e), `prayer_of_mending_native` passes8/8 in37.08s,0ignored/0warnings: initial/logged5 stacks, real heal/delayed4-stack jump, healthy decoy, natural expiry, five-hit exhaustion/full-health eligible ally, foreign ward/original ownership, no-recipient consumption, no-override Agony floor/refresh and unprotected damage. Initial exhaustion setup attempted Fireball9053 before its authored4000ms recovery (7PASS/1setupFAIL); the fixture now advances actual spell/category/GCD readiness without resetting clocks or changing any assertion, row, amount or cooldown. Independent lead integration gate remains pending. Receipts: `data/class-completeness6-2026-10-12/prayer-of-mending-{source,proof-ledger}.json`. No generic aura4/original-caster, talent, AI-policy, legal/full-Holy, follower execution, fresh-UDP or rendered certificate.

## Shared death-prevention commit — 2026-10-18

`apply_attack_damage` commits Ardent Defender31850 and Guardian Spirit47788 with exclusive World access for melee, direct and periodic hits. Guardian stores native non-aura effect1's healing percentage on its aura instance (including refresh), checks its actual316 school/overkill limit, removes the parent, then casts real48153 on the rescued unit. Removing the parent first prevents its separate +60% healing-taken effect inflating the native40%-maximum-health heal. Ardent retains its existing20% health-floor behavior, restoring health before removal; committed absorption is only damage beyond that floor, or the whole hit when already below it. This is named-ability support, not a generic316 certificate or new Ardent heal-child implementation.

`try_swing` itself queues the health/rescue commit even if its caller discards the returned outcome. Its returned `SwingCommitReceipt` holds a one-shot committed result; optional later publication reads that actual result without applying a second hit. This preserves the same-tick contract exercised by the unchanged direct combat helper. The initial implementation incorrectly tied health to `record_swing`; independent gs582 found nine existing combat regressions because callers discarding the outcome never committed damage. No test expectation changes are part of this correction.

Melee no longer mutates health under the target query. An exclusive outer system invokes the cached existing attacker system once per attacker; each attacker's health/removal/healing commands finish before the next attacker reads mitigation. Merely deferring all health commits together was insufficient: the next same-frame attacker otherwise retained consumed Ardent's30% reduction. Death, overkill and damage/absorption publication follow actual committed results.

Source: pinned TC a352b1fa `scripts/spell_priest.cpp2136–2176`, `scripts/spell_paladin.cpp174–210`, `Unit.cpp1003–1031`; unchanged native70448 rows. Genuine Guardian RED fa56ab948 gives0 versus600HP after native209140 wounds1000→200 and native314616 becomes lethal. Initial implementation4b75b37e6 passes2/2. Native Ardent log RED cc29f4f14 gives(0damage,700absorbed) versus required(100,600),7PASS/1FAIL; native same-frame RED6a700d245 gives second swing7 versus10 after consumption,9PASS/1FAIL. The first Ardent fixture incorrectly assumed209140 respects damage-taken modifiers; its native attr4=0x100 bypass is retained, and two314616 casts qualify the actual300→200 floor case instead. Native tests use real62627 NPC Hit Buffs for deterministic hit/expertise admission, not RNG/flag/stat rewrites; untuned NPC damage remains the existing fixture's flat10. These API-boundary fixtures are not legal loadout, follower execution, fresh-UDP or rendered proof. At4caa35ee6, combined `guardian_spirit_native` passes10/10 in39.92s,0ignored/0warnings, including direct/periodic/full-vs-partial absorption, source-order48153, expiry, one-shot, exact2×max refusal and two real same-frame melee attackers for both abilities. Independent lead integration gate remains pending. Receipts: `data/class-completeness6-2026-10-12/guardian-spirit-proof-ledger.json`. [Contract](../../specs/auras.md).

## Application-anchored periodic deadlines — 2026-10-17

`AuraEffectState` now stores its private periodic phase origin and next tick ordinal, not an accumulating cached deadline. `next_tick_at()` derives origin plus ordinal×native integer milliseconds/1000; the integer-millisecond product is formed before seconds conversion, avoiding both repeated fractional addition and fractional-product endpoint rounding. Normal effects start at ordinal1; native ExtraInitialPeriod starts at0. Refresh preserves the existing effect's phase/ordinal; owning dynamic-object alignment updates that origin/ordinal using the existing alignment rule. Rolling-damage budgets and phase controls read the derived deadline; no native clock, period, duration, coefficients, expected counts, wire schema or callback order changes. Existing expiry1e-6 guard remains unchanged; no due-now epsilon was added.

Root cause: native Fel repeat trace at63.5s has next_tick_at63.50000000000002842 after repeated0.2 additions. The expiry guard permits it, but exact due-now rejects it and natural expiry removes the aura before later f32 channel completion. This is a generic scheduler representation bug, not missing healing logic. Lead approved a separate generic prefix fix ahead of the Vengeance hook; the up-front Fel source disagreement remains outside this change.

Independent generic RED at7cf3a9906 uses unmodified native Causality Tear436117 effect1127247 (periodic damage3/200ms/3000ms/2damage), not a Fel script. Four ordinary lifetimes at later unmodified application uptime produce[14,14,14,14] instead of[15,15,15,15] (5.36s); actual damage/log/health and expiry are checked without HP/stat/native-row/clock resets. Named RED ignore is removed with the fix. At development revisione5ed121f6, generic four-lifetime regression plus refresh/floor/ExtraInitial/area-pulse/rolling-budget/support-phase controls pass8/8 in4.55s,0ignored/0warnings; all four lifetimes now deliver15 ticks with unchanged native values. The generic fix is published as its own prefix before the Vengeance hook. Requested union blast gives1548PASS/9FAIL/2IGN1136.50s; the nine fixture/environment failures all pass after landed follower enum/master integration and explicit prepared-world/ground env, with no feature changes. Combined19-case native/compatibility/generic batch passes60.03s;13-case repaired-failure/geometry/Singed batch passes41.98s. Plain master3accdf1a1 also passes the requested five environment comparisons15.80s. Server-bin check23.65s/fmt2.33s pass atc80db3cac without warnings; manual changed-Rust audit and diff whitespace check pass. Independent lead gate pending. [Contract](../../specs/auras.md); receipts: engine `data/diagnostics/class-coverage-b-2026-10-17/periodic-deadline-*` plus `vengeance-repeat-deadline-diagnosis.json`. Native436117's encounter/selector semantics are not certified by this generic aura-boundary regression.

## Fel Devastation scheduled healing — 2026-10-17

`fel_devastation.rs` registers only212084's existing effect0 periodic hook. It checks actual rank320639 on the aura caster and casts the real212106 self-heal before the unchanged default A23→212105 damage trigger. No on-apply hook, new state/trigger API, target override or damage-copy/leech calculation is introduced. The native212106 AP5.625/variance0.05/crit/ordinary heal pipeline remains authoritative; zero/multiple/immune recipients do not gate or multiply periodic self-healing.

Source: authenticated70448/read-only runtime19 selected effects plus5 supplemental controls; TC a352b1fa spell_dh.cpp1291–1310 periodic handler, SimC76f8118 sc_demon_hunter.cpp4204–4212/11274–11275 periodic order/rank. **FEL_DEVASTATION_UPFRONT_HEAL_SOURCE_CONFLICT** is deferred: SimC's execute4167–4177 additionally heals; TC's cited handler has no on-apply hook. This implementation/tests do not certify an up-front heal absent. Full TriggeringAura/proc/limited-trigger-flag parity and haste/fractional-final-tick behavior remain outside this baseline.

Before any product hook, b48121a3f existing native child controls3/3 PASS11.81s qualify damage target104 ConeToDest/current facing,75deg/13yd/Max0, ordinary recipient exclusions and self-heal Target1 (Width6 is not area6). Genuine c8a421d56 scheduled-heal RED6FAIL/2PASS21.12s uses actual50Fury from five native Shears, original2s/200ms clocks and real Fireball injury; foreign-rank/refused-resource controls pass. No HP/stat/native-row/cooldown/RNG overrides. Combined native8 plus10 compatibility and generic lifetime control pass19/19 in60.03s, including unchanged40sec-recovery repeat10+10 heals. Geometry3/3 reruns in the13-case env/Singed batch; all13 pass41.98s. Server-bin check23.65s/fmt2.33s pass atc80db3cac without warnings; manual changed-Rust audit complete, metrics binary unavailable. Independent lead gate pending. [Contract](../../specs/class-completeness.md#vengeance-fel-devastation-scheduled-healing-active70448), [coverage/proof](../../class-completeness.md#vengeance-fel-devastation-scheduled-healing-2026-10-17). Evidence: engine `data/diagnostics/class-coverage-b-2026-10-17/vengeance-*`.

## Agony ramp and Sudden Onset (2026-10-17)

`agony.rs` binds only native980's damage ramp and1260209's initial3-stack floor. Native980 AuraOptions9094 caps at8;196102 Writhe is outside the current265 membership, not an imported4-start/18-cap rule. Aura construction reads the caster's actual applied Sudden Onset e1 amount; refresh keeps the existing damage stack or raises it to that minimum, preserving tick phase. The named damage hook reads the live owned aura's base amount × stack before the unchanged damage/spellmod pipeline, including precollected deadline-flush events. The periodic damage branch advances that exact aura after a nonimmune hit; full absorption remains a landed hit, lethal removal is respected. No generic tick reordering, coefficient/clock/resource change or A226 no-op registration; Agony Soul Shard RNG stays unimplemented. Source: active70448 effects374/198244/1269560/1269561, TraitDefinition140864; SimC76f8118 `sc_warlock_actions.cpp1405–1491` damage-before-increment/initial floor and native980 refresh description. [Contract](../../specs/class-completeness.md#affliction-agony-baseline-active70448), [audit/proof](../../class-completeness.md#affliction-agony-baseline-2026-10-17). Nativeeff2f2115 RED8/9 in7.63s: flat15 versus30, recast2 versus1, initial1 versus3/minfloor2 versus3; unchanged Corruption control passes. Native9/9 GREEN8.95s at5b0ee370f,0ignored; deadline-flush is a native API-boundary proof, not live-lag certification. Subsequent pet-battle merge/refresh-comment change leaves runtime logic unchanged. At557fc91df retained controls10/10 pass8.87s, scoped server-bin check20.53s and fmt pass warning-free; manual changed-Rust readability inspected (metrics binary unavailable). Scoped proof19/19,0ignored with native9 retained on unchanged runtime logic. Final master fetch/merge is already included; independent lead gate pending.

## Ebon Might baseline (2026-10-17)

`ebon_might.rs` selects native395152 allied-area candidates, filters known damage-player roles before authoredMaxTargets30, and keeps the native caster exclusion/500yd radius/map/relation rules. Ordinary cast completion grants native395296 self and computes owned395152 support: caster Intellect excluding effective foreign Ebon grants × native8% × min(1,native2/N). `auras_changed` adds each supported amount to the recipient's native primary-flat field; no generic540 selector mapping or new stat/protocol representation. Existing bonus-percent/total-percent ordering is used when excluding borrowed Intellect. The self buff's authored1000ms tick updates only owned amounts/counts; expiry/stacks remain untouched. Self damage uses unchanged native masked spellmods; the existing Eruption/Sands of Time flat extension path is not replaced. Unknown specs, hot spec swaps and optional/full-ability boundaries remain separate. Source: authenticated70448 rows1035393/1035394/1248097 and395296 fields; SimC76f8118 `sc_evoker.cpp4731–5073/9000/10725`, `buff.cpp3553–3563`; TC a352b1fa540 is NULL, not an algorithm. [Contract](../../specs/class-completeness.md#ebon-might-baseline-active70448), [proof](../../class-completeness.md#ebon-might-baseline-2026-10-17). Native16114cc3 RED6/6: missing self chain, absent primary support and tanks wrongly selected. Corrected224e0300 native6/6 GREEN14.08s; the first target-plan implementation incorrectly imposed explicit NoTarget and was replaced by native selectors with deferred role cap. Additional simultaneous HostClient casts reproduce source11.671872 versus12.72 at73f68bf70 before any first tick: aura modifiers changed in FixedUpdate while the normal Update producer had not refreshed CharacterStats. Ebon now runs that existing cached native producer before its source snapshot; no duplicated stat formula, timer delay or global schedule change. At06b2d6cd7, native Ebon8/8 passes12.93s, including first-tick simultaneous casts, native recipient Fireball and single-share/last-removal controls; prior support4/4, legacy Eruption2/2 and native mastery/timer1/1 pass (15/15 total,0 ignored). Scoped server-bin check28.73s and fmt pass warning-free; manual changed-Rust readability inspected. Ebon lead gate493 subsequently passes1061 tests/fmt; baseline slicebaa9b0124 landed.

[Health aura-state qualification](../investigations/health-aura-state-admission.md) records the separate stored-state lifecycle contract and unresolved integration seams; raw restriction loading and aura-spell consumers do not implement health states.

## Augmentation support stats (2026-10-17)

`augmentation.rs` binds only native70448 Prescience410089 effect0 (3 critical-strike percentage points) and Shifting Sands413984 effect0 (caster mastery as recipient Versatility). Prescience409311's explicit-friendly effect1 casts its real child; automatic prioritization/effect0 remain open. Sands snapshots actual caster `CharacterStats.mastery_pct`, then its authored effect1/226 ticks update only that instance's amount, preserving expiry/stacks. Ordinary aura recomputation owns stat removal and normal damage/heal/defense consumers read the result. No generic540 registration or invented selector mapping: TC a352b1fa `SpellAuraEffects.cpp612` is HandleNULL; spell-specific semantics come from pinned SimC76f8118cf3f7f41dad20708e8328e1f4ab2deebc `sc_evoker.cpp3383/7611/8538/8968`. Installed70448 rows1074054/1080811,1075415 and1082678/1082696 supply amounts/clock. E202/other dummies, hero/procs, Ebon and empowered-spell/mastery grant/target selection remain named gaps. [Contract](../../specs/class-completeness.md#bounded-augmentation-support-stats-active70448), [audit/proof boundary](../../class-completeness.md#augmentation-read-only-selection-and-bounded-support-stats-2026-10-17). Native RED4 at e9f5cded: Prescience gives no child; Sands recipient Versatility remains0 despite caster mastery2.176/2.992. Merged5ed1b656 native4/4 GREEN5.52s; the first functional test's unrelated observer-connection failure was corrected test-only, with exact zero-variance native damage and untouched healing variance. At docs-only0114bbf7, retained native mastery-refresh/Sands timer control passes1/1 in5.53s, scoped server-bin check passes91s and fmt check passes warning-free; production Rust unchanged from5ed1b656. Lead gate482 subsequently passes1053 tests/fmt; first support slice landed on server masterc6b92cb31.

## Mechanic-specific damage (2026-10-17)

TC a352b1fa `Unit.cpp:6980–6984` adds the sum of aura276 amounts for the damage effect's nonzero mechanic, otherwise the spell mechanic. `spell_cast::effects::mechanic_damage_multiplier` reads actual caster aura amounts × stacks and joins the existing percent-done pipeline; resolved-source and ignored-caster-modifier paths retain their exclusions. Genuine native Leymor RED: fourteen Sprouts produced one ArcanePower374736 aura with14 stacks, but ordinary45s Stomp374731 dealt8631, equal to its unbuffed baseline instead of32797.8 (3.8×). Both native matching Stomp and nonmatching Brand374582 cases pass in the consolidated34-test Azure run at99cca08a (507.23s, no failures or ignored cases); fmt passes. Corrected scoped check at docs-only96da2b88 passes in1m39s with zero compilation warnings; the prior invalid helper --bin/check combination was rejected before compilation. Independent source and final artifact verification pass; twelve source hashes and exact48e60d67 DB remain stable before/after both gates. No native effect-mechanic-precedence proof beyond spell-mechanic22/no-match0. The diagnostic handler registry still lacks276 classification despite its actual damage consumer; no all-runtime-warning-free claim. No global/school bonus, manual stacks, caster rewrite, stat or spell-row override. [Contract](../../specs/follower-dungeons.md).

## Optional cost spell modifiers (2026-10-16)

TC a352b1fa SpellInfo.cpp4220–4226 explicitly excludes flat spellmods from optional power costs. `spell_mods::modifier_totals` retains the existing live/charged snapshot matching; ordinary `modify` uses `(base+flat)×mult`, while optional `modify_percent` uses only `base×mult`. Native Practiced Strikes429647 effect6/op34 previously subtracted100 from both Protection Execute163201's required and optional Rage: a400 input left200 instead of100. No spell-specific production exception or resource-cap change. Native Sudden Death52437 effect5/op34=-100% and Arms order0 are independent controls. Native OptionalCostPct remains unconsumed. Five native regression/control tests passed within the38-test resource/modifier run (14.74s); after master merge006efb17, all five pass again (14.76s), workspace/all-target check passes64.411s. Independent verification preserves the charged/live matching boundary; no whole-ability/legal-build/fresh-UDP claim. [Contract](../../specs/class-completeness.md).

## Additional optional resource cost (2026-10-16)

TC a352b1fa SpellInfo.cpp4140–4144 adds aura63 only in the optional branch, filtered by power type and `IsAffectingSpell`. `resources::additional_optional_cost` reads the actual caster's applied effects and shared native family/mask matcher; `power_costs` adds that total before the existing optional modifier/availability clamp. Native193531/394321 already raise ComboPoint maximum through418, but their63 effects were missing: ordinary Envenom at native cap6 left1 point, including after unlearning one of two Stratagems. No resource caps, native rows, RNG or simulation clocks change. Native availability and base Energy cost remain separate from optional spend. Native OptionalCostPct fields remain separate from the percentage spell modifiers below. [Contract](../../specs/class-completeness.md).

## Autoattack-only critical chance (2026-10-16)

Native aura334 folds signed applied amounts/stacks into `AuraStatMods.autoattack_crit_pct`. The swing hit table adds only the attacker's total after ordinary crit and victim187, then clamps nonnegative; spell crit does not read it. Source: TC a352b1fa Unit.cpp2387. Avenging Wrath31884 has separate aura290effect2=20 and334effect5=20; TC SpellAuraEffects.cpp4449–4465 updates weapon/spell stats for290 already, so334 is not another global stat modifier. Native340094 has334=100 without290 and native456718 has334=-999, isolating both endpoints through ordinary casts and attacks. Genuine behavioral RED: positive native buff produced only normal swings and negative native aura retained critical swings. Full Wings/talent bindings, legal rating budgets and fresh bot UDP acceptance remain open. [Contract](../../specs/class-completeness.md).

## Independent melee and ranged attack power

Native166/167 fold independent percentages into separate `StatModifiers` fields before character AP computation. `ChrClasses.RangedAttackPowerPerAgility` loads with class properties; ranged AP is `(level + max(live Agility,0)) × coefficient`, not melee AP. `CharacterStats.ranged_attack_power` is server-only; existing replicated `DerivedStats.attack_power` still means melee AP. Non-melee ranged-weapon spell AP coefficients and hunter pet owner inheritance consume RAP; AP-to-spell-power remains melee-owned. Source: TC a352b1fa StatSystem.cpp354–414, Unit.cpp6880–6890/9973–9998. [Contract](../../specs/class-completeness.md). Source-backed weapon/flat-AP additions and client RAP display are not certified by this percentage slice.

Server aura runtime in `crates/server/src/auras/`. `AuraPlugin` keeps active units' auras in `ActiveAuras`; cold `GridDormant` creatures defer bulk initialization and retain pending addon effects until their real grid wake. The addon applies in that same fixed step, without resetting an existing aura on subsequent wakeups (see [[creature-ai]]). It handles stacking and refresh, periodic ticks, procs, spell modifiers, stat modifiers, expiry and removal. The replicated view is `UnitAuras`, which changes only when the aura set changes. [[spell-casting]] resolves aura effects through `auras::apply_aura` and reads spell modifiers when it computes cost, cast time, cooldown, GCD, range and amounts. The data comes from [[spell-info]].

## Application

- **Aura effects:** SpellEffects 6 (APPLY_AURA), 35 (party area aura) and 65 (raid area aura). There is no group backend, so area auras reach only their resolved targets (always the caster in scope).
- **One aura per target:** `resolve_effects` collects the aura effects that hit the same unit in one resolution into an effect mask, then calls `apply_aura` once for that unit. Shadow Word: Pain 589 effects 1 (aura 3) and 3 (aura 269) form one instance on the target.
- **Amounts:** each effect's per-stack amount is set at application: `BasePoints` with Points and PointsIndexN mods, plus SP/AP coefficients (BonusCoefficient mods) for damage, heal and absorb types. The amount does not change after that. Ticks and modifiers use amount × stacks.
- **Duration:** `spell_duration` with Duration mods. A duration of 0 or -1 makes the aura permanent, and passives are always permanent.
- **Stacks and charges:** max stacks = `CumulativeAura` with MaxAuraStacks mods; 0 means the aura does not stack. Charges = `ProcCharges` with ProcCharges mods.
- **Dead targets:** get no auras unless the spell has `SPELL_ATTR3_ALLOW_AURA_WHILE_DEAD`.
- **Unhandled aura types:** still applied, tracked and replicated. A warning is logged once per (spell, aura type).

### Stacking rule

This is a simplified TrinityCore `Aura::CanStackWith`:

| Case | Result |
|---|---|
| Same spell, same caster, same source | Refresh: new duration, +1 stack up to the maximum, amounts and absorbs reset, tick phase kept. `SPELL_ATTR1_AURA_UNIQUE` auras are not refreshed. |
| Native `CumulativeAura >1`, same source, non-channel/non-DOT/non-multislot, other caster | Reapply the same stack slot and add one native stack up to capacity, retaining the existing aura caster. TC `SpellInfo.cpp:1807–1815` / `Unit.cpp:3403–3465`; no manual stack assignment or caster rewrite. |
| Otherwise same spell, other caster | The auras coexist if the aura is harmful, has a periodic effect or has `SPELL_ATTR3_DOT_STACKING_RULE`. Otherwise the new aura replaces the old one. |

The shared-slot boundary was exposed by [native Sprout Arcane Power](../investigations/follower-data.md#sprout-native-spawn-aura--2026-10-17): fourteen actual source casters reached the correct summoner but beneficial replacement retained only one count. Passive/native multislot exceptions, channels and the native DOT rule stay out of shared-slot merging. This rule does not implement aura276's damage modifier.

### Polymorph recovery swing oracle — 2026-10-18

Post-break/expiry attack resumption is proved by a new ordinary melee combat-log event from the Kobold to the Mage (no spell, Damage or Miss), not an HP decrease within100 simulation frames. `combat.rs::swing_log_event` publishes avoided attempts with zero damage, so the previous HP oracle could reject correct resumed attacks. Native Evasive Maneuvers50240, normally cast after full Polymorph expiry, deterministically produces three real Miss(Miss) events with unchanged Mage health while the old oracle fails. The control retains the actual60s expiry, display restoration and ordinary attack path; no RNG/stat/row edits or264 consumer relaxation. A positive swing event also fails if the creature genuinely remains unable to attack.

### Classification

- **Harmful** (`AuraApplication::_InitFlags`, SpellAuras.cpp:113-147): from itself or a caster not friendly to the target, an aura with a negative applied effect; from a friendly caster, an aura with no positive applied effect. Effect positivity is `spell_info::positivity`, a port of `SpellInfo::_InitializeSpellPositivity` / `_isPositiveEffectImpl` (SpellInfo.cpp:4612-5105): `SPELL_ATTR0_AURA_IS_DEBUFF` and `SPELL_ATTR4_AURA_IS_BUFF` first, then effect and aura types, base-point signs and enemy implicit targets. Polymorph 118 (MOD_CONFUSE, TRANSFORM at an enemy target) is a debuff on the neutral Blackrock Spy (`FactionTemplate` 7), which the old hostility rule made a buff. `spell_custom_attr` marks are not modelled.
- **Hidden:** passive (`SPELL_ATTR0` 0x40), or `SPELL_ATTR0_DO_NOT_DISPLAY_SPELLBOOK_AURA_ICON_COMBAT_LOG` (0x80).
- **From player:** the caster has `Player` at application.
- **Dispel type:** `spell_categories.DispelType`.

### Native signed haste positivity — 2026-10-18

Native70448 Curse of Weakness702 has aura342=−20 on effect0 and aura290=0 on effect1. The positivity classifier lacked342, so both effects were positive and ordinary friendly Remove Curse475 left the actual curse in place. Add342 to the existing signed-stat rule (negative base/per-level points), preserving passive/attribute precedence and the generic dispel/picker predicates. No spell-ID exception or stored aura flag override.

Pinned TC a352b1fa `SpellInfo.cpp4833–4859` covers counterpart192 but also omits342. Applying the same signed rule to342 is a native70448 counterpart extension, not an exact TC342 positivity port: `SpellAuraEffects.cpp264/414` maps both to `HandleModMeleeRangedSpeedPct`. Native Curse of the Satyr442804 is negative, Slice and Dice5171 stays positive and passive Crusading Strikes404542 stays positive.

Behavioral RED0/3 includes a normal creature702 application followed by a real Mage player475 cast: SpellGo succeeds but the curse remains with `harmful=false`. Generic fix makes all three pass, including the previously ignored [follower classification regression](../investigations/follower-data.md#native-dispel-and-caster-control-conditions--2026-10-18). Targeted49/49 GREEN includes retained utility, polymorph/positive-buff, cancel and ordinary Purify controls. This proves the player dispel boundary and metadata polarity, not a complete positivity port, follower702 targeting/full kit, legal loadout or fresh-UDP/rendered behavior. [Contract](../../specs/auras.md).

## Typed invisibility and spell-area eligibility (2026-10-11)

Aura18 and19 now reconcile an `Invisibility` component: maps from authored MiscValue0 types to summed integer invisibility/detection values, plus aura335's reciprocal-detection exception for unit targets. Reciprocal flags do not apply to game objects (TC Object.cpp:1003); their own hidden types still require matching detection. Both type presence and value matter; zero does not remove a type. Interest checks this per observer on both observer and target updates, including stationary detectors and late arrivals. Spell relation checks, melee swings and creature sight use typed detection rather than treating every invisible unit as universally hidden. The old `Invisible` boolean marker is removed; the boss vanish regression asserts player detection instead. Applying reciprocity to every entity hid all arena gates during32727/type4 preparation, despite correct positions and map copy; the real-channel Cage preparation fixture reproduces that boundary.

`spell_area.rs` consumes authored `tdb_spell_area` rules for area/zone, race/sex, signed aura prerequisites and quest-status masks. Current quest representation supports none0, complete1, incomplete3 and rewarded6. Autocast/autoremove and ignore-quest-change flags govern managed auras; aura-triggered changes do not count as quest-only changes. Managed autocasts cannot be canceled by the player. Location/status/aura changes reconcile before quest-update dirtiness is drained.

Jaga's89302/type35/value100 is detected by100616/type35/value1000 in Valley of Wisdom area5171 until29401 is rewarded. Real private5496 Modern+Forever native loot, credit, turn-in and post-reward disappearance were captured and inspected (1400XP,20silver,Horde Balloon). Authentic Jaga model125367 and11 reference assets were receipted against local CASC; no aura stripping or objective/item grants. The original root cause was boolean invisibility routed through universal `interest::set_hidden` and absent spell-area input.

[Safe incremental import and source gaps](../../invisibility-import.md) owns rollout instructions and the unproven Vin/Rendan eligibility routes. [Contract](../../specs/invisibility-detection.md). Pinned TC source is archived read-only under engine `data/diagnostics/invis-detection-2026-10-11/`: Object.cpp992, SpellAuraEffects.cpp1650/1676, SpellMgr.cpp732/2309 and Player.cpp26499/26509.

## Passives

- Passive spells with an aura effect in the unit's `KnownSpells` are applied as permanent self-cast auras (`AuraSource::KnownSpell`).
- When a spell leaves `KnownSpells`, its passive aura is removed (`Unlearned`).
- A `KnownSpells` change is queued in `Update` and synced in the next frame's `FixedUpdate`.
- Talent passives: `compute_known_spells` adds `TraitLoadout::active_talent_spells()` to `KnownSpells`, so passive talents apply and unapply through the same sync.

## Removal

| Reason | Trigger |
|---|---|
| Expired | `expires_at` passed (after that frame's ticks) |
| Death | the unit's health reached 0: every non-passive aura without `ALLOW_AURA_WHILE_DEAD` is removed |
| Cancelled | `CancelAura{spell_id}` from the owner: removes the owner's non-harmful, non-passive auras of that spell, unless the spell has `SPELL_ATTR0_NO_AURA_CANCEL` or its source is a spell-area autocast |
| Dispelled | effect 38 DISPEL (below) |
| Interrupted | aura interrupt flags (below) |
| ChannelEnded | a channel completed or was cancelled: that caster's auras of the channel spell are removed from every unit |
| Consumed | the last charge was spent, or the absorb was depleted |
| Unlearned | the passive left `KnownSpells` |
| CasterGone | the caster entity despawned, since ticks need a source |

Every removal logs `CombatLogEvent{AuraRemoved}`.

### Dispel (effect 38)

- Removes up to `BasePoints` auras (at least 1) whose dispel type is `EffectMiscValue_0`. Type 7 matches every dispel type.
- On a hostile target it removes buffs; on a friendly target, debuffs. Passives are never dispelled.
- Each removed aura logs `Dispel`, with source = the dispeller, `spell_id` = the dispelling spell and `extra_spell_id` = the removed aura's spell, plus `AuraRemoved`.
- Example: Purify 527 removes Shadow Word: Pain (DispelType 1).

### Interrupt flags

`SpellInterrupts.AuraInterruptFlags_0/_1` are implemented where a matching system exists. Counts are in-scope aura spells carrying the flag.

| Flag | Bit | Raised by | In-scope spells |
|---|---|---|---:|
| HostileActionReceived | 0 0x1 | taken melee swing, or taken harmful spell, ability or periodic | 3 |
| Damage | 0 0x2 | any damage taken above 0 after absorbs | 7 |
| Action | 0 0x4 | the unit successfully cast a spell | 8 |
| Moving | 0 0x8 | `Moving(true)` | 3 |
| Attacking | 0 0x1000 | the unit swung in melee | 11 |
| NonPeriodicDamage | 0 0x1000000 | non-periodic damage taken | 0 |
| EnteringCombat | 0 0x10000000 | `CombatStatus` false → true | 1 |
| LeavingCombat | 0 0x80000000 | `CombatStatus` true → false | 0 |
| ChangeSpec | 1 0x40 | `ActiveSpec` changed | 131 |

- Event interrupts skip auras of the spell that raised the event, so a spell's own hit does not remove the aura it just applied.
- Not implemented (no system): LeaveWorld 0x80000 (97 spells; despawn removes the auras anyway), EnterWorld 0x400000 (56), DamageCancelsScript 0x8000000 (42, NYI in TrinityCore too; Polymorph 118 carries it and breaks through its proc instead, see [Crowd control](#crowd-control)), Interacting, Mount, ItemUse, Login, Looting, Turning, Anim, Summon, and the flags-2 raid/instance/talent/glyph events. ChangeTalent (flags 2 0x4000, 82 spells) waits for the trait system.

## Periodic effects

- Each periodic effect ticks at `applied + k × period` while the tick time is at or before expiry. A 16 s aura with a 2 s period ticks 8 times.
- Period = `EffectAuraPeriod` with Period mods.
- The aura caster is the tick source. Tick events are not triggered, so they can proc.

| Aura | Tick |
|---|---|
| 3 PERIODIC_DAMAGE | `deal_damage(periodic)` |
| 89 PERIODIC_DAMAGE_PERCENT | amount% of the target's max health as damage |
| 8 PERIODIC_HEAL | `apply_heal(periodic)` |
| 20 OBS_MOD_HEALTH | amount% of max health as a heal |
| 24 PERIODIC_ENERGIZE | `EffectMiscValue_0` power, raw units |
| 23 PERIODIC_TRIGGER_SPELL | casts `EffectTriggerSpell` from the caster at the aura's unit, as a triggered spell (depth 1) |
| 226 PERIODIC_DUMMY | `AuraScripts` `on_periodic` |

Examples: Shadow Word: Pain 589 ticks 12 (0.1246 × SP 100) eight times. Renew 139 heals 61 five times.

### Recuperate full-health completion

World of Osso user decision (2026-10-05): spell 1231411 triggers aura 1231418 (OBS_MOD_HEALTH, 5% max health every 1000 ms for 10 s). A per-spell completion rule in `periodic.rs` removes that aura on the healing tick reaching max health, or before a tick if already full. It also ends a matching channel; queued ticks observe the removal and cannot heal afterward. Other periodic heals remain unchanged.

Casting at full health stays allowed and ends at the first healing tick without a Heal event. `world.db` SpellMisc 785968/785975 and SpellEffect 1223435/1223445 contain no applicable full-health rejection rule; no Recuperate-specific Retail rule is established. TrinityCore `a352b1fa` `SharedDefines.h:1714` defines `SPELL_FAILED_ALREADY_AT_FULL_HEALTH = 2`, but `Spell.cpp:7461-7476` uses it for consumable items with direct HEAL effects, not this triggered aura. Its `SpellAuraEffects.cpp:5879-5881` skips full-health ticks only for permanent auras, so early completion deliberately differs from TC.

Heal log `amount` is raw healing and `overflow` is overheal; effective healing is `amount - overflow` (`spell_cast/effects.rs`). Behavioral tests cast the real data spells, verify immediate aura/channel removal, absence of later heal events, and unchanged Renew overheal ticks.

## Native attack restriction264 — 2026-10-18

`casting_restriction.rs` now reads aura264 at ordinary player admission/completion, separately from263: TC a352b1fa `Spell.cpp:5774–5791` blocks ranged-slot/next-swing/autoattack-enabling spells and attack/weapon-damage effects unless the parent aura's native family matches caster-class metadata and its flags overlap the cast. The existing263 parent-family matcher is shared with an explicit aura-type argument; effect masks are not substituted. Passive264 casts, NPC casts and full-triggered casts bypass this player-only check. `ActiveAuras::cannot_auto_attack` separately stops every unit's ordinary swings while264 is present (`Unit.cpp:2259`), without labelling264 as25/60 Pacify.

Genuine REDbc5a0c05e fails2/2 in6.34s/155823ms: normal Turtle186265 produces205769, but Steady Shot reports later Pacified instead of source264 CantDoThatRightNow; a native Phalanx287353 NPC, which has264 but no pacify/263, still lands a20damage autoswing. Production312234334 adds those consumers; Turtle's existing defensive test now expects the more specific source-priority refusal, retaining its no-swing and post-expiry positive-damage assertions. No native rows, clocks, RNG, actor budgets, wire fields or producer scripts change.

Integrated4457dcfe9 passes24/24,0failed,0ignored in7.06s/146566ms: eight new cases include eight unchanged native predicate rows (5019/75/2764/6603/27580/96/1464/585), actual Cheetah utility/Fireball damage, Steady Shot midcast gain, natural expiry and full-trigger damage, plus NPC swing suppression/removal. Thirteen existing263 family/class/recipient/NPC/trigger controls and three real Turtle controls remain green. Native264 positive offensive-family exceptions have no qualified current input: only three parent rows have nonzero masks; Felstorm/Wrathstorm use pet-family57 absent from player class sets, while Hunter205769 overlaps only itself and non-attack186265. The shared matcher has real263 positive/negative behavior proof; do not call that a new264 positive-case proof. Passive/NPC ordinary spell exemption is source inspection here, not a new264 runtime certificate. Opcode17/78 predicate rejection does not certify their still-unbound execution. Server/all-target check atc2156173b passes107.438s without warnings; fmt and manual changed-Rust readability pass (metrics CLI unavailable). No whole-ability, legal-loadout, rendered/fresh-UDP or full13/40 claim.

[Contract](../../specs/class-completeness.md); receipts `data/class-completeness6-2026-10-12/aura264-{goal,proof-ledger}.json` and `aura264-{readonly-native-candidates,family-qualification}.json`.

## Mechanic immunity, pushback and normalized movement

Aura 77 (`mechanic_immunity.rs`) shares one mechanic matcher between hit-time rejection, direct aura application and removal of existing matching aura instances through normal removal hooks. It compares MiscValue with `SpellCategories.Mechanic` and `SpellEffect.EffectMechanic`; `spell_cast/hit.rs` removes each matching harmful effect from that target before effects execute, respecting the no-immunities bypass; it logs Immune only when no planned effect remains on the target (TC `UpdateSpellCastDataTargets`), so partially immune targets still receive the remaining effects. Aura 147 resolves MiscValue0 through the TDB `creature_immunities` row and consumes its mechanic mask and effect list; missing rows and nonzero school/dispel/aura/AoE/chain fields are logged unsupported. Its effect lists name only knockback/pull effects, which are not executed yet ([spec](../../specs/class-completeness.md#immunity-definition-source-compatibility)). Direct aura applications use the same immunity check; the cast effect loop has no second rejection path. Icebound Fortitude 48792 removes/rejects Hammer of Justice 853 (mechanic 12), not Hamstring 1715 (effect mechanic 11). Unending Resolve 104773 rejects Pummel 6552 (interrupt mechanic 26).

Aura 7 (`auras/fear.rs`, `unit_state.rs`) marks the unit `Feared` with the first fear aura's caster. It stops the cast and auto-attacks, clears a creature's target and chase, and removes dodge/parry/block. Fleeing casters fail unless the spell purges each fear effect by its own immunity, or Attr5 AllowWhileFleeing and its allowed mechanics cover the fear spell's mechanics (TC `CheckCasterAuras`, `SpellCancelsAuraEffect`). `auras/fleeing.rs` runs both players and creatures through collision-clipped Detour paths, quiet28..43yd, native≤30yd route limit,100/200ms retries and800..1500ms pause (TC a352b1fa `FleeingMovementGenerator.cpp:26–27,66–89,118–198`). `MovementControl` epoch ownership prevents player input and preserves a newer owner on removal. `Object.cpp:430–434` distance uses both combat reaches, read from the existing `encounter_creature_model_info` table; `encounter_model_geometry.py` now imports all98,609 TDB1210 source rows. Canonical publication is lead-owned and applied: read-only equality verifies98,609 rows match the slot source table (receipt `model-reach-publication-equality.json`). No guessed reach or unbaked direct route. [Bounded proof](../../class-completeness6.md#native-fear-navmesh-fleeing--batch53), [spec](../../specs/class-completeness.md#fear-control-state).

Aura 149 sums percentages into `AuraStatMods.pushback_reduction_pct`. Positive non-periodic damage events apply reduced pushback to the replicated CastState: normal casts restore up to500ms of elapsed progress, channels retain their existing−25% duration behaviour, each multiplied by `1 − reduction/100`, at most twice. 100% protection consumes no count. The shared CastState pushback method remains unused by the server. Barkskin22812 is100%; catalog spell1258016 supplies the70% test. No spell-specific gameplay IDs are hardcoded in these handlers.

### Normal cast remaining-timer cap — 2026-10-17

Pinned TC `Spell.cpp:7994–8028` clamps remaining timer plus damage pushback to the cast's original duration. For the literal server fields `duration=T`, `elapsed=E`, remaining is `T−E`; `min(T−E+delay,T)` is implemented by keepingT fixed and assigning `E=max(E−delay,0)`. No new state/wire fields are needed, and repeated early hits cannot grow remaining beyondT. Channel logic, reductions, full protection and the two-hit count stay unchanged. This corrects the old unconditional extension of `duration`.

Real native Omor30686 RED at0648c5040 issues an actual damage hit at elapsed0.35000002; the existing deferred aura event arrives at0.40000004 and leaves3.1s remaining on a native3s bar (0PASS/1FAIL/36.85s/183555ms). The fixture retains normal50ms ticks, real damage/threshold, repeated hits and natural completion; no imported clock, raw cast/health/resource state or RNG changes. Late-hit reduction controls now observe absolute completion deadlines after normal progress, not an extended-duration representation. Initial148639e86 corrected cohort passes6/6 in133.19s/428707ms, covering real early first/second/third hits and completion, full/reduced/protected/periodic/channel controls. Current-master integration also retains the Omor full500ms completion-latency control and Frostbolt's exact ten-tick launch delay while expecting a fixed native duration. Integrated6ae25ad4b passes9/9 in99.87s/501336ms, including both strict Omor clock/pushback controls, exact active Shield40–43.2s bound and Frostbolt's ten-tick launch boundary, with no ignored cases. After integrating the priority captured-destination and skyriding native-child fixes, b20f0c4bd passes31/31 in63.00s/123063ms: those same nine clamp/Omor/Frostbolt cases plus all21 skyriding cases and unchanged full Akunda. Server/all-target check passes10.186s without warnings; fmt and manual changed-Rust readability pass. No general cast-lifecycle, legal-loadout or rendered/fresh-UDP/full13/40 acceptance follows.

Aura 319 (`AuraStatMods.attack_time_mult`) is melee attack time only: TC `HandleModMeleeSpeedPct` → `ApplyAttackTimePercentMod` (SpellAuraEffects.cpp:4564, Unit.cpp:11032) multiplies each swing by 100/(100+amount), like aura 252 but without cast time. TC's exclusive same-effect spell-group check is not modelled; in TDB1210 only Berserker Stance 386196 has aura 319 in a group (1502, the three exclusive Warrior stances), and the other stances carry no aura 319, so the check never selects a different value.

Aura 191 is an **absolute speed cap**, not a percentage or immunity to slows. `stat_mods.rs` takes the greatest positive amount, caps run/flight against base 7 and swim against base 4.722222 yards/second, then applies the strongest slow and root/stun. Pillar of Frost 51271 has amount 2: run speed is 2 yd/s, or 1 yd/s under Hamstring. Swim uses the existing movement path with a separately normalized multiplier.

Reference: TrinityCore `a352b1fa`, `Unit::UpdateSpeed` (normalization before slows), `Spell::Delayed`/`DelayedChannel` (summed REDUCE_PUSHBACK), and `AuraEffect::HandleModMechanicImmunity` → `SpellInfo::ApplyAllSpellImmunitiesTo`. Tests: `auras::spell_auras_tests`. These handlers do not implement the other missing aura types on these spells.

## Damage and healing

`spell_cast::effects::deal_damage` applies these steps in order:

1. SCHOOL_IMMUNITY 39: the target takes nothing and `Miss(Immune)` is logged.
2. HealingAndDamage mods (PeriodicHealingAndDamage for ticks).
3. Caster `damage_done` (MOD_DAMAGE_PERCENT_DONE 79).
4. Target `damage_taken` (87) × MOD_SPELL_DAMAGE_FROM_CASTER 271 auras from this attacker.
5. SCHOOL_ABSORB 69: matching absorbs (MiscValue school mask) soak the damage, oldest first. A depleted absorb removes its aura. `absorbed` is logged.

Healing takes the same mods, then MOD_HEALING_DONE_PERCENT 136 and the target's healing taken (118, 283).

Crits:
- Chance = `CharacterStats.crit_pct` (base, rating and MOD_CRIT_PCT auras) + CritChance mods.
- A crit adds 100% of the amount, scaled by CritDamageAndHealing mods.

Melee auto-attack (`combat.rs`) takes the attacker's physical `damage_done` × `autoattack_damage_mult` (344) and the target's physical `damage_taken`. It shares school immunity and absorb consumption with spell damage; query reads and target capacity mutations are separated by `ParamSet`. See [defensives audit](../../defensives-audit.md) for outcomes, Brew's health effect and verification limits.

## Autoattack range

Native153 folds signed yards into `AuraStatMods.autoattack_range_bonus`. `combat::try_swing` adds the attacker's value to existing nominal5yd range for both hands, comparing squared distance inclusively as TC a352b1fa Unit.cpp691–709. Independent effects/stacks sum; ordinary aura reconciliation handles removal, expiry and per-cast refresh. Victim modifiers and spell range do not consume it. Existing combat-reach geometry is not implemented by this change.

Native Cat768 effect8 and Astral Influence197524 effect1 are zero in current lead-published rows; no talent-to-form amount binding is invented. Actual positive326486/+5 and338449/+10, negative225319/−3 and zero/native ownership/lifecycle fixtures prove the consumer, not a whole Cat/Astral Influence certificate. [Batch51 proof](../../class-completeness6.md#native-autoattack-range--batch51).

## Damage shields

`damage_shield.rs` implements native aura15 at the damaging-melee boundary, not through ordinary aura proc flags. `combat::record_swing` queues it after health/death handling; lethal swings run shared death-aura removal first because the normal lifetime sweep is later. Passive/allow-while-dead exceptions remain native. Effects are copied before retaliation, since damage can mutate aura lists.

The shield owner receives damage attribution; the original aura caster supplies SP/AP coefficients and damage bonuses at retaliation. Incoming hit defenses run even though the shield spell itself is beneficial. Damage uses normal school immunity, absorbs and health/death handling without a critical roll or recursive melee reaction. Aura15 damage does not extend a creature's last-damaged leash window (TC `Unit.cpp:1085–1089`).

Source: pinned TC `a352b1fa`, `Unit.cpp:1607–1657`; [encounter requirements](../../specs/boss-scripts.md#wailing-caverns-source-slices). Tests cover real Anacondra8148 melee retaliation/dispel/lethal ordering, source7s/63s buff timing, original-caster percentage attribution, native20570 power coefficient after changing SP100→200 (3+200=203), immunity/absorbs and leash exclusion. Full Wailing fights, missing spell2, loot/lockout and native rendering remain separate gates.

## Procs

Emitters push `AuraEvent{actor, target, actor_mask, target_mask, spell, can_proc, periodic, damage}` into `PendingAuraEvents`. `damage` is the damage the target took, after absorbs, in its own health units: a level-scaled creature's health loss (`level_scaling::health_loss`), not the attacker-frame amount. `process_aura_events` drains the queue in `FixedUpdate`. Masks use TrinityCore `ProcFlags`/`ProcFlags2`.

| Event | Actor | Target |
|---|---|---|
| Melee swing (`auto_attack_system`) | DEAL_MELEE_SWING + MAIN_HAND_WEAPON_SWING | TAKE_MELEE_SWING + TAKE_ANY_DAMAGE |
| Spell lands (one per distinct target) | by `DefenseType`: MELEE_ABILITY / RANGED_ABILITY / HARMFUL or HELPFUL_SPELL (magic) / HARMFUL or HELPFUL_ABILITY (none) | the TAKE counterpart, plus TAKE_ANY_DAMAGE and the damage when the spell damaged it (`Spell::TargetInfo::DoDamageAndTriggers`) |
| Damage outside a resolving spell (scripts) | — | TAKE_ANY_DAMAGE |
| Periodic damage / heal | DEAL_HARMFUL / DEAL_HELPFUL_PERIODIC | TAKE_HARMFUL (+ TAKE_ANY_DAMAGE) / TAKE_HELPFUL_PERIODIC |
| Kill | KILL | DEATH |
| Cast success | flags 2 CAST_SUCCESSFUL | — |

An aura procs when all of these hold (autogenerated TrinityCore proc entries):
- its `ProcTypeMask` intersects the event side's mask;
- it has a trigger-type effect (`handlers::can_proc`: 4, 226, 42, 231, 12, 26, 455, 69, 79, 87, 271, 110, 138);
- for spell events, when the trigger effects' `SpellClassMask` union is non-zero, the event spell has the aura's `SpellClassSet` and an overlapping mask;
- the event is not from a triggered spell (resolution depth > 0), unless the aura has `SPELL_ATTR3_CAN_PROC_FROM_PROCS` or the triggered spell `SPELL_ATTR3_NOT_A_PROC` (Frostbolt's missile 228597 has it), and the event spell is not the aura's own trigger spell (`Aura::GetProcEffectMask`, `IsTriggeredByAura`); `PROC_ATTR_TRIGGERED_CAN_PROC` from `spell_proc` is not imported;
- a breakable crowd-control effect takes part only in damaging events of other spells (`AuraEffect::CheckEffectProc`);
- `ProcCategoryRecovery` has elapsed;
- the roll passes. Chance is RPPM (`BaseProcRate`, TrinityCore `CalcPPMProcChance` with the 10 s attempt cap and bad-luck factor) when present, else `ProcChance`, with ProcChance mods.

When an aura procs:
- each PROC_TRIGGER_SPELL (42) or PROC_TRIGGER_SPELL_WITH_VALUE (231) effect casts `EffectTriggerSpell` from the aura's unit at the other party, at depth 1;
- the aura's `AuraScripts` `on_proc` runs;
- breakable crowd-control effects lose the event's damage; one at 0 removes the aura (below);
- one charge is spent.

Direct spell damage is gathered per target while the spell resolves (`spell_cast::ResolvingHits`, one frame per resolution, triggered spells inside their trigger's) and rides on that target's hit event.

Example: Devastator 236279, a passive with mask 0x4 and 100% chance, casts 236282 (131 damage at AP 100) on every swing.

Not modeled: `SpellTypeMask` (damage/heal/no-damage filter), hit masks (there are no misses or crit-only procs), `PROC_ATTR_REQ_EXP_OR_HONOR`, and proc events for ranged auto-attack, jump, enter combat, dispel and loot.

## Crowd control

- **Breaking on damage** (`handlers::breaks_on_damage`, TrinityCore `AuraEffect::CalculateAmount`/`CheckEffectProc`/`HandleBreakableCCAuraProc`): MOD_CONFUSE 5, MOD_FEAR 7, MOD_STUN 12, MOD_ROOT 26 and TRANSFORM 56 effects of a spell with a `ProcTypeMask` hold 10% of the target's max health at application. Each damaging proc takes its damage off; at 0 the aura ends. MOD_ROOT_2 455 is not included (not `isTriggerAura` in TrinityCore).
- **MOD_CONFUSE 5** (`unit_state.rs`, `SetControlled(UNIT_STATE_CONFUSED)`): the first aura cancels the unit's cast (`CastStop`) and inserts `Confused` at its position. Casting fails unless the spell has `SPELL_ATTR5_ALLOW_WHILE_CONFUSED`; melee swings stop (`UNIT_STATE_CANNOT_AUTOATTACK`, swing timers run on); the creature's `UnitTarget` is empty (`SetTarget(Empty)`); `creature_ai::confused_movement_system` walks it to random points up to 2 yd either way from the anchor with 0.8-1.5 s pauses (`ConfusedMovementGenerator`), overriding chase, wander, waypoints and leash. Threat and the victim are kept: when the last aura goes, it chases and swings at its victim again. Players are not moved (their client drives them).
- **TRANSFORM 56** (`HandleAuraTransform`, `RestoreDisplayId`): the newest transform aura, a harmful one before a newer helpful one, sets `ModelDisplay` to a model of the MiscValue creature template (`zones::template_display_id`, `ObjectMgr::ChooseDisplayId`); `Transformed` keeps the native display, restored with the last transform. MiscValue 0 (TrinityCore's per-spell racial displays) and unknown templates are logged and change nothing.
- **MOD_SHAPESHIFT 36** (`auras/shapeshift.rs`, `HandleAuraModShapeshift`): applying a form removes every other form aura. Display order (`RestoreDisplayId`): a TRANSFORM, else the form's `SpellShapeshiftForm.CreatureDisplayID` (Cat Form 1 → 115603, Bear Form 5 → 115602), else native; `Transformed` records which aura shows. The race/gender form model (`GetModelForForm` via `ChrCustomizationDisplayInfo`) is not modelled: world.db has no ChrCustomization tables. The form also sets the display power ([[spell-casting]] Powers). Not modelled: shapeshift boosts (`HandleShapeshiftBoosts` 3025/1178...), form attack speed (`CombatRoundTime`), `PresetSpellID`, polymorph/movement-impair removal on shifting, MOD_POWER_DISPLAY 179.
- **Polymorph 118** (12.1.0.69933): effects MOD_CONFUSE 5, TRANSFORM 56 → template 16372 Polymorphed Sheep (displays 856/857), MOD_SCALE 61 with 0 points (unhandled, no effect). 60 s (`SpellDuration` 3; PvP 6 s not modelled), 1.7 s cast, 30 yd. `ProcTypeMask` 0x800AAAA8 (every TAKE_* hit) with 100%, `SPELL_ATTR3_CAN_PROC_FROM_PROCS`; `AuraInterruptFlags` DamageCancelsScript | LeaveWorld. `TargetCreatureType` 8385 (beast, humanoid, critter, wild pet) is enforced on explicit targets (`targets::creature_type_allowed`); a Training Dummy (mechanical) is an invalid target. A mage learns it at level 10 (`skill_line_ability` 25214, AcquireMethod 4; `SpellLevel` 10).
- **Regeneration** (`spell_cast::powers::regenerate_creature_health`, `Creature::RegenerateHealth`): a creature with `RegenHealth` regains a third of its max health every 2 s while it is not engaged (TrinityCore `CREATURE_REGEN_INTERVAL`, `maxValue/3`, Creature.cpp:893, 1023); none while evading. While polymorphed (`IsPolymorphed`: the shown transform is a mage spell with family flag 0x1000000 and effect 0 MOD_CONFUSE) it regains 5% of its max health every 1 s, engaged or not: the user's Retail observation (2026-09-29, empty to full in 20 s), not TrinityCore, whose `maxValue/3` per 2 s fills a sheep in 6 s. Spell 118 `Description_lang`: "the victim cannot take actions but will regenerate health very quickly". Engaged and not polymorphed, a creature keeps its damage (`polymorph_tests.rs`). Player polymorph regeneration is not modelled.
- **Not modelled:** diminishing returns (Polymorph `DiminishType` 16), "Limit 1" single-target polymorph (TrinityCore `IsSingleTargetWith`), `ThreatManager::EvaluateSuppressed` (threat against a crowd-controlled unit), the PvP duration.

## Spell modifiers

- **Types:** ADD_FLAT_MODIFIER 107 and ADD_PCT_MODIFIER 108 match the aura spell's `SpellClassSet` plus an overlap between the effect's `SpellClassMask` and the target spell's `spell_class_options` mask. The label types 219 (flat) and 218 (pct) match `EffectMiscValue_1` against the target spell's `SpellLabel` rows (`SpellInfo.labels`).
- **Op:** `EffectMiscValue_0` is the `SpellModOp`.
- **Formula:** `(base + Σflat) × Π(1 + pct/100)`, with amount × stacks.
- **Charges:** a charged mod aura that affects a cast spell loses one charge on each cast and is removed at 0.

| Op | Name | Applied to |
|---:|---|---|
| 0 | HealingAndDamage | direct damage and heal amounts |
| 1 | Duration | aura duration, channel duration |
| 3, 12, 23, 32, 33 | PointsIndex0-4 | base points of that effect index |
| 4 | ProcCharges | aura charges |
| 5 | Range | max range check |
| 7 | CritChance | crit chance (percentage points) |
| 8 | Points | every effect's base points |
| 10 | ChangeCastTime | cast time (before spell haste) |
| 11 | Cooldown | spell and category cooldown |
| 14, 34, 39 | PowerCost0-2 | `SpellPower` row by `OrderIndex` |
| 15 | CritDamageAndHealing | crit bonus |
| 18 | ProcChance | proc chance of the aura |
| 19 | Period | periodic aura period |
| 21 | StartCooldown | GCD |
| 22 | PeriodicHealingAndDamage | periodic damage and heal ticks |
| 24 | BonusCoefficient | SP/AP coefficients |
| 37 | MaxAuraStacks | max stacks |

In-scope uses of the ops listed above, as (aura, op) pairs, number more than 30. The most common are (108, 0) with 624 effects, (108, 22) with 271, (219, 3) with 148, (218, 0) with 133 and (219, 12) with 91.

Not applied:
- 2 Hate: spell-specific threat modifiers are not applied; [school aura modifiers](creature-aggro.md#threat-and-engage) are.
- 6 Radius.
- 9 ResistPushback: no pushback.
- 17 ChainTargets: no chains.
- 20, 27 Amplitude.
- 31 Doses.
- 40 MaxTargets.
- The PvP modifier auras 647/648/649: there is no PvP combat context.

Examples:
- Unwavering Will 373456 cuts Flash Heal 2061's cast time to 1425 ms.
- Mental Agility 341167 halves Purify's cost to 65 mana.
- Shadow Priest 137033 scales Mind Blast 8092 by 0.93 × 4.00 (class mask) × 2.37 (label 4040).

## Cast pipeline integration

- **Costs:** a `SpellPower` row with `RequiredAuraSpellID` applies only while the caster has that aura. Priest costs are gated on the spec passives 137031/137032/137033/417191, so a priest pays the row of whichever spec passive it knows; with none known the spell is free.
- **Cast time:** ChangeCastTime mods, then ÷ `1 + CharacterStats.haste_pct/100`.
- **Charges:** max charges and recharge follow MOD_MAX_CHARGES 411, CHARGE_RECOVERY_MOD 453 and CHARGE_RECOVERY_MULTIPLIER 454 on the charge category (`EffectMiscValue_0`). Double Time 103827 gives Charge 100 2 charges and a 17 s recharge.
- **Power regen:** MOD_POWER_REGEN_PERCENT 110 and MOD_MANA_REGEN_PCT 379 scale regen and decay rates. Continuous pools and Mana add recipient/power-matched aura85 raw amount/5 afterward; discrete pet aura85 remains a separate unproved consumer. Native Mana proof: [[auras#Water Shield flat Mana — 2026-10-18]].
- **Max power:** MOD_INCREASE_ENERGY 35 and MOD_MAX_POWER 418 add their amount to the MiscValue power's max (`AuraStatMods.max_power_flat`; mana is not covered, its max comes from `stats.rs`).
- **Charged combo points:** SET_POWER_POINT_CHARGE 494 (NYI in TrinityCore) marks its MiscValue (1-based point index) charged: `AuraStatMods.charged_power_points`, ascending (Echoing Reprimand 354835 → 2-5, Supercharge 470398-470416 → 1-7). The finisher bonus of a charged point is not modelled.
- **Movement:** movement reads `AuraStatMods.movement_speed`. It replaced the unused `shared::aura::Auras` read. The combination is the strongest MOD_INCREASE_SPEED 31 × every MOD_SPEED_ALWAYS 129 × the strongest MOD_DECREASE_SPEED 33, and 0 under MOD_ROOT 26, MOD_ROOT_2 455 or MOD_STUN 12.
- **Crowd control:** cast validation fails with `Stunned` under MOD_STUN 12 (unless `SPELL_ATTR5_ALLOW_WHILE_STUNNED`), `Silenced` under MOD_SILENCE 27 or MOD_PACIFY_SILENCE 60 for `PreventionType` 1 spells, and `Pacified` under MOD_PACIFY 25 or 60 for `PreventionType` 2 spells. A cast already in progress is not interrupted.

## Water Shield flat Mana — 2026-10-18

Native70448 SpellEffect79509 supplies79949 effect1 aura85/Mana/10; effect79508's42→79950 sibling remains unchanged. SpellMisc58951/Duration6 supplies600000ms. TC a352b1fa StatSystem.cpp874-908 multiplies base-mana regeneration before adding flat85/5. The Elixir slice already collected recipient/power flat amounts, but separate authoritative Mana regeneration omitted them.

Atf4c17e35b, an ordinary Shaman Healing Surge8004 with native137039 cost eligibility creates a real deficit. Native137039 aura379 gives25% base regeneration:10000/100*1.25=125/s. An ordinary79949 should add10/5 afterward for127/s; rate/order and cap/prediction cases genuinely failed125vs127. Baseline, actual600-second expiry through native50ms frames, and foreign authenticated self-cast controls passed3; two genuine REDs took25.33s. The earlier746a harness error incorrectly read Mana through the non-mana-only `Harness::power`; that run is not regression evidence. Tests now read the unchanged authoritative Mana component.

The Mana consumer now adds the existing raw85 sum/5 after its multipliers; native spells, resources, frame clocks, proc behavior and definitions are unchanged. Definition-level GREEN pending. This is native79949, not Retail52127/legal-build/full shield-proc certification; discrete pet85/global census claims remain separate. [Requirement](../../specs/class-completeness.md); tests `spell_cast::water_shield_mana_tests::`; assigneds6 evidence `data/class-completeness6-2026-10-12/water-shield-mana85-*`.

## Energizing Elixir flat Energy — 2026-10-18

Native70448 SpellEffect129038 supplies115288 aura85, power3, amount75; SpellMisc88282/Duration28 supplies5000ms, and effect288813 energizes2Chi independently. TC a352b1fa StatSystem.cpp901-908 adds flat aura85/5 AFTER percentage modifiers. Existing ActiveAuras retained85, but AuraStatMods had no flat collection and power regeneration only multiplied native rates. Ordinary Tiger Palm's real50-Energy spend followed by ordinary115288 reproduced10vs25/s; actual13750 composition reproduced17.5vs32.5/s. Baseline/expiry/actual foreign self-cast controls passed3; two genuine REDs at3993b7fe8 took26.42s.

The modifier now stores raw five-second amounts by recipient/power; continuous non-mana pool rates add that sum/5 after multiplying native rates and before selecting refill/decay limits. Source metadata is not mutated. Atc62d61e9b/protocolf9a9175, native5/5 GREEN14.48s includes actual two-Chi preservation, removal, expiry, percentage order and foreign self-cast. Retained regeneration/class-resource30/30 GREEN14.31s covers existing mana/Focus/runes/Chi/caps/replication. Both have zero warnings; independent lead auras/check/format gate pending. Native spells/rows/powers/clocks remain unchanged. Legal talent/loadout acceptance, generic mana/pet85 and global aura85 census certification are not claimed. [Requirement](../../specs/class-completeness.md); tests: `spell_cast::energizing_elixir_tests::`; source and RED receipts: assigneds6 `data/class-completeness6-2026-10-12/energizing-elixir-*`.

## Stat modifiers

Every aura change recomputes two components, each written only when its value changed:

- **`stats::StatModifiers`** (additive percentages, [[character-stats]]): `update_character_stats` folds it into `CharacterStats`, so each stat below is applied exactly once, there.
  Percentages add across auras: two +10% stamina auras give +20%. TrinityCore multiplies them (`GetTotalAuraMultiplier`), which gives +21%. The results differ only when several auras modify the same stat.

| Field | Aura types |
|---|---|
| `primary_flat` | MOD_STAT 29 (MiscValue stat index 0 Str, 1 Agi, 2 Sta, 3 Int; negative = all) |
| `primary_pct` | MOD_TOTAL_STAT_PERCENTAGE 137 (MiscValueB stat bitmask; 0 = all, TrinityCore `HandleModTotalPercentStat`) |
| `crit_pct` | MOD_CRIT_PCT 290 |
| `haste_pct` | 138, 140, 192, 342, 193, 216, plus the strongest MOD_CASTING_SPEED_NOT_STACK 65 (retail haste is one stat) |
| `mastery_points` | MASTERY 318 |
| `max_health_pct` | MOD_INCREASE_HEALTH_PERCENT 133 |

  Consumers: spell crit rolls `CharacterStats.crit_pct` (+ CritChance mods); cast time divides by `1 + CharacterStats.haste_pct/100`. Ordinary NPCs without `CharacterStats` are unchanged. Approved [NPC follower bridge units](../investigations/follower-data.md#passive-derived-stat-refresh--2026-10-15) carry `CharacterStats` and now share player stat recomputation; their sourced modifiers apply without a synthetic `Player` identity.
- **`AuraStatMods`** (multipliers, `1 + amount/100` each): combat paths read it directly.

| Field | Aura types | Consumer |
|---|---|---|
| `damage_done_mult[7]` / `damage_taken_mult[7]` | 79 / 87 per school bit | spell, periodic and melee damage |
| `healing_done_mult` / `healing_taken_mult` | 136 / 118, 283 | heals |
| `autoattack_damage_mult` | 344 | melee swing |
| `autoattack_range_bonus` | 153 signed additive yards | attacker-only main/off-hand swing distance |
| `rage_from_damage_dealt_pct` | 213 | rage per melee swing dealt (summed percent) |
| `power_regen_mult` | 110 (by power), 379 (mana) | regen |
| `power_regen_flat` | 85 raw units/5s, summed by power | continuous pools and Mana after percent; discrete-pet85 unproved |
| `movement_speed` | 31, 129, 33, 26, 455, 12 | movement |
| `water_breathing` / `water_breathing_mult` | WATER_BREATHING 82 / MOD_WATER_BREATHING 155 | breath timer ([[swimming-and-breath]]) |

Aura amounts add SP/AP coefficients only for PERIODIC_DAMAGE 3, PERIODIC_HEAL 8 and SCHOOL_ABSORB 69; every other type uses its base points (Bloodlust 2825 is +30% haste regardless of spell power).

`MOD_THREAT` 10 is read directly from active aura effects at threat-generation time, not stored in `AuraStatMods`. See [creature threat](creature-aggro.md#threat-and-engage) for school matching and healing assist threat. Pinned world.db Righteous Fury 25780 has +950% threat across school mask 127; no hardcoded historical +43% modifier.

## Replication

- Every `Player` and `Npc` gets `ActiveAuras`, `UnitAuras` and `AuraStatMods`.
- `sync_unit_auras` rebuilds `UnitAuras` only for units whose aura set changed (apply, refresh, removal, charge use). It then marks the entity in `ReplicationDirtyQueue`.
- `remaining_ms` is measured when the view is built, and the client counts down from it. Ticks do not resend the view.
- `AuraView` fields:
  - `instance_id` is unique per unit;
  - `caster` is entity bits;
  - `stacks` and `charges`;
  - `duration_ms` is 0 for permanent auras;
  - `harmful` and `dispel_type`;
  - `flags` carry PASSIVE, HIDDEN and FROM_PLAYER.
- Combat log: `AuraApplied`, `AuraRefreshed` and `AuraRemoved` go to the owners of the caster and the unit, with `amount` = stacks. `Dispel` goes out as described above.

## Critical-damage school bonuses

Aura163 adds `(school_mask, multiplier)` entries to `AuraStatMods.critical_damage_mods`. The critical hit multiplies each matching effect once, so a school127 hit against separate mask1 and mask4 effects consumes both; a per-school maximum would lose one. Stacks scale the effect amount before its percentage multiplier. Mask0 matches nothing. Direct damage/leech and auto-attack critical paths consume these entries; ordinary hits and critical healing do not.

TC a352b1fa Unit.cpp:1222-1237/7297-7324 applies CritDamageAndHealing spellmods before163 for weapons, but after its critical-bonus calculation for magic. Existing floating damage and log rounding/truncation remain; no exact Retail integer-intermediate parity claim. Native-input damage tests cover both orderings, mixed masks, stack caps, noncritical/zero-mask behavior and unlearning; [contract/tests](../../specs/class-completeness.md#critical-damage-bonuses).

## Flat versatility bonuses

Aura471 contributes integer percentage points to `StatModifiers.versatility_pct`, outside rating diminishing returns; computed `CharacterStats` feeds existing spell/heal/weapon consumers. Incoming spell and auto damage now consumes half the combined percentage. Attr4IgnoreDamageTakenModifiers bypasses the new spell reduction. TC SpellAuraEffects.cpp4024–4034 and Unit.cpp6944/7036/8229 define these boundaries.

Pinned69933 local CASC GlobalCurve3752002 maps types5/11 to21024/21035. All123 CombatRatings1391669 levels have Taken's points-per-percent exactly twice Done's; native21035 is half-scaled21024 at every knot and clamped endpoint. Therefore half of the existing derived Done percentage is the native Taken rating contribution, not an invented rating conversion. Receipts: `data/class-completeness6-2026-10-12/versatility-rating-source-batch46.json`.

Nine native Bevy App controls cover computed3% Mark, actual spell/heal/taken/auto damage, additive3+4+3 points, passive unlearning/buff cancellation, low/high-DR combined factors, native262166's IgnoreDamageTakenModifiers and unowned-creature exclusion. One healing fixture removes only random variance; fixed powers/weapon/rating endpoints isolate math, not player-build availability. Charm/pet-owner inheritance and native-rendered parity remain open. [Contract](../../specs/class-completeness.md#versatility-bonuses).

## Category cooldown offsets

Aura341 contributes signed integer milliseconds to the caster's matching native spell category. `spell_cast::category_cooldown_modifier_ms` reads all live effects; `start_cooldowns` adds that total after ordinary cooldown/haste modifiers to both positive spell and category durations, then clamps at zero. Charge recovery and already-running deadlines are unchanged. TC a352b1fa SpellHistory.cpp469–500 is the runtime source; Player.cpp30693–30744 separately maintains the client's replicated category offset list, which this client does not expose as a separate list.

Native Counterspell/Quick Witted tests prove25s→20s recovery with real blocked/successful recasts. Native Optical Blast supplies both24s timers, using an Energy-bearing fixture; Angel's Mercy reduces Desperate Prayer90s→70s. Tests also cover category0 exclusion, absent talent, unlearning, learning during an active lockout and native30000−30000ms clamping. Cross-class/internal-trigger fixtures do not claim player spellbook availability. [Contract](../../specs/class-completeness.md#category-cooldown-modifiers).

## Cooldown recovery rates

Aura143 (`MOD_RECOVERY_RATE_BY_SPELL_LABEL`) shares the286 timer machinery but selects the affected spell by native `SpellLabel`: primary MiscValue OR nonzero MiscValueB, without the class-mask restriction. Source: pinned TC a352b1fa `SpellHistory.cpp448–451` and `SpellAuraEffects.cpp6271–6284`. Existing aura apply/refresh/remove hooks retime running spell/category deadlines; no new polling or alternate clock.

Normal native Blessing of Autumn388010 leaf casts produce its30% effect. Matching Divine Protection498 starts60000→46153ms; Hammer of Justice853 category32 starts45000→34615ms. Seven real-App controls cover those starts, already-running application, player cancellation, natural expiry plus actual blocked/successful recast, unlabelled Healing Potion439 and foreign recipient. Initial matching-start RED2/3 became GREEN7; all ten286 controls remain GREEN. Native rows contain no2960-labelled spell without16: the secondary-only selector and143-specific unchanged refresh are source-wired, not independently native-certified. The full Seasons cycle, Autumn's charge173, global ModTimeRate, legal loadout and rendered/full-ability coverage remain open. [Contract](../../specs/auras.md).

Aura286 matches native class masks and multiplies cooldowns by100/(100+amount), clamping the amount to−99. `spell_cast::recovery_rate` applies the factor after haste and before341 offsets. Aura apply/remove rescales only remaining time; unchanged refreshes do nothing, while changed native amounts undo/reapply the factor. Expiry follows the same removal path. Source: TC SpellHistory.cpp432–455/608–628 and SpellAuraEffects.cpp6258.

`SpellHistory.category_spell_ids` stores the spell that started each category timer: Sprint has only a category timer, so its class-mask association cannot be recovered from `spell_ready_at`. Active spell/category deadlines are updated once each and owner messages report the remaining maximum. Existing charge timers and GCDs are untouched.

Ten native App controls prove Rapid Fire16s→10s, Sprint120s→40s, real blocked/successful active-timer recasts, expiry, unchanged refresh, native Hot Hand label1935's zero→34 rate refresh, recipient/mask negatives and owner updates. Hot Hand's controlled refresh does not prove the random proc's availability; full abilities, global ModTimeRate interactions and native-rendered parity remain open. [Contract](../../specs/class-completeness.md#cooldown-recovery-rates).

## Script hooks

Know Your Enemy388118 and Penetrating Shots459783 share Critical Chaos's effect1-percent-of-owner-crit updater. They recalculate effect0 on apply/effect1 ticks; `auras_changed` refreshes stat multipliers for live damage. Source: TC spell_dh.cpp:1571-1593/632-655 and spell_hunter.cpp:869-891. Penetrating Shots uses native25% with1s ticks; real Aimed Shot casts retain native variance, coefficient, cast time and missile travel. Tests cover live crit changes, unlearning, zero-crit damage and healing isolation. Guaranteed-crit endpoints and cross-class healing fixtures are test boundaries, not player-build or talent-availability claims.

Ysera's Gift 145108 reads its owner's live max health on each authored 5 s effect-0 tick and floors its native 3% amount. Injured owners trigger 145109; full owners trigger the [raid-area group child](spell-casting.md#implicit-targets), 145110. `cast_triggered_with_effect0_amount` overrides only a cloned per-cast child; later direct casts retain the native 15-point definition. Both children carry Can'tCrit, enforced before crit modifiers/scripts. Six native Bevy App tests cover timing, live max health, self/group priority, 100% crit, unlearning, overflow, no-group isolation and override non-leakage. Source: TC a352b1fa `spell_dru_yseras_gift`, SharedDefines.h:539; [contract](../../specs/class-completeness.md#yseras-gift-periodic-healing).

Combat Wisdom 121817's effect-0 tick casts Power Strike Proc 129914 (first due tick immediately via ExtraInitialPeriod, then every 15 s). Its proc hook casts 121283 for one Chi before the generic charge consumer removes it. Combat Wisdom effect 1 remains unmodeled.

Critical Chaos 320413 calculates effect 0's ProcChance modifier as effect 1's percentage of owner crit on apply and each effect-1 tick. Chaos Strike's custom refund roll in `spell_cast/scripts/batch5.rs` consumes this modifier through `spell_mods::modify`; merely updating the live aura amount did not affect that custom roll. Native cast tests cover actual Fury gain, one roll per root, and a no-talent control. Source: TC a352b1fa `spell_dh_critical_chaos`, `spell_monk_power_strike_periodic` and `spell_monk_power_strike_proc`; [contract and tests](../../specs/class-completeness.md#periodic-class-proc-hooks).

`AuraScripts(HashMap<spell_id, AuraScript>)` holds four optional hooks: `on_apply`, `on_remove`, `on_periodic` and `on_proc`. Production registers Aspect of the Turtle186265 apply/remove and Barkskin22812's Brambles periodic hook. Other DUMMY4/PERIODIC_DUMMY226 values still require per-spell consumers; `AuraKind::Script` alone is not implementation. See [class scripts](spell-casting.md#class-dummy-scripts).

## Coverage

- Handled aura IDs (`handlers::kind`): 3, 4, 5, 8, 12, 20, 23, 24, 25, 26, 27, 29, 31, 33, 39, 42, 56, 60, 65, 69, 79, 82, 87, 89, 107, 108, 110, 118, 129, 133, 136, 137, 138, 140, 155, 192, 193, 213, 216, 218, 219, 226, 231, 271, 283, 290, 318, 342, 344, 379, 411, 453, 454, 455.
- Of the 95% cut (77 IDs, 6576 instances in [spell coverage](../../spell-coverage.md)), 36 IDs covering 5693 instances (87%) are handled. DUMMY4 is a registered type, not blanket spell-specific coverage; the current class hooks are listed above.
- Unhandled cut IDs, all tracked and replicated:
  - no pet or summon system: 429, 382;
  - client or visual only: 332 action bar override, 61 scale, 260 screen effect, 403 visual override, 44 tracking;
  - no mount, shapeshift or area-trigger system: 78, 36, 275, 395;
  - no PvP context: 647, 648, 649;
  - crowd control without a cast-fail mapping or movement model: 321 no actions;
  - no mechanics or diminishing returns: 77, 232;
  - no armor, block or avoidance model: 142, 51, 187, 229;
  - no haste-regen stat: 416, 417, 457;
  - no consumer: 418 max power, 443 leech;
  - not yet: 63 additional power cost, 213 rage from damage, 220 ability school, 285 linked, 468/396 trigger on health/power, 191/305 speed rules, 467 stat bonus pct, 320, 531.

## Sources

- [auras/mod.rs](../../../crates/server/src/auras/mod.rs) — instances, stacking, removal, replication, CancelAura, dispel, passives
- [auras/periodic.rs](../../../crates/server/src/auras/periodic.rs) — ticks
- [auras/procs.rs](../../../crates/server/src/auras/procs.rs) — events, procs, interrupts
- [auras/spell_mods.rs](../../../crates/server/src/auras/spell_mods.rs) — `SpellModOp`, matching, charges
- [auras/stat_mods.rs](../../../crates/server/src/auras/stat_mods.rs) — `StatModifiers` and `AuraStatMods` from auras
- [auras/handlers.rs](../../../crates/server/src/auras/handlers.rs) — aura type registry
- [auras/unit_state.rs](../../../crates/server/src/auras/unit_state.rs) — invisibility, unattackable, confuse, transform
- [auras/tests.rs](../../../crates/server/src/auras/tests.rs) — real-DB App tests
- [auras/polymorph_tests.rs](../../../crates/server/src/auras/polymorph_tests.rs) — Polymorph end to end
- TrinityCore `a352b1fa` `SpellAuraEffects.cpp` (`CalculateAmount`, `CheckEffectProc`, `HandleProc`, `HandleAuraTransform`, `HandleModConfuse`), `Unit.cpp` (`SetControlled`, `SetConfused`, `RestoreDisplayId`, `IsPolymorphed`), `ConfusedMovementGenerator.cpp`, `Creature.cpp` (`RegenerateHealth`), `SpellAuras.cpp` (`GetProcEffectMask`), `Spell.cpp` (`CheckCasterAuras`, `DoDamageAndTriggers`)
- [spec](../../specs/auras.md)
- TrinityCore master `a96d8977` `SpellAuras.cpp`, `SpellMgr.cpp` (`CanSpellTriggerProcOnEvent`, proc autogeneration), `SpellDefines.h` (`SpellModOp`, `SpellAuraInterruptFlags`)

## See Also

- [Class defensives audit](../../defensives-audit.md) — real-cast spell/melee proofs, catalog presence versus reproduced gaps (2026-10-09)

- [[spell-casting]] — the pipeline that applies auras and reads spell mods
- [[spell-info]] — aura options, interrupts, class options, labels
- [[class-progression]] — `KnownSpells` passives
- [[combat]] — melee swing events
- [[character-stats]] — `StatModifiers` fold-in

## Category charge-recovery rates

Aura148 uses its native MiscValue charge category and rate100/(100+max(amount,−99)). `auras::charge_mods` multiplies category effects alongside native453/454/457; `SpellHistory::charges_mut` floors native recovery milliseconds and preserves remaining-progress fractions. Existing `sync_charge_recovery` publishes changed clocks/maxima, including refresh/removal/expiry, without an additional cast. No new polling or alternate recovery path.

Pinned TC a352b1fa SpellHistory.cpp880–914 and1003–1005, SpellAuraEffects.cpp6285–6293 establish current/running semantics. Native Trueshot288613 effect1 uses40/category1715 for Aimed Shot19434. [Contract](../../specs/class-completeness.md#category-charge-recovery-rates). Type-mask/global-time/charm and complete ability/rendered proof remain open.

Batch48 proof: native Aimed Shot15,000→10,714ms, running application/removal owner receipts, unchanged refresh, serial two-charge expiry/replenishment, foreign recipient/category exclusions, independent Quick Shot150 product4,285ms, native Demonic Resolve−25/Fiery Brand slower recovery. The per-cast amount-refresh API fixture uses native Quick Shot150 as an override; it proves clock refresh only, not a Retail Quick Shot→Trueshot binding. Native−99 clamp edge/global time/charm/rendered parity remain unproved.

## Charred Warblades

Pinned TC a352b1fa `spell_dh.cpp480–519` checks Fire school, accumulates each proc into a uint32 and overrides child213011 effect0 on periodic effect0. Native213010 carries3%/1000ms and CanProcFromProcs. The implementation stores pending integer heals per aura instance on its recipient; normal proc masks/chance/trigger eligibility still apply. Proc scripts can inspect the existing AuraEvent before proc admission and consume its actual damage after a successful proc. Refresh retains instance state; removal discards it. No generic periodic-dummy certification. [Contract](../../specs/class-completeness.md#charred-warblades).

Eight native controls plus12 adjacent controls/check/fmt pass at d9d216d9. Three source-bounded memberships promote; full class/talent and rendered parity remain open. [Exact proof ledger and fixture corrections](../../class-completeness6.md#native-charred-warblades--batch49).

## Damage versus creature type

Aura168 stores `(creature mask,multiplier)` pairs in `AuraStatMods`, combining matching entries multiplicatively on actual spell/periodic and main/off-hand damage, independent of school. The shared `unit_creature_type.rs` resolver uses NPC template type; player positive native form type, else exact race type, and also serves target admission. Runtime loads required `chr_races.CreatureType` through the world connection; `prepare_site_catalog.py` retains the exact selected product CSV field. Canonical59-row equality is verified. No inferred Humanoid or generated source map. Zero masks/amounts do not boost damage; removal/expiry/reapplication, native Attr3/6 exemptions, resolved copies and healing retain their contracts. [Source/behavior receipts and boundaries](../../class-completeness6.md#native-creature-type-damage--batch54), [contract](../../specs/class-completeness.md). Generic tracking44 and whole-ability/talent/rendered acceptance remain separate.

## Caster-owned school amplification — 2026-10-16

Native70391 Haunt48181 effect1 is aura270, amount16, school mask127. The shared victim multiplier handled family271 and label537 but omitted270, so Haunt applied normally without increasing the owner's damage. TC a352b1fa `Unit.cpp:7056–7059/8178–8181` matches both aura caster and incoming school. The same multiplier now handles270 using the existing amount/stack product and IgnoreDamageTakenModifiers boundary. Autoswings do not use this special-spell condition.

Ordinary HostClient Haunt→Fireball reproduces the missing bonus (386 health loss, expected native16% boosted439–461); baseline/foreign/school exclusions and removal/expiry controls pass before the fix. `spell_cast/caster_school_damage_tests.rs` retains this boundary and two actual2000ms Corruption172→146739 ticks, each amplified by16%. Final targeted run at96785a2f passes4/0/0 (8.35s). [Contract](../../specs/class-completeness.md#caster-owned-school-damage-taken). Haunt381/380, optional periodic talents, full Affliction and rendered/UDP parity remain open.
