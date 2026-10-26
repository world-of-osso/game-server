# Zatha'tek, Breath of Corruption

Equipped weapon271093 follows pinned SimC76f8118 `unique_gear_midnight.cpp5233–5280`, with native70448 values. Source: `crates/server-spell-scripts/src/modules/zathatek_breath.rs`. See [weapon relationship](../wiki/systems/zathatek-weapon.md) for implementation details.

## What it must do

- [ ] Actual equipped1298023 procs deliver1305395 using its native item-level-scaled budget and native role expression. Retain generic native proc eligibility/rates and payload damage rules; do not proc before combat or recursively from its own action.
- [ ] Each impact applies or refreshes owned1305391 after the damage. Driver1298023 effect1 supplies the stack percentage; native1305391 supplies its cap and lifetime. Its item-scaled effect0 is not the percentage.
- [ ] Existing recipient stacks amplify only that caster's1305395 payload. A different recipient's stacks do not substitute; preserve ordinary damage/critical resolution.
- [ ] Gate both actual equipped profiles259/261 and the unequipped260 negative profile without changing their equipment. Notify the lead before landing for both equipped specs' remeasurement.

## How it works

- [Weapon relationship](../wiki/systems/zathatek-weapon.md).
- [DPS parity contract](dps-parity.md).

## Implementation inventory

- `crates/server-spell-scripts/src/modules/zathatek_breath.rs` — missing weapon relationship, native budget/role and owned recipient stacks.
- `crates/server-spell-scripts/src/modules.rs` — alphabetical module registration, preserving historical dispatch positions.
- `crates/server-combat/src/{item_proc_data.rs,auras/procs.rs,auras/mod.rs}` — existing native role, recipient and aura-application helpers exposed downstream, without behavior changes.

## Tests asserting this spec

- `crates/server-simulation/src/dungeon_simulator/dps_parity/zathatek_tests.rs` — real equipped swings and native budget, idle/unequipped refusal, owned Tear and instant native recipient-amplification controls.

## Known gaps (current cycle)

- [ ] Native RED259a2b9e604e6 and261b9b13e112f record265/240real swings without any payload.260negative ac13dc9bac passes. GREEN and final all-equipper measurements remain pending.

## Out of scope

- Ret16615's Loa/Rite implementation in `midnight_stat_items.rs` is separately owned and unchanged.
- Stat/coefficient substitutions and artificial equipment changes are prohibited by the parity contract.
