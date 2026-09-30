# Authoritative spell casting

Server-side spell casting in `crates/server/src/spell_cast/`: validation, cast bars, costs, cooldowns, targets and effect resolution. It replaces the presentation-only cast lifecycle. See [spell casting](../wiki/systems/spell-casting.md).

## What it must do

- [x] Accept spell requests only for the authenticated in-world connection's player; a connection's stop request must not cancel another entity's cast.
- [x] Reject unknown spells, dead casters, active casts, school lockouts, GCD, cooldown, exhausted charges, moving casters (cast-time/channel spells), wrong target relation, out-of-range (min and max) targets and unaffordable costs, sending `CastFailed` to the owner.
- [x] Instant spells resolve in the receiving tick; cast-time spells resolve on completion; channels resolve at start and tick per aura period.
- [x] Spend every applicable `SpellPower` cost in raw DB2 units (`PowerCostPct` of base mana for mana, of the max pool otherwise); aura-gated rows only while the caster has the aura.
- [x] Start GCD (at cast start), cooldown, category cooldown and charges; send `SpellCooldownUpdate`/`SpellChargesUpdate` to the owner.
- [x] Movement input cancels casts that are not cast-while-moving.
- [x] Resolve effects 2, 6/35/65 (auras), 10, 30, 31, 38, 58, 64 (bounded depth), 68, 121, 136; log unknown effects once per spell and skip them.
- [x] Fail the cast on unsupported implicit targets of handled effects.
- [x] Emit `CombatLogEvent` (CastStart, CastSuccess, Damage, Heal, Energize, Interrupt, Death) to source and target owners.
- [x] Reject player casts of spells not in `KnownSpells` (`NotSupported`, "spell not known"); NPC casters exempt.
- [x] Maintain class power pools with DB2 regen/decay; replicate `UnitPowers` (player `UnitLevel` belongs to class progression; NPC `UnitLevel` from the template).
- [x] Replicate raw whole power, fractional thousandths, signed regen rate, and charged combo-point indices. Resend on whole value, max, rate, display order or charged-point changes, not fraction alone; report zero rate on the tick a pool reaches its limit.
- [x] Replicate indexed rune ready times and full recharge duration, including queued runes' wait. Resend on spend, completion, duration or rune-count changes, not countdown ticks.
- [x] Use contract `CastFailReason` variants directly; `NotSupported` names the cause in `detail`.

## How it works

- [Spell casting](../wiki/systems/spell-casting.md)

## Implementation inventory

- `crates/server/src/spell_cast/mod.rs`: receivers, validation, start/complete/cancel, `SpellHistory`, messages.
- `crates/server/src/spell_cast/targets.rs`: implicit targets, relations, range.
- `crates/server/src/spell_cast/effects.rs`: effect registry, damage/heal paths.
- `crates/server/src/spell_cast/powers.rs`: `PowerTypeTable`, `PowerPools`, regen, `UnitPowers` sync.
- `crates/server/src/spell_cast/runes.rs`: per-rune cooldowns and `UnitRunes` sync.
- `crates/server/src/networking.rs`: `Moving` input marker.
- `crates/server/src/combat.rs`: `enter_combat`, `mark_npc_dead_world`.
- `crates/server/src/spell_info/`: spell data including `spell_category` charges.

## Tests asserting this spec

- `crates/server/src/spell_cast/tests.rs` (real `world.db` spells): instant Mortal Strike cost/GCD/cooldown messages and repeat failures; Fireball cast time and combat log order; movement cancel; Kick interrupt and Fire lockout; OutOfRange `CastFailed`; Flash Heal overheal; Charge energize raw units and charges message; rage decay; bounded trigger chain; unsupported implicit target (`NotSupported`); unknown-spell rejection; warrior `UnitPowers`; Fireball scaling with computed Mage stats; percent mana cost from base mana; Flash Heal at a hostile kobold self-casts.
- `crates/server/src/zones.rs`: NPC `UnitLevel` and empty `UnitPowers`.
- `crates/server/src/spell_cast/class_resource_tests.rs`: fractional Essence and send cadence, charged-point addition/removal, form rates, same-tick prediction limits, rune spend/wait/completion, haste-scaled duration/rate changes and max-passive rune-count updates.

## Known gaps (current cycle)

- [x] Auras, aura-gated costs, procs, periodic effects: [auras spec](auras.md).
- [ ] Players have no gear or weapon stats (see [character stats](character-stats.md)); weapon hits use a placeholder.
- [ ] Target relations use faction reactions without reputation or PvP flags ([factions wiki](../wiki/systems/factions.md)). No facing check. NPCs do not cast and have no power pools.
- [ ] Combat log reaches only source/target owners, not nearby observers.
- [ ] End-to-end wire replication and client rendering are unverified here.

## Out of scope

- Learned-spell permissions (known-spells task), haste-scaled GCD, spell travel time, ground-targeted spells (target 87).
