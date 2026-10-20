# Server auras

Server-side aura runtime in `crates/server/src/auras/`: aura instances on active units, periodic effects, procs, spell and stat modifiers, removal and `UnitAuras` replication. See [auras](../wiki/systems/auras.md).

## Computed target aura state

TargetAuraState/ExcludeTargetAuraState checks use only sourced, computable states. State15 (Druid periodic heal) derives from active aura source class family7 with mask0&0x50 or mask1&0x04000000, after TC's category1133 classification. Any caster's matching active aura qualifies:15 is not in TC's per-caster mask14/16. Owner-target and modelled vehicle-caster exemptions, plus affecting aura262, bypass state checks only, not other spell/type restrictions. Unknown states remain `TARGET_AURA_STATE_UNCOMPUTED`, never a reason to refuse a spell. Charmer and unmodelled vehicle representations remain explicit gaps; do not synthesize them. Follower owner mapping is [named project policy](follower-dungeons.md#full-spec-follower-kits-user-decision-2026-10-18).

## Germination Rejuvenation routing

With actual learned155675, ordinary774 must apply native155777 instead when its caster already owns774 and has no own155777, or when own155777 has less remaining time than own774. Otherwise apply/refresh774 normally. Compare only that caster's actual aura deadlines, retaining the other instance's absolute expiry. Untalented casts must remain single774 refreshes. Source: SimC76f8118 `sc_druid.cpp:6487–6518`; child155777 uses its native periodic heal effects/duration. Preserve ordinary parent costs, GCD, cast receipt and generic aura behavior; no timer/amount overrides, Swiftmend interaction or global stacking changes.

## Swiftmend own-HoT consumption

On a successful native18562 hit, consume exactly one HoT owned by that caster from the actual hit patient, in Regrowth8936 → Wild Growth48438 → Rejuvenation774 order (SimC76f8118 `swiftmend_t::impact`). Preserve other casters' instances and lower-priority HoTs. Actual learned native Verdant Infusion392410 disables consumption; no fabricated talent grants or aura removals. This hook retains ordinary healing/cost/timing/eligibility and does not implement the consumed-HoT bonus healing, which remains a named unimplemented effect outside this slice.

## Abundance native threshold

Actual learned207383 grants one native207640 buff only while the caster has at least the native threshold of own primary Rejuvenation774 auras active (70448 effect0 threshold5, buff CumulativeAura1). Reconcile on ordinary774 application/removal; natural expiry below the threshold removes the buff. Preserve native207640 cost/crit effects and duration, without old TC per-Rejuvenation stack increments. Germination counting, duplicate-refresh semantics and other-caster policy changes are outside this slice.

## Restoration Soul of the Forest

Successful Swiftmend18562 with actual learned158478 grants native114108 to the caster, not the healed patient. The next Regrowth8936 or Rejuvenation774 consumes the buff after resolving its effects. Use the buff's actual effect0 percentage (native60), as SimC76f8118's persistent multiplier: direct Regrowth healing and the affected periodic-heal snapshot gain the bonus, which remains on that snapshot after buff removal. Never reread the live buff to amplify old periodic ticks. Germination's Rejuvenation child uses the same sourced snapshot rule; parent774 retains consumption/cost ownership. Untalented casts and unrelated heals stay unchanged. Extra targets392302 and other Soul-of-Forest variants are excluded.

## Lifebloom near-expiration refresh

`LIFEBLOOM_REFRESH_BLOOM_THRESHOLD_SIMC`: ordinary33763 refreshing its caster's existing33763 on the hit patient triggers one native33778 heal when the old remaining duration is inclusively at most30% of its full duration (SimC76f8118 `sc_druid.cpp:6229–6240`). Inspect the old own deadline before ordinary aura refresh replaces it. Early refresh does not bloom. Keep the refreshed HoT active with ordinary duration/period/costs; retain existing native expiry/dispel bloom and no-bloom target-swap behavior. Native70448's tooltip names the near-expiration refresh bloom, while the exact threshold is sourced from SimC, not DB2. Other Lifebloom talents, Omen, target selection and global refresh/timer changes are excluded.

## Wild Growth falloff policy

