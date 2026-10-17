# Level-80 class completeness audit and shared cast behavior

Source: `spell_cast`, `auras`, `spell_info`; [implementation and proof](../class-completeness.md), [spell-casting system](../wiki/systems/spell-casting.md#level80-class-completeness).

## What it must do

- [x] Effect64 dispatch queues its child using native `MiscValue` milliseconds instead of executing reentrantly inside the parent. Dispatch uses stable due-time/insertion order and the existing virtual clock/depth limit; nested zero-delay callbacks follow the current callback, and positive delays require time advance. Native0ms298992→181448 and1ms412677→412680 expose the boundary without clock edits. This permits required `CasterAuraSpell` on triggered children while preserving real native producers, exclusions/removal and foreign-recipient controls. Effect32/140 timing, effect64 native cost/reagent retention and general launch-vs-hit phase parity remain separate.

- [x] Triggered casts refuse an existing active native `ExcludeCasterAuraSpell` on the actual caster, sharing the ordinary exclusion predicate. Native146481→146480 excludes146543 supplied by an ordinary self cast; removal and foreign-recipient controls preserve the child. Triggered required-caster refs follow the queued effect64 contract above; no pre-granted markers, early effect activation or requirement bypass may substitute for that ordering.

- [x] Triggered casts with an explicit unit still enforce the implemented target aura-spell/type restrictions before child launch. Native full-trigger flags do not suppress TARGET_AURASTATE; relation bypass is separate. Native412677→412680 must create the180-second child once, not refresh it while its excluded target aura remains, and resume after removal without borrowing another unit's aura. Caster-trigger requirements follow the queued effect64 contract above; missing explicit/auto-self resolution, other implicit targets and full flag compatibility remain separate.

- [x] Ordinary casts require a nonzero native `TargetAuraType` and reject a present `ExcludeTargetAuraType` on the resolved explicit/auto-self target, after aura-spell references. Presence comes from active effect types, not amounts, source spell-ID lists or another unit's auras; removal/expiry reverses eligibility. Refusal uses TargetAuraState/detail. Native1293603/1293610 indicators and440490/448026 Headlock variants provide controlled metadata fixtures, not implemented highlight UI/dummy scripts or complete Headlock/state1 mechanics. Caster types, implicit/area filtering, full-trigger target checks and aura485 effect semantics remain separate.

- [x] Ordinary casts require the actual caster's native `CasterAuraSpell` when nonzero, at admission and completion, before excluded-spell checks and effect/payment launch. Recipient-only auras do not qualify; removal/expiry restores refusal, ordinary NPCs share the check, and triggered caster references follow the queued effect64 contract above. Refusal uses the existing source CasterAuraState/detail contract. Void Torrent205065 consumes Voidform194249 from the maintained228260 producer; Sheilun's Gift205406 consumes214502 identity, not guessed cloud amounts/stacks. TargetAuraSpell remains separate:322109's1221044 notification needs an authenticated producer policy, not injected markers or weakened health-only tests.

- [x] Ordinary player/NPC casts enforce native `ExcludeCasterAuraSpell` on the caster and `ExcludeTargetAuraSpell` on the resolved explicit/auto-self unit, at admission and completion. Active aura identity is sufficient regardless of amount; recipient mismatch, removal and expiry do not block. Admission refusal starts no GCD/spell cooldown, spends no resources or charges and applies no effects. Completion refusal preserves the already-started GCD but does not launch/spend. Triggered caster/explicit-target references follow the contracts above. Carry source `SPELL_FAILED_CASTER_AURASTATE`/`SPELL_FAILED_TARGET_AURASTATE` through the existing InvalidTarget/detail wire contract. Required aura refs, states, aura types and other implicit/area targets remain separate.

- [x] `SpellInfo` retains all twelve native required/excluded caster/target aura-state, spell and type IDs from `spell_aura_restrictions` at DifficultyID0. Missing rows stay absent and orphan rows create no spells; missing table/columns fail explicitly instead of silently loading unrestricted metadata. Canonical source publication precedes use of this loader. State production/admission and Massacre/Execute eligibility remain separate.

- [x] The maintained DB2 importer retains every native SpellAuraRestrictions state, spell-reference, type and exclusion column in `spell_aura_restrictions`, with the selected build receipt and normal header validation/atomic replacement. Reimport replaces prior rows and preserves non-DB2 data. This producer contract does not certify runtime aura-state admission, Massacre/Execute eligibility or canonical publication.

- [x] Native aura63 contributes its signed caster-owned sum only to optional resource cost, matching both power type and spell family/mask before existing cost modifiers. Deeper/Devious Stratagem's native extra cap permits six/seven-point finishers to spend those available points; smaller pools remain admissible and unrelated base Energy costs stay unchanged. Unlearning removes only that talent's contribution. Native OptionalCostPct fields and full talent/finisher acceptance remain separate.

- [x] Optional resource spend ignores flat spell modifiers while retaining percentage modifiers for the native cost order, family/mask and caster. Practiced Strikes429647 discounts only Protection Execute's base cost; native Sudden Death52437 effect5 still makes order1 base/optional spend free without changing Arms order0. Existing available-resource clamping remains unchanged.

- [x] Native aura334 adds signed attacker-owned percentage points only to autoattack critical chance, summing applied amounts/stacks before the nonnegative clamp. Removal ends its contribution; victim-only auras and magic/weapon spell crit remain independent. Avenging Wrath31884 also has aura290, which continues through normal stat computation; this consumer does not replace or double-count it. Full talent amount bindings, legal gear and fresh native acceptance remain separate.

- [ ] At the existing live-player melee weapon-spell dodge boundary, caster aura251 contributes its signed active total before expertise subtraction and final nonnegative clamp. Positive and negative native effects sum on the actual attacker; recipient-only effects do not contribute, and removal ends the modifier. Keep native facing/control/attribute/ranged exclusions. NPC dodge, aura248, player expertise and full251/ability acceptance remain separate.

- [ ] Physical weapon-spell resistance consumes victim aura117 totals for the effect mechanic when nonzero, otherwise the spell category mechanic; take the nonnegative maximum total across selected mechanics, not their sum. Add its truncated hundredths after miss and before NoActiveDefense on the same native roll; resisted casts apply no stun/control effects. Recipient ownership, removal and mismatched selectors retain native behavior. Magic probability and full-ability acceptance remain separate.

- [ ] Non-player attacker native aura240 expertise reduces live player melee weapon-spell dodge by one percentage point per four expertise, clamped at zero. Only the actual caster's active expertise contributes; removal restores native avoidance. Real NPC Hit Buffs62627 and native Strike38598 against a Rogue's real Evasion exercise the boundary without fake AP/dodge values. Player expertise, NPC dodge, parry, generic aura-handler/census and whole-spell certification remain separate requirements.

- [ ] Existing live player DodgePercentage feeds melee weapon-spell hit resolution before block on the same cumulative native roll. Native Evasion5277 self-cast must prevent frontal melee damage; behind-player attacks, controlled/casting states, NoAttackDodge/NoActiveDefense and ranged attacks retain source exclusions. Removal/expiry restore the live value. NPC dodge, player expertise/248/251 modifiers and complete hit probabilities remain distinct unproved boundaries.

- [ ] Implemented weapon-spell miss, ranged deflection and block chances share one native integer roll0..9999 with cumulative probabilities truncated to hundredths of a percent. NativeNoAttackMiss/NoActiveDefense and existing target guards retain their ordering. Magic hit behavior remains separate. Missing NPC dodge/parry and player baseline hit statistics remain explicit gaps; physical mechanic resistance has its own bounded requirement above, while magic remains unproved; this bounded table does not certify the full source hit system.

- [ ] Native weapon-spell blocks on ordinary NPCs use authenticated template `flags_extra` (NO_BLOCK16, distinct from NO_XP64), 3% plus victim aura51 and 1.5% per positive effective level difference. They stop native30% damage, or60% on a critical block, before shields. Shared facing/control/attribute exclusions remain intact. Missing template authority fails explicitly. Totem identity requires the source summon runtime, never creature type or title alone; unsupported Totem/Lightwell spawning and full cumulative hit-table ordering remain separate gaps.

- [x] Record native cast/passive-sync outcomes, warning tuples and raw resources for all 13 classes/all 40 local Retail specs, using level-80 actors and real spell rows, without writing world.db or opening a realm port.
- [x] Preserve exact missing effect/aura types and source/state prerequisites; never close unidentified Dummy fields with no-ops or guessed coefficients.
- [x] Census uses authored production destinations/maps, actor identity and group state rather than reporting absent fixture startup resources as product gaps.
- [x] Druid census exercises required, non-excluded Cat/Bear/Moonkin forms through real native casts. Preserve separate prerequisite cast/observed-form/warning/resource receipts; never inject form auras or bypass admission. Moonkin setup does not certify talent eligibility or combinations. Unmatched forms, prerequisite failures and underlying mechanics remain explicit gaps.
- [x] Native Prowl5215/Tiger Dash252216 automatically cast Cat768 BeforeCast; Incapacitating Roar99/Berserk50334/Stampeding Roar106898 cast Bear5487, including replacing the opposite form. Same form does not recast/refresh; initial refusals do not transform. Normal and triggered casts use real child effects. Census records actual post-cast form separately from prerequisites and unresolved child mechanics.
- [x] Successful native Berserk50334 AfterCast resets existing Mangle33917/Thrash77758/Growl6795 spell/category cooldowns, preserving charges, GCDs, unrelated spells and other casters. Rejected casts do not reset; native update=false does not invent an owner reset packet. Source category-only recovery still owns a spell-history entry until its native deadline.
- [x] Native player aura166 and167 modify melee and ranged attack power independently with multiplicative applied percentages. Player ranged AP uses source ChrClasses.RangedAttackPowerPerAgility times level plus nonnegative live Agility, never a melee-AP substitute. Native non-melee ranged-weapon spell coefficients consume ranged AP; melee coefficients and AP-derived spell/healing power retain melee AP. Hunter pet owner scaling uses ranged AP. Removal, expiry, reapplication, independent amounts/ownership and zero native fields preserve the separate values. Existing unmodeled weapon/flat-AP additions and NPC/follower percentage consumers remain explicit limits, not full AP-system certification.
- [ ] Replicate source ranged AP separately without changing the existing melee AP field's meaning; native ranged spell tooltip coefficients and explicit RAP tokens consume it rather than the melee value. Preserve melee/SP/other token behavior and source spell classification.
- [x] Native aura168 multiplies attacker damage against matching victim creature-type bits, independent of school, per applied amount/stack. Spell/periodic and main/off-hand consumers preserve removal, expiry, reapplication, zero-mask/zero-amount identity, ignored caster-modifier flags, already-resolved copies and healing. Tracking44 remains a separate requirement.
- [x] Creature type uses an NPC's source template; a player's positive native form type, otherwise exact source race type. Target-type admission shares this authority. Unknown race/template data is explicit, never invented Humanoid7. Runtime loads mandatory `chr_races.CreatureType` from the world connection; copy-only catalog preparation retains exact selected Retail/Forever metadata without changing prior race identity/faction.
- [x] Native aura153 adds signed attacker-owned yards to autoattack range, summing independent applied effects/stacks. Main/off-hand swings consume it at the inclusive distance boundary; removal/expiry and refreshed amounts update range. Victim-owned modifiers and zero amounts do not extend attacker range, and weapon-spell range remains independent. Combat-reach geometry and unnamed talent-to-form amount bindings are separate, unproved requirements.
- [x] Native target24 selects hostile units within the authored caster-facing cone/radius.
- [x] Native Shockwave46968 and Storm Bolt107570 effect0 damage hits cast their source stun children132168/132169 before the default damage handler. Shockwave preserves ordinary child validation except cast-in-progress and suppressed cast-error reporting; Storm Bolt uses the source full triggered mask. A caster without Storm Bolts436162 selects only the explicit Storm Bolt target; its own talent retains native destination-area targets. Stun immunity, missile flight, child expiry and source effect boundaries remain independent.
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

- [x] Native aura7 runs players and creatures along collision-limited navmesh flee paths. Native caster distance subtracts both source combat reaches; random points use quiet distances28..43 and the pinned near/far/quiet rules, paths≤30yd, LOS200ms/path100ms retry and800..1500ms pause. Players lose movement control; last-aura removal/expiry stops fleeing and restores its owned control without releasing a newer owner. Missing navigation/model metadata never becomes guessed direct movement.
- [x] Native aura7 makes the unit fleeing: its cast stops, auto-attacks stop, a creature shows no target and stops chasing, and a fleeing victim cannot dodge, parry or block. Removal restores swings and the creature's target.
- [x] A fleeing caster fails with "Can't do that while fleeing" unless the spell purges every active fear effect through its own mechanic, school or definition immunity (Attr1 ImmunityPurgesEffect). Spells with Attr5 AllowWhileFleeing pass only when each fear spell's mechanics intersect the spell's allowed mechanics, including TC's Barkskin/Dispersion/Lichborne cases.
- [ ] Flee movement (TC FleeingMovementGenerator) is pending a direction decision: pinned TC adds the owner's orientation to the away-from-caster angle. Aura7 therefore remains an unhandled census type. Player client-control loss is not modelled.

## Armor percentage

- [x] Native aura101 with the physical bit multiplies final armor as TC's armor TOTAL_PCT: after base/gear and stat-derived armor, alongside aura466, with independent auras multiplying. Creatures (including level-scaled ones) multiply their armor before the attacker-level armor multiplier. Removal restores the remaining factors. Nonphysical resistance bits are logged unmodelled.

## Class trans-door portals

- [x] All 29 native class TRANS_DOOR (effect50) spells spawn their authored TDB type22 spellcaster objects; an absent template fails cast admission with its reason instead of silently succeeding.

## Dual wield and parry knowledge

- [x] Knowing a native SPELL_EFFECT_DUAL_WIELD (40) spell permits one-hand and off-hand weapons in the off hand; without it they fail with CantDualWield, and auto-equip offers only the main hand. One-hand polearms never fit the off hand.
- [x] Learned native Titan's Grip effect155 permits two-hand weapon hand placement with native subclass masks; the off hand additionally needs Dual Wield40. Explicit and auto-equip preserve the permitted main/off-hand pair. This does not establish acquisition or whole-Fury acceptance. See [inventory](inventory.md).
- [ ] When a player stops knowing Dual Wield (spec/talent change), TC moves the off-hand weapon to the bags or mails it (OffhandCheckAtSpellUnlearn defaults on). Not yet modelled.
- [x] Knowing any of the six native SPELL_EFFECT_PARRY (22) spells enables parry; without one, parry chance never applies.

## Touch of Death admission

- [x] Native322109 refuses a healthy stronger target before CastSuccess or cooldown publication. Its baseline admits creatures whose current health is strictly below the caster's maximum health; learned passive322113 additionally admits targets strictly below its authored health percentage. The native15% threshold is exclusive. PvP integration, Stagger removal and other follow-up talents remain separate, uncertified paths.
- [x] Native322109 primary damage uses the caster's live maximum health and authored100%/35% fields for weaker creatures/other admitted targets. Native Can'tCrit, ignore caster percent damage modifiers, and ignore victim school/caster/versatility modifiers plus armor retain their source exclusions. These flags do not disable ordinary spells' modifiers. Mechanic-specific damage-taken auras, PvP integration, Stagger removal and other follow-up talents remain separate.

## Melee attack speed

- [x] Native aura319 multiplies melee swing time by 100/(100+amount) per aura (negative amounts by (100−amount)/100) for players and creatures, independently of haste and cast time.

## Tiger Dash decay

- [x] Native Tiger Dash 252216 starts at its effect0 run-speed bonus and loses effect1's amount on every 500 ms periodic tick (TC spell_dru_tiger_dash_aura); expiry restores normal speed.

## Bloodsurge

- [x] Native Bloodsurge 384361 rolls every 3 s periodic tick with chance sqrt(units carrying the warrior's own Rend 388539 within 50 yd) × its amount and energizes 384362 on success; Rends from another caster do not count.

## Demon Hunter Metamorphosis toggles

- [x] Monster Rising 452414 and Enduring Torment 452410 apply their buff (452550, 453314) on each 1 s tick outside Metamorphosis (Havoc/Vengeance, Havoc/Devourer) and remove it inside. A player's 453314 keeps effects 0–1 only as Havoc and 2–3 only as Devourer.

## Periodic class proc hooks

- [x] Combat Wisdom 121817 casts Power Strike Proc 129914 on each effect-0 tick, immediately on the first due tick (authored ExtraInitialPeriod) and then every 15 s. Effect 1 remains unmodeled.
- [x] Power Strike Proc 129914 casts native 121283 for one Chi on its authored melee-ability proc event and consumes its one charge; a later ability cannot trigger the spent proc.
- [x] Critical Chaos 320413 adds effect 1's percentage of the owner's crit chance to Chaos Strike's proc chance on application and recalculates it on each 1 s effect-1 tick. Chaos Strike's refund roll consumes that live modifier once per landed root, not per weapon child.

## Raid-area healing

- [x] For player casts, native selector 56 selects living members of the caster's party/raid and their controlled units in range and the same map copy, not just the caster. ExcludeCaster applies before caps.
- [x] Ysera's Gift group heal 145110 selects one target, preferring injured players over full-health members; outsiders and the caster are excluded. Charmed controllers and NPCs marked TreatAsRaidUnit through unmodeled CreatureStaticFlags remain outside this proof.

## Ysera's Gift periodic healing

- [x] Native passive 145108's effect-0 tick heals 3% of the owner's live maximum health every 5 s, through 145109 when the owner is injured and 145110 when full. The first tick is delayed by the authored period.
- [x] Both heal children honor Can'tCrit even with 100% crit; normal health clamping and overflow logging remain intact. Unlearning the passive removes its ticks.

## Critical-damage bonuses

- [x] Aura163 multiplies existing critical-damage paths by every matching school-mask effect once, including mixed-school hits and melee auto-attacks; it does not modify critical healing.
- [x] Know Your Enemy388118 calculates effect0 from effect1's native40% of the owner's crit on application and every1s effect1 tick; live damage consumes the recalculated amount.
- [x] Penetrating Shots459783 calculates effect0 from effect1's native25% of the owner's crit on application and every1s effect1 tick; real ranged critical damage consumes the live bonus. Noncritical damage and healing remain unchanged; unlearning removes the bonus.
- [x] CritDamageAndHealing spell modifiers precede aura163 for weapon attacks and follow its critical bonus for magic. Existing floating damage/final-rounding behavior is retained; exact Retail intermediate-integer parity is not asserted.

## Category cooldown modifiers

- [x] Aura341 sums signed milliseconds for the spell's native nonzero category and applies after cooldown spellmods/haste to each positive spell/category cooldown, clamping at zero. It does not create cooldowns for zero-duration entries, retime running cooldowns or alter charge recovery.
- [x] Native Quick Witted382297 reduces Counterspell2139 category88 lockout from25s to20s; real casts remain blocked before recovery and succeed afterward.

## Versatility bonuses

- [x] Aura471 adds native integer percentage points to computed player versatility outside rating diminishing returns. Actual spell damage and healing consume the combined value.
- [x] Incoming spell and auto-attack damage use half the combined rating/aura versatility percentage, with damage clamped at zero. Spells carrying IgnoreDamageTakenModifiers bypass this new reduction.
- [ ] Full charm/pet-owner inheritance and native-rendered parity remain unproved; base-probe status is not a whole-ability certificate.

## Cooldown recovery rates

- [x] Aura286 multiplies matching native class-mask cooldown durations by100/(100+amount), with amount floored at−99. Application/removal scales only remaining spell/category time; category timers retain their owning spell for exact mask matching. Existing charge recovery remains separate.
- [x] Native Trueshot288613 rate60 starts Rapid Fire257044's16s cooldown at10s. Native Deadly Pursuit1259614 rate200 scales Sprint2983's category44 cooldown120s→40s; real active-timer recasts prove application/removal boundaries.
- [x] Unchanged refreshes do not apply the factor twice; changed native Hot Hand modifiers replace the old factor, expiry restores remaining time, and affected owners receive current spell/category cooldown updates. Other recipients and out-of-mask spells remain unchanged.
- [ ] Full Trueshot/Deadly Pursuit ability behavior, global ModTimeRate interactions and native-rendered parity remain unproved.

## Tests asserting this spec

- `crates/server/src/auras/crit_damage_bonus_tests.rs`: actual static/mixed-school damage, magic/weapon spellmod ordering, dynamic crit percentage, auto-attacks and no crit-heal leakage.

- `crates/server/src/auras/yseras_gift_tests.rs`: native timing, owner-max scaling, self/group switching, unlearning, overflow and Can'tCrit.

- `crates/server/src/spell_cast/raid_area_tests.rs`: real native group healing, one target and injured-member priority.

- `crates/server/src/auras/power_strike_tests.rs`, `critical_chaos_tests.rs`: native periodic proc application and live crit-scaled proc modifier.

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
- `crates/server/src/inventory_titans_grip_tests.rs`: native46917/120553 two-hand placement, both ordering directions, auto-equip, subclass and missing/unlearned capability controls.
- `crates/server/src/auras/defensives_tests.rs`: six native Parry spells versus none.
- `crates/server/src/spell_cast/attack_speed_tests.rs`: native Ruthless Aggression/Swift as a Coursing River creature swing intervals and unchanged Fireball cast time.
- `crates/server/src/auras/tiger_dash_tests.rs`: native Tiger Dash per-tick decay and expiry.
- `crates/server/src/auras/bloodsurge_tests.rs`: native Bloodsurge certain (12 owned Rends) and zero-chance (none, foreign) controls.
- `crates/server/src/auras/monster_rising_tests.rs`, `enduring_torment_tests.rs`: native tick toggles, Metamorphosis removal/return and spec-filtered buff effects.
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

## Readiness elapsed time

- [x] Spell/category cooldowns, GCDs, school lockouts and charge recovery use real elapsed server time, including slow frames, matching owner timer milliseconds. Existing rate modifiers, charge restoration, readiness credit and cooldown resets use that same clock. Simulation catch-up, cast/channel progression, aura expiry and missiles retain their existing clocks.
- Source: TC a352b1fa `SpellHistory.cpp:834–855` consumes charges against GameTime. Native70391 Overpower7384/category1680 recovers one charge in12000ms; Throw Glaive185123/category1612 in9000ms; both and Slam1464 use GCD category133/1500ms before existing modifiers.

## Category charge-recovery rates

- [x] Native aura148 applies100/(100+max(amount,−99)) per matching charge-category effect to future recovery durations. Existing running recovery preserves completed progress on application, amount refresh, removal and expiry; queued charges retain serial recovery and owner updates. Native Trueshot effect1 amount40/category1715 must accelerate actual Aimed Shot recovery.
- Type-mask rates, global ModTimeRate, charm inheritance and full Trueshot/rendered parity remain separate unproved boundaries.

## Charred Warblades

- [x] Native213010 must collect eligible owner Fire-school damage using effect0's native percentage, truncate each event contribution, cast self213011 once at the1000ms tick and clear accumulated damage after ticking/removal. Non-Fire damage, other actors and already-spent contributions must not heal. Refresh of the same aura preserves pending contributions.

### Distinct owner attack powers

The replicated owner stats carry distinct melee and ranged attack power. Ranged-defense spell coefficient text uses ranged power, matching native damage selection; other spells and explicit `$AP` tokens retain melee power. Missing owner stats must not synthesize either value from the other.

### Ordinary NPC physical-spell dodge

Ordinary NPC melee-spell dodge uses native3% plus victim aura49 and1.5% per positive effective-level difference. Rear attacks may be dodged in PvE. Controlled/casting victims, ranged attacks and native avoidance/ignore-result attributes retain their exclusions. Signed attacker251 and nonplayer expertise240 apply before the final nonnegative clamp. Specialized Totem/Lightwell spawning and player expertise/248 remain separate gaps; template creature type must not impersonate runtime IsTotem.

## Caster-owned school damage taken

- [x] Aura270 multiplies incoming spell damage only for the aura caster and overlapping native school masks, using live amounts/stacks; removal and expiry remove the bonus. Existing IgnoreDamageTakenModifiers bypass remains in force.
- [x] Native Haunt48181 effect1 increases the owner's actual Fireball damage by16%; foreign Haunt and physical-only Open Wounds188400 do not amplify Fire damage.
- [ ] Haunt pet amplification381, unknown380, optional periodic/talent behavior and fresh UDP acceptance remain separate.
