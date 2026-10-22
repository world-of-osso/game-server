# Spell summons and area triggers

Server effects 28 and 179 use authentic spell/world records. Source: `crates/server/src/spell_cast/`; [runtime](../wiki/systems/spell-casting.md#temporary-summons-and-spell-area-triggers).

## What it must do

- [x] SUMMON resolves MiscValue as creature and MiscValueB as SummonProperties, spawns at the caster/target destination, records caster ownership and expires at spell duration.
- [ ] Guardians share the existing pet owner/follow representation without occupying a persisted hunter stable slot.
- [x] CREATE_AREATRIGGER spawns once at its destination, uses spell radius/duration, applies a supplied trigger spell on enter or at its supplied period, excludes units outside its radius, and expires.
- [ ] Missing creature and SummonProperties are reported rather than invented.
- [x] Missing matrix trigger bindings do not acquire fictional actions; five available matrix area roots produce lifecycle-only entities.

## Source target74 destinations

- [ ] Source target74 centers its destination on the explicit object, independently of caster/current-critter selection. A missing explicit object or one in another map does not create a substituted destination at the selector boundary.
- [ ] TargetA/TargetB choose their authored radius entry; absent B radius inherits A's entry and target identity (source38107 `[6,74]` therefore uses a fixed radius/random direction). TC `CalcRadius(nullptr)` uses RadiusMax and samples `(Max-Min)*sqrt(random)` without adding Min; a source nonzero Radius value must not replace RadiusMax.
- [ ] Destination-area recipients and the emitted destination share one sampled center per processed effect group; selecting units and then independently re-sampling the destination is invalid (native86088 `[74,16]`).
- [ ] Apply the existing direct collision and target-ground boundary, preserving target orientation. Missing collision/ground authority refuses explicitly; source FORCE_DEST_LOCATION retains its named MAX_HEIGHT geometry gap. This does not certify Detour path parity.
- [ ] Cranky Crab166662 remains unavailable for its source-described battle-pet ride interaction until actual action binding/data are established; destination74 and a scenery actor alone do not implement that ride.

## How it works

- [Spell casting runtime](../wiki/systems/spell-casting.md#temporary-summons-and-spell-area-triggers).

## Implementation inventory

- `crates/server/src/spell_cast/summon.rs`: metadata loading, spawn, ownership and expiry.
- `crates/server/src/spell_cast/area_trigger.rs`: area lifetime, enter/periodic trigger execution.
- `crates/server/src/spell_cast/effects.rs`: once-per-cast dispatch.
- `crates/server/src/spell_cast/targets.rs`: destinations and summon selectors.
- `crates/server/src/toy_interaction.rs`: exact source-described ride/action gaps remain unavailable independently of selector support.
- `scripts/db2.py`, `scripts/db2_schema.sql`: local SummonProperties extraction/import.

## Tests asserting this spec

- `crates/server/src/spell_cast/summon_at_tests.rs`: real-data cast/lifecycle tests.
- `crates/server/src/spell_cast/target74_tests.rs`: real70448 destination/radius/origin/map/authority boundaries and preserved Crab action gap.

## Known gaps (current cycle)

- [ ] Six matrix summon roots have no creature template in current world data.
- [ ] Nine matrix area-trigger roots have no supplied trigger spell or server action binding; no spell IDs may be guessed from their names. Four roots additionally require unsupported selector 87. Generic effect execution is tested with authentic non-matrix Purging Breath 1287715 → 1287711.

## Out of scope

- Client changes, deployment, other effect types, arbitrary ground-point protocol support.
- Full guardian combat scripts, permanent pet persistence, vehicle/puppet control.