Native48438 periodic heals use `WILD_GROWTH_FALLOFF_SIMC_SHAPE_ON_NATIVE_COEFFICIENT`: `c(t) = native_coefficient + decay/2 - decay*elapsedFraction`, with `decay = 0.07*(1-native382559percentage/100)` when Unstoppable Growth is actually learned, else0.07. Keep Retail70448's native0.481 coefficient, native period, full duration and natural expiry. Use each tick's scheduled time, not the update frame's wall time. SimC76f8118 `sc_druid.cpp:6611–6635` supplies only the explicitly approved shape; its historical0.175 and TrinityCore's legacy curve must not replace native data. Absolute Retail shape remains uncertified. This slice covers real level42 decline, level43's native40%-less normalized decay, and finite native ticks/expiry; target selection/caps, Tree of Life and other talent interactions are excluded.

## Restoration Omen and Clearcasting

Actual learned113043 rolls its native effect0 percentage (70448 value4) only on its caster's real periodic33763 healing, triggering native16870. Native DB ProcChance100 is admission metadata, not a substitute for that effect-amount roll (TC a352b1fa `spell_druid.cpp:1871–1882`; SimC76f8118 `sc_druid.cpp:6247,11532–11535`). Clearcasting's existing native16870 modifier makes the next Regrowth8936 free; consume the caster-owned buff when that Regrowth successfully lands, preserving the composed Soul-of-the-Forest hit hook. Unrelated Rejuvenation must neither consume nor refresh Clearcasting. Retain ordinary mana, regeneration, costs, timing and spell-modifier behavior. Natural acquisition tests use a confidence bound derived from native chance, never a gameplay timer or fake buff/RNG/resource grant. Moment of Clarity155577, extra charges, healing bonus392220, other Omen variants and global proc changes are excluded.

## Tranquil Mind Clearcasting stacks

Actual learned403521 replaces Omen113043's proc chance with its native effect2 value (70448 value5%, not +5%). Native effect0/op37 supplies one additional Clearcasting16870 stack through the existing modifier runtime: preserve cap1 without the talent and cap2 with it, including a third real proc at the cap. Each successfully completed Regrowth spends exactly one owned Clearcasting stack without refreshing its deadline; two earned stacks fund two free Regrowths, then ordinary mana cost resumes. Retain the existing Soul-of-the-Forest hit composition and generic stacking/cost behavior. Confidence bounds for natural acquisition account for multiple required procs within the actual finite native lifetime; they are not gameplay timers or frequency certification. Index1 mana-regeneration numeric certification, other extra-charge talents and global stacking changes are excluded.

## What it must do

