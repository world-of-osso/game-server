# Combat assist effective costs

Owner-only effective spell costs for the [client combat assist](../../../game-engine/docs/specs/cooldown-assist.md). Normal cast admission remains authoritative; authored rotation ordering belongs to the client catalog, not this snapshot.

## What it must do
- [x] Supply effective nonzero power costs and affordability from ordinary cast admission. Tests cover300raw rage and real Purify base-mana130→65 with Mental Agility; native consumption is proved in private combat.
- [x] Replace owner readings when learned spells, effective amounts or affordability change; native cache replacement/error preservation and live learning/casting are proved.
- [x] Preserve unavailable metadata/caster facts as explicit errors, not free casts; wire and native cache tests retain errors.
- [x] Publish known charge categories when a persisted character's spellbook initializes/changes, after its passive auras apply and before any cast. Deduplicate shared categories; retain recovery/maximum updates without idle full-state notifications or changing consume/restore accounting. Full categories have zero remaining recovery time.

## How it works
- [Spell system](../wiki/systems/spell-system.md)

## Implementation inventory
- `crates/server/src/spell_cast/assisted_costs.rs`: owner readings and change publication.
- `crates/server/src/spell_cast/mod.rs`: cost calculation and publication scheduling.
- `shared::protocol::SpellCostSnapshot`: full replacement of effective readings.

## Tests asserting this spec
- `crates/server/src/spell_cast/assisted_costs.rs`: real Mortal Strike300raw rage admission and threshold updates.
- Shared protocol wire test retains effective numbers and explicit errors.
- `crates/server/src/spell_cast/charge_snapshot_tests.rs`: actual Fire Blast category1500 baseline1/1 owner spellbook snapshot, idle notification cadence, shared Charge category1386 with Double Time, and last-charge restoration clearing the recovery clock.
- Existing `auras::tests::charge_auras_add_a_charge_and_shorten_recovery` and skyriding tests retain consume/no-charge notification assertions unchanged.
- Semantics: TrinityCore `a352b1fa`, `src/server/game/Spells/SpellHistory.cpp`: `WritePacket(SendSpellCharges)`282–300, `ConsumeCharge`755–774, `ModifyChargeRecoveryTime`775–796, `RestoreCharge`831–840, `SendSetSpellCharges`995–1006. Initial serialization is distinct from queue mutations; restoration removes the newest consumed charge and an empty queue has zero next-recovery time.

## Known gaps (current cycle)
- [x] Modern/Forever private owner transport and recommendation consumption pass. [Client contract](../../../game-engine/docs/specs/cooldown-assist.md) owns unresolved rule and assistant-GCD semantics.

## Out of scope
Numeric rotation comparator guesses; this publisher does not evaluate authored rotation rules.
