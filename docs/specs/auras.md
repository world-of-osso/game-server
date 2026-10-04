# Server auras

Server-side aura runtime in `crates/server/src/auras/`: aura instances on every unit, periodic effects, procs, spell and stat modifiers, removal and `UnitAuras` replication. See [auras](../wiki/systems/auras.md).

## What it must do

- [x] Apply aura effects (6, 35, 65) as one instance per target and resolution, with per-effect amounts from base points, SP/AP coefficients and the caster's spell modifiers.
- [x] Stack and refresh: same caster refreshes duration and adds a stack up to `CumulativeAura`; other casters coexist for harmful, periodic or DoT-stacking-rule auras, else replace.
- [x] Expire on duration (0/-1 permanent); remove on death (except passives and allow-while-dead), `CancelAura` (own positive non-passive cancelable auras), dispel (effect 38 by dispel type), channel end, charge/absorb depletion, unlearned passives and despawned casters.
- [x] Remove auras on the interrupt flags that have a system: hostile action received, damage, non-periodic damage, action (cast), moving, attacking, entering/leaving combat, spec change.
- [x] Tick periodic damage/heal/energize/trigger/dummy effects every period until expiry.
- [x] Proc PROC_TRIGGER_SPELL auras on melee swing, spell hit, periodic damage/heal, damage taken, kill/death and cast success, using `ProcTypeMask`, `ProcChance`/RPPM, `ProcCategoryRecovery`, `ProcCharges` and class-mask filters; a spell hit carries its direct damage (`TAKE_ANY_DAMAGE`); triggered spells proc only auras with `SPELL_ATTR3_CAN_PROC_FROM_PROCS` or when they have `SPELL_ATTR3_NOT_A_PROC`, never the aura that triggered them.
- [x] Break crowd control on damage (MOD_CONFUSE 5, MOD_FEAR 7, MOD_STUN 12, MOD_ROOT 26, TRANSFORM 56 of a spell with `ProcTypeMask`): the effect holds 10% of the target's max health, each damaging proc takes its health loss off, and the aura ends at 0 (TrinityCore `HandleBreakableCCAuraProc`).
- [x] TRANSFORM 56 shows a model of the MiscValue creature template on the unit's `ModelDisplay` (newest transform, harmful first) and restores the native display when the last one ends.
- [x] MOD_CONFUSE 5: stop the cast, block casts (`SPELL_ATTR5_ALLOW_WHILE_CONFUSED` exempt) and melee swings, clear the creature's shown target, and wander the creature within 2 yd of where it was confused instead of chasing.
- [x] Polymorphed creatures (`SPELL_SPECIFIC_MAGE_POLYMORPH`) regenerate a third of their max health every 2 s while engaged; creatures with `RegenHealth` do so whenever they are not engaged.
- [x] Apply ADD_FLAT/PCT_MODIFIER (class mask) and label modifiers (`SpellLabel`) to cast time, duration, cooldown, GCD, costs, range, points, coefficients, crit, damage/heal and proc/aura fields.
- [x] Apply `RequiredAuraSpellID` cost gating, charge-category auras (411/453/454), damage/heal multipliers, absorbs, school immunity, crit, spell haste, power regen and movement speed.
- [x] Auto-apply passive known spells; remove them when unlearned.
- [x] Fold aura stat changes into `stats::StatModifiers` (applied once, in `CharacterStats`); keep combat multipliers in `AuraStatMods`.
- [x] Replicate `UnitAuras` on every player and NPC only when the aura set changes, with `remaining_ms` at send time; log `AuraApplied`/`AuraRefreshed`/`AuraRemoved`/`Dispel` to source and target owners.
- [x] Track unhandled aura types (logged once per spell) so they still show in the UI.
- [x] Fail casts under stun, silence and pacify with `CastFailReason::Stunned`/`Silenced`/`Pacified`, and under confuse with `NotSupported` ("Can't do that while confused": shared-protocol has no `Confused` reason).
- [x] Apply talent passives (talent spells are in `KnownSpells`).
- [x] MOD_BLOCK_PERCENT 51: add to a player's block chance when it can block and holds a shield (StatSystem.cpp:496-501); creatures' block ignores it.
- [x] MOD_GLOBAL_COOLDOWN_BY_HASTE_REGEN 417: divide the GCD of a spell its class mask affects by 1 + haste, within 750-1500 ms (Spell.cpp:9220-9224).

## How it works

- [Auras](../wiki/systems/auras.md)

## Implementation inventory

- `crates/server/src/auras/mod.rs`: `AuraPlugin`, `ActiveAuras`, apply/refresh/remove, dispel, interrupts, passives, `UnitAuras` sync, `CancelAura`.
- `crates/server/src/auras/periodic.rs`: periodic ticks.
- `crates/server/src/auras/procs.rs`: `AuraEvent` queue, procs, event interrupts.
- `crates/server/src/auras/spell_mods.rs`: `SpellModOp`, modifier matching, charge use.
- `crates/server/src/auras/stat_mods.rs`: `StatModifiers`/`AuraStatMods` computation.
- `crates/server/src/auras/handlers.rs`: aura type registry.
- `crates/server/src/auras/scripts.rs`: `AuraScripts` hooks (empty).
- `crates/server/src/auras/unit_state.rs`: invisibility, unattackable, confuse and transform unit state.
- `crates/server/src/auras/tests.rs`: real-DB App tests.
- `crates/server/src/auras/polymorph_tests.rs`: Polymorph 118 on a kobold and a level-scaled Blackrock Spy.