- [x] Apply aura effects (6, 35, 65) as one instance per target and resolution, with per-effect amounts from base points, SP/AP coefficients and the caster's spell modifiers.
- [x] Stack and refresh: same caster refreshes duration and adds a stack up to `CumulativeAura`; other casters coexist for harmful, periodic or DoT-stacking-rule auras, else replace.
- [x] Expire on duration (0/-1 permanent); remove on death (except passives and allow-while-dead), `CancelAura` (own positive non-passive cancelable auras), dispel (effect 38 by dispel type), channel end, charge/absorb depletion, unlearned passives and despawned casters.
- [x] Remove auras on the interrupt flags that have a system: hostile action received, damage, non-periodic damage, action (cast), moving, attacking, entering/leaving combat, spec change.
- [x] Tick periodic damage/heal/energize/trigger/dummy effects every period until expiry. Each deadline follows the original application/owning-area phase plus its native period ordinal; fractional-second periods must not accumulate drift or lose a tick at exact expiry across repeated lifetimes. Preserve refresh phase, ExtraInitialPeriod's application tick, area alignment, native periods/expiry and callback order.
- [x] Recuperate 1231411's healing aura 1231418 ends on the tick reaching full health; when already full, it ends on the first tick without a heal event. End any matching channel and emit no later heals. Other periodic heals retain their existing full-health behavior. World of Osso user decision (2026-10-05), not TrinityCore parity; casting at full health remains allowed because no applicable data/Retail rejection rule is established.
- [x] Proc PROC_TRIGGER_SPELL auras on melee swing, spell hit, periodic damage/heal, damage taken, kill/death and cast success, using `ProcTypeMask`, `ProcChance`/RPPM, `ProcCategoryRecovery`, `ProcCharges` and class-mask filters; a spell hit carries its direct damage (`TAKE_ANY_DAMAGE`); triggered spells proc only auras with `SPELL_ATTR3_CAN_PROC_FROM_PROCS` or when they have `SPELL_ATTR3_NOT_A_PROC`, never the aura that triggered them.
- [x] Break crowd control on damage (MOD_CONFUSE 5, MOD_FEAR 7, MOD_STUN 12, MOD_ROOT 26, TRANSFORM 56 of a spell with `ProcTypeMask`): the effect holds 10% of the target's max health, each damaging proc takes its health loss off, and the aura ends at 0 (TrinityCore `HandleBreakableCCAuraProc`).
- [x] TRANSFORM 56 shows a model of the MiscValue creature template on the unit's `ModelDisplay` (newest transform, harmful first) and restores the native display when the last one ends.
- [x] MOD_CONFUSE 5: stop the cast, block casts (`SPELL_ATTR5_ALLOW_WHILE_CONFUSED` exempt) and melee swings, clear the creature's shown target, and wander the creature within 2 yd of where it was confused instead of chasing.
- [x] Polymorphed creatures (`SPELL_SPECIFIC_MAGE_POLYMORPH`) regenerate a third of their max health every 2 s while engaged; creatures with `RegenHealth` do so whenever they are not engaged.
- [x] Apply ADD_FLAT/PCT_MODIFIER (class mask) and label modifiers (`SpellLabel`) to cast time, duration, cooldown, GCD, costs, range, points, coefficients, crit, damage/heal and proc/aura fields.
- [x] MOD_RECOVERY_RATE_BY_SPELL_LABEL 143 scales positive spell/category cooldowns and their running remaining time by `100/(100+max(amount,-99))` when either MiscValue label or nonzero MiscValueB label matches. Aura refresh does not compound unchanged rates; removal/expiry restores the remaining timer's rate. Charge recovery and ModTimeRate are separate contracts.
- [x] Apply `RequiredAuraSpellID` cost gating, charge-category auras (411/453/454), damage/heal multipliers, absorbs, school immunity, crit, spell haste, power regen and movement speed.
- [x] MECHANIC_IMMUNITY 77 uses MiscValue to reject matching spell-category or effect mechanics; applying immunity removes existing matching auras through normal removal hooks. Removing immunity restores susceptibility.
- [x] REDUCE_PUSHBACK 149 sums aura percentages (bounded to 0–100%) to reduce damage pushback for casts and channels; positive non-periodic damage causes at most two pushbacks. Fully protected casts consume no pushback count. Normal-cast remaining time is capped at that cast's original haste-adjusted duration; an early hit cannot add unelapsed progress. Keep the duration fixed and rewind elapsed progress no earlier than zero. Repeated early hits share that same cap; channel behaviour remains unchanged.
- [x] USE_NORMAL_MOVEMENT_SPEED 191 caps run, swim and flight at the strongest positive aura amount in yards/second before slows; roots/stuns still stop movement. Removing the aura restores uncapped speed.
- [x] Auto-apply passive known spells; remove them when unlearned.
- [x] Fold aura stat changes into `stats::StatModifiers` (applied once, in `CharacterStats`); keep combat multipliers in `AuraStatMods`.
- [x] Replicate `UnitAuras` on every player and NPC only when the aura set changes, with `remaining_ms` at send time; log `AuraApplied`/`AuraRefreshed`/`AuraRemoved`/`Dispel` to source and target owners.
- [x] Classify native MOD_MELEE_RANGED_HASTE_2 342 by the signed-stat positivity rule: negative base/per-level points are harmful, preserving prior passive/buff/debuff attribute precedence. Ordinary friendly Curse dispels must remove eligible native curses without adding spell-ID positivity overrides.
- [x] Track unhandled aura types (logged once per spell) so they still show in the UI.
- [x] Fail casts under stun, silence and pacify with `CastFailReason::Stunned`/`Silenced`/`Pacified`, and under confuse with `NotSupported` ("Can't do that while confused": shared-protocol has no `Confused` reason).
- [x] The first MOD_STUN 12 application stops the unit's current cast or channel (TC `Unit::SetControlled` → `CastStop`).
- [x] Apply talent passives (talent spells are in `KnownSpells`).
- [x] Guardian Spirit47788 and Ardent Defender31850 share one exclusive-World death-prevention commit for melee, direct and periodic damage. Guardian matches native overkill school/limit, removes its parent before real48153 healing from snapshotted native effect1, and is consumed once. Each hit completes rescue before the next; death and logged damage/absorption use the committed result. This does not establish generic aura316 completeness.
- [x] Melee and spell damage share school immunity and absorb capacity handling; immune swings log `Immune`, fully absorbed swings log `Absorb`, and partial absorbs log remaining damage plus absorbed amount. Depleted shields are removed.
- [x] MOD_DODGE_PERCENT 49 adds nondiminishing dodge percentage points to player avoidance; Evasion uses its unscaled catalog amount. MOD_PARRY_PERCENT 47 adds nondiminishing parry points with weapon/parry capability requirements; no spell-specific amounts are substituted.
- [x] MOD_ATTACKER_MELEE/RANGED/SPELL_HIT_CHANCE 184/185/186 modifies incoming melee/ranged/matching-school spell misses. MECHANIC_IMMUNITY 77 blocks matching incoming spell/effect mechanics. DEFLECT_SPELLS 287 deflects magic/ranged attacks from the front unless controlled; IGNORE_HIT_DIRECTION 288 removes that facing restriction. New-hit defenses do not cancel already-landed periodic damage; applicable bypass attributes remain respected.
- [x] MOD_PACIFY 25 and MOD_PACIFY_SILENCE 60 prevent auto-attacks as well as PACIFY-prevention abilities. Turtle 186265 applies catalog pacify child 205769 and removes it when the parent ends, including natural expiry. See [defensives audit](../defensives-audit.md) for catalog/proof boundaries.
- [x] Fortifying Brew 115203 applies buff 120954 with 20% damage reduction and a fixed health bonus computed from pre-buff maximum health and the cast's health percentage. MOD_INCREASE_HEALTH 34 adds that bonus to both current and maximum health and removes it without killing a living unit. See [defensives audit](../defensives-audit.md) for source and proof boundaries.
- [x] MOD_BLOCK_PERCENT 51: add to a player's block chance when it can block and holds a shield (StatSystem.cpp:496-501); creatures' block ignores it.
- [x] MOD_GLOBAL_COOLDOWN_BY_HASTE_REGEN 417: divide the GCD of a spell its class mask affects by 1 + haste, within 750-1500 ms (Spell.cpp:9220-9224).

