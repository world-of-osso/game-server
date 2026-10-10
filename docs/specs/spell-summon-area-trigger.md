# Spell summons and area triggers

Server effects 28 and 179 use authentic spell/world records. Source: `crates/server/src/spell_cast/`; [runtime](../wiki/systems/spell-casting.md#temporary-summons-and-spell-area-triggers).

## What it must do

- [ ] SUMMON resolves MiscValue as creature and MiscValueB as SummonProperties, spawns at the caster/target destination, records caster ownership and expires at spell duration.
- [ ] Guardians share the existing pet owner/follow representation without occupying a persisted hunter stable slot.
- [ ] CREATE_AREATRIGGER spawns once at its destination, uses spell radius/duration, applies a supplied trigger spell on enter or at its supplied period, excludes units outside its radius, and expires.
- [ ] Missing creature, SummonProperties or trigger metadata is reported rather than invented.

## How it works

- [Spell casting runtime](../wiki/systems/spell-casting.md#temporary-summons-and-spell-area-triggers).

## Implementation inventory

- `crates/server/src/spell_cast/summon.rs`: metadata loading, spawn, ownership and expiry.
- `crates/server/src/spell_cast/effects.rs`: once-per-cast dispatch.
- `crates/server/src/spell_cast/targets.rs`: destinations and summon selectors.
- `scripts/db2.py`, `scripts/db2_schema.sql`: local SummonProperties extraction/import.

## Tests asserting this spec

- `crates/server/src/spell_cast/summon_at_tests.rs`: real-data cast/lifecycle tests.

## Known gaps (current cycle)

- [ ] Six matrix summon roots have no creature template in current world data.
- [ ] Nine matrix area-trigger roots have no supplied trigger spell or server action binding; no spell IDs may be guessed from their names.

## Out of scope

- Client changes, deployment, other effect types, arbitrary ground-point protocol support.
- Full guardian combat scripts, permanent pet persistence, vehicle/puppet control.
