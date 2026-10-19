# Server auras

Server-side aura runtime in `crates/server/src/auras/`: aura instances on active units, periodic effects, procs, spell and stat modifiers, removal and `UnitAuras` replication. See [auras](../wiki/systems/auras.md).

## What it must do

- [x] Apply aura effects (6, 35, 65) as one instance per target and resolution, with per-effect amounts from base points, SP/AP coefficients and the caster's spell modifiers.
- [x] Stack and refresh: same caster refreshes duration and adds a stack up to `CumulativeAura`; other casters coexist for harmful, periodic or DoT-stacking-rule auras, else replace.
- [x] Expire on duration (0/-1 permanent); remove on death (except passives and allow-while-dead), `CancelAura` (own positive non-passive cancelable auras), dispel (effect 38 by dispel type), channel cancellation/explicit forced end, charge/absorb depletion, unlearned passives and despawned casters.
- [x] Remove auras on the interrupt flags that have a system: hostile action received, damage, non-periodic damage, action (cast), moving, attacking, entering/leaving combat, spec change.
- [x] Tick periodic damage/heal/energize/trigger/dummy effects every period until expiry. Each deadline follows the original application/owning-area phase plus its native period ordinal; fractional-second periods must not accumulate drift or lose a tick at exact expiry across repeated lifetimes. Preserve refresh phase, ExtraInitialPeriod's application tick, area alignment, native periods/expiry and callback order.
- [ ] Successful natural channel completion must drain its final native ticks and clear cast state before the finish callback/damage, without cancel-style forced removal of recipient auras. Recipients retain their own expiry; cancellation/interruption still removes channel auras immediately and does not invoke the successful callback. Preserve empty-recipient channels and independent child-recipient lifetimes. No spell-specific snapshots, death delays or altered damage observers. [Source/lifecycle proof](../wiki/systems/spell-casting.md#successful-channel-lifecycle--2026-10-18).
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