- [x] Cold dormant creatures defer aura-state initialization and authored addon effects until activity resumes. A real player-grid wake applies the addon exactly once in that same fixed step; another sleep/wake does not reset or reapply it. Active spawns retain first-step application, health, clocks and population.
- [ ] Native CPU timings are diagnostic only; performance acceptance uses frozen matching release baseline/after artifacts in an allocated window.

## How it works

- [Auras](../wiki/systems/auras.md)

## Implementation inventory

- `crates/server/src/auras/mod.rs`: `AuraPlugin`, `ActiveAuras`, apply/refresh/remove, dispel, interrupts, passives, `UnitAuras` sync, `CancelAura`.
- `crates/server/src/auras/periodic.rs`: periodic ticks.
- `crates/server/src/auras/procs.rs`: `AuraEvent` queue, procs, event interrupts.
- `crates/server/src/auras/spell_mods.rs`: `SpellModOp`, modifier matching, charge use.
- `crates/server/src/auras/stat_mods.rs`: `StatModifiers`/`AuraStatMods` computation.
- `crates/server/src/auras/handlers.rs`: aura type registry.
- `crates/server/src/auras/scripts.rs`: `AuraScripts` hooks, including Turtle's catalog pacify child lifecycle.
- `crates/server/src/auras/unit_state.rs`: invisibility, unattackable, confuse and transform unit state.
- `crates/server/src/auras/tests.rs`: real-DB App tests, including an actual authored Cenarion15184 spawn/addon18950 and real player-grid wake.
- `crates/server/src/zones.rs`: authored creature spawning and activity-gated addon application.
- `crates/server/src/auras/polymorph_tests.rs`: Polymorph 118 on a kobold and a level-scaled Blackrock Spy.
