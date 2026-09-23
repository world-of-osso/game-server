# Server auras

Server-side aura runtime in `crates/server/src/auras/`: aura instances on every unit, periodic effects, procs, spell and stat modifiers, removal and `UnitAuras` replication. See [auras](../wiki/systems/auras.md).

## What it must do

- [x] Apply aura effects (6, 35, 65) as one instance per target and resolution, with per-effect amounts from base points, SP/AP coefficients and the caster's spell modifiers.
- [x] Stack and refresh: same caster refreshes duration and adds a stack up to `CumulativeAura`; other casters coexist for harmful, periodic or DoT-stacking-rule auras, else replace.
- [x] Expire on duration (0/-1 permanent); remove on death (except passives and allow-while-dead), `CancelAura` (own positive non-passive cancelable auras), dispel (effect 38 by dispel type), channel end, charge/absorb depletion, unlearned passives and despawned casters.
- [x] Remove auras on the interrupt flags that have a system: hostile action received, damage, non-periodic damage, action (cast), moving, attacking, entering/leaving combat, spec change.
- [x] Tick periodic damage/heal/energize/trigger/dummy effects every period until expiry.
- [x] Proc PROC_TRIGGER_SPELL auras on melee swing, spell hit, periodic damage/heal, damage taken, kill/death and cast success, using `ProcTypeMask`, `ProcChance`/RPPM, `ProcCategoryRecovery`, `ProcCharges` and class-mask filters; triggered spells never proc.
- [x] Apply ADD_FLAT/PCT_MODIFIER (class mask) and label modifiers (`SpellLabel`) to cast time, duration, cooldown, GCD, costs, range, points, coefficients, crit, damage/heal and proc/aura fields.
- [x] Apply `RequiredAuraSpellID` cost gating, charge-category auras (411/453/454), damage/heal multipliers, absorbs, school immunity, crit, spell haste, power regen and movement speed.
- [x] Auto-apply passive known spells; remove them when unlearned.
- [x] Expose `AuraStatMods` for the stat system.
- [x] Replicate `UnitAuras` on every player and NPC only when the aura set changes, with `remaining_ms` at send time; log `AuraApplied`/`AuraRefreshed`/`AuraRemoved`/`Dispel` to source and target owners.
- [x] Track unhandled aura types (logged once per spell) so they still show in the UI.
- [x] Fail casts under stun, silence and pacify with `CastFailReason::Stunned`/`Silenced`/`Pacified`.
- [ ] Talent passives: needs the trait system to add talent spells to `KnownSpells`.

## How it works

- [Auras](../wiki/systems/auras.md)

## Implementation inventory

- `crates/server/src/auras/mod.rs`: `AuraPlugin`, `ActiveAuras`, apply/refresh/remove, dispel, interrupts, passives, `UnitAuras` sync, `CancelAura`.
- `crates/server/src/auras/periodic.rs`: periodic ticks.
- `crates/server/src/auras/procs.rs`: `AuraEvent` queue, procs, event interrupts.
- `crates/server/src/auras/spell_mods.rs`: `SpellModOp`, modifier matching, charge use.
- `crates/server/src/auras/stat_mods.rs`: `AuraStatMods`.
- `crates/server/src/auras/handlers.rs`: aura type registry.
- `crates/server/src/auras/scripts.rs`: `AuraScripts` hooks (empty).
- `crates/server/src/auras/tests.rs`: real-DB App tests.
