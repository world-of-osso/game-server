# Combat assist effective costs

Owner-only effective spell costs for the [client combat assist](../../../game-engine/docs/specs/cooldown-assist.md). Normal cast admission remains authoritative; authored rotation ordering belongs to the client catalog, not this snapshot.

## What it must do
- [ ] Supply effective nonzero power costs and affordability from the same aura-gated, base-mana, modifier and script calculations used by ordinary casting.
- [ ] Replace the owner's readings when learned spells, effective amounts or affordability change.
- [ ] Preserve unavailable metadata/caster facts as explicit errors, not free casts.

## How it works
- [Spell system](../wiki/systems/spell-system.md)

## Implementation inventory
- `crates/server/src/spell_cast/assisted_costs.rs`: owner readings and change publication.
- `crates/server/src/spell_cast/mod.rs`: cost calculation and publication scheduling.
- `shared::protocol::SpellCostSnapshot`: full replacement of effective readings.

## Tests asserting this spec
- `crates/server/src/spell_cast/assisted_costs.rs`: real Mortal Strike300raw rage admission and threshold updates.
- Shared protocol wire test retains effective numbers and explicit errors.

## Known gaps (current cycle)
- [ ] Native owner transport/live recommendation consumption awaits private proof.

## Out of scope
Numeric rotation comparator guesses; this publisher does not evaluate authored rotation rules.
