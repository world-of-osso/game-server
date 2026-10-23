# Spell summons and area triggers

Server effects 28 and 179 use authentic spell/world records. Source: `crates/server/src/spell_cast/`; [runtime](../wiki/systems/spell-casting.md#temporary-summons-and-spell-area-triggers).

## What it must do

- [x] SUMMON resolves MiscValue as creature and MiscValueB as SummonProperties, spawns at the caster/target destination, records caster ownership and expires at spell duration.
- [ ] Guardians share the existing pet owner/follow representation without occupying a persisted hunter stable slot.
- [x] CREATE_AREATRIGGER spawns once at its destination, uses spell radius/duration, applies a supplied trigger spell on enter or at its supplied period, excludes units outside its radius, and expires.
- [ ] Missing creature and SummonProperties are reported rather than invented.
- [x] Missing matrix trigger bindings do not acquire fictional actions; five available matrix area roots produce lifecycle-only entities.

## Source caster offsets41/43/48

- [x] Targets41/43/48 center on the caster, at source front-right/back-left/back angles. Use caster CalcRadius including source motion bonus and inherited B radius; only corner41/43 use source default3 when their own radius entry is absent. Back48 without radius uses zero before the combat-reach floor.
- [x] Preserve caster orientation and direct collision/ground authority. Area recipients share the final emitted destination; missing authority refuses, FORCE_DEST_LOCATION retains the named MAX_HEIGHT gap. Existing32 grounding-only behavior and generic spell-family modifiers remain separate.
- [ ] Actual75042/105229 and245946/1240088 physical learning/UseToy prove source1s cast,60s actor expiry, model/owner/map/current-critter identity, source zero item recovery/reuse and existing DismissPet removal. Do not invent a UseToy toggle.
- [x] Arena108735/155042 remains unavailable without its source-described gladiator battle; two neutral actors alone are insufficient. Other affected script/action gaps remain independent of selector support.

## Source target73 caster-radius destinations

- [x] Target73 uses the caster as destination reference, randomizes direction and retains authored CalcRadius Max rather than sampling target72's radial disk. TargetB selects its own radius; an absent B radius inherits A's entry and target identity. For unit casters, cap Radius+PerLevel×level at authoredMax. Preserve source forward/strafe/falling +2 range while not walking, unless NO_MOVEMENT_RADIUS_BONUS is authored. Clamp distance upward to the caster's sourced combat reach. Generic spell-family radius modifiers and native movement-ingress parity require separate qualification.
- [x] Destination-area recipients and emitted positions share one sample per processed effect group. Caster ground/orientation and direct collision authority apply; absent collision/ground refuses, and FORCE_DEST_LOCATION retains the named MAX_HEIGHT gap. No Detour parity claim.
- [x] Ruthers82467 requires actual learning/UseToy, source61667 model/owner/map/radius5, natural30s expiry and1800000ms recovery/reuse before new admission is qualified. Generic73 does not implement Wikky86594 forage/presents/Pandaria or Carp131814 kick interaction; preserve those named action gaps.

## Source target74 destinations

- [x] Source target74 centers its destination on the explicit object, independently of caster/current-critter selection. A missing explicit object or one in another map does not create a substituted destination at the selector boundary.
- [x] TargetA/TargetB choose their authored radius entry; absent B radius inherits A's entry and target identity (source38107 `[6,74]` therefore uses a fixed radius/random direction). TC `CalcRadius(nullptr)` uses RadiusMax and samples `(Max-Min)*sqrt(random)` without adding Min; a source nonzero Radius value must not replace RadiusMax.
- [x] Destination-area recipients and the emitted destination share one sampled center per processed effect group; selecting units and then independently re-sampling the destination is invalid (native86088 `[74,16]`).
- [x] Apply the existing direct collision and target-ground boundary, preserving target orientation. Missing collision/ground authority refuses explicitly; source FORCE_DEST_LOCATION retains its named MAX_HEIGHT geometry gap. This does not certify Detour path parity.
- [x] Cranky Crab166662 remains unavailable for its source-described battle-pet ride interaction until actual action binding/data are established; destination74 and a scenery actor alone do not implement that ride.

## How it works

- [Spell casting runtime](../wiki/systems/spell-casting.md#temporary-summons-and-spell-area-triggers).

## Implementation inventory

- `crates/server/src/spell_cast/summon.rs`: metadata loading, spawn, ownership and expiry.
- `crates/server/src/spell_cast/area_trigger.rs`: area lifetime, enter/periodic trigger execution.
- `crates/server/src/spell_cast/effects.rs`: once-per-cast dispatch.
- `crates/server/src/spell_cast/targets.rs`: destinations, summon selectors and paired sampled destination/recipients.
- `crates/server/src/spell_cast/targets/ordered_destination.rs`: source73/74/148 direct collision/ground authority,73 caster radius/motion and force-location gap.
- `crates/server/src/spell_cast/mod.rs`: processed effect-group cache preserves sampled73/74 destinations with their recipients.
- `crates/server/src/toy_interaction.rs`: exact source-described ride/action gaps remain unavailable independently of selector support.
- `scripts/db2.py`, `scripts/db2_schema.sql`: local SummonProperties extraction/import.

## Tests asserting this spec

- `crates/server/src/spell_cast/summon_at_tests.rs`: real-data cast/lifecycle tests.
- `crates/server/src/spell_cast/target74_tests.rs`: real70448 destination/radius/origin/map/authority boundaries and preserved Crab action gap.
- `crates/server/src/spell_cast/target73_tests.rs`: native caster-radius/movement/reach/ordered-area/authority cases and exact preserved Wikky/Carp action gaps.
- `crates/server/src/toys_target73_tests.rs`: real Ruthers learning/UseToy/source actor/natural30s expiry/30min recovery reuse and rejected-cast GCD-vs-item-recovery retry.

## Known gaps (current cycle)

- [ ] Six matrix summon roots have no creature template in current world data.
- [ ] Nine matrix area-trigger roots have no supplied trigger spell or server action binding; no spell IDs may be guessed from their names. Four roots additionally require unsupported selector 87. Generic effect execution is tested with authentic non-matrix Purging Breath 1287715 → 1287711.

## Out of scope

- Client changes, deployment, other effect types, arbitrary ground-point protocol support.
- Full guardian combat scripts, permanent pet persistence, vehicle/puppet control.
