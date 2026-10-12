# Combat assist effective costs

Owner-only effective spell costs for the [client combat assist](../../../game-engine/docs/specs/cooldown-assist.md). Normal cast admission remains authoritative; authored rotation ordering belongs to the client catalog, not this snapshot.

## What it must do
- [x] Supply effective nonzero power costs and affordability from ordinary cast admission. Tests cover300raw rage and real Purify base-mana130→65 with Mental Agility; native consumption is proved in private combat.
- [x] Replace owner readings when learned spells, effective amounts or affordability change; native cache replacement/error preservation and live learning/casting are proved.
- [x] Preserve unavailable metadata/caster facts as explicit errors, not free casts; wire and native cache tests retain errors.
- [x] Publish known charge categories before any cast and retain recovery/maximum updates. First-frame real owner delivery and private Crusader Strike2→1 with positive recharge are proved.

## How it works
- [Spell system](../wiki/systems/spell-system.md)

## Implementation inventory
- `crates/server/src/spell_cast/assisted_costs.rs`: owner readings and change publication.
- `crates/server/src/spell_cast/mod.rs`: cost calculation and publication scheduling.
- `shared::protocol::SpellCostSnapshot`: full replacement of effective readings.

## Tests asserting this spec
- `crates/server/src/spell_cast/assisted_costs.rs`: real Mortal Strike300raw rage admission and threshold updates.
- Shared protocol wire test retains effective numbers and explicit errors.
- `crates/server/src/spell_cast/charge_snapshot_tests.rs`: actual Fire Blast category1500 authored baseline1/1 (no extra-charge talent) owner message before casting.

## Known gaps (current cycle)
- [x] Modern/Forever private owner transport and recommendation consumption pass. [Client contract](../../../game-engine/docs/specs/cooldown-assist.md) owns unresolved rule and assistant-GCD semantics.

## Out of scope
Numeric rotation comparator guesses; this publisher does not evaluate authored rotation rules.
