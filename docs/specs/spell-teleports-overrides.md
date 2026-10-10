# Spell teleports and overrides

Data-driven server displacement and matrix aura overrides in `crates/server/src/spell_cast` and `auras`. [Implementation](../wiki/systems/spell-casting.md).

## What it must do

- [x] Blink 1953 moves 20 yards forward from target55's radius and snaps to ground.
- [x] Blink stops before real map0 wall geometry and bumps the movement teleport epoch.
- [ ] Hearthstone 8690 returns to a persisted homebind, including NewWorld/WorldPortAck across maps.
- [ ] Legacy teleport15 and Retail252 use authored spell_target_position rows for target17, never fabricated destinations.
- [ ] Matrix related leap effects move the caster, not the enemy target.
- [ ] Aura332 selects its data-specified replacement spell while active and restores the base after removal/expiry.
- [ ] Aura312 exposes its animation replacement set through aura replication; client consumption is a separate task.

## How it works

- [Spell casting](../wiki/systems/spell-casting.md)
- [Map transfer](../wiki/systems/zone-architecture.md)

## Implementation inventory

- `spell_cast/teleport.rs`: LOS displacement and destination validation.
- `spell_cast/teleport_positions.rs`: authored destination loader.
- `spell_cast/homebind.rs`: separate persisted homebind ABI and login initialization.
- `spell_cast/effects.rs`, `targets.rs`: dispatch and target selection.
- `auth_character.rs`, `persistence.rs`: homebind login/table lifecycle.

## Tests asserting this spec

- `crates/server/src/spell_cast/teleport_tests.rs`

## Known gaps (current cycle)

- [ ] Local world.db has no spell_target_position table/rows; import pinned Retail-authored destinations before claiming fixed-spell gameplay proof.
- [ ] Ground-point leap spells require protocol destination support (target87 is currently unsupported).

## Out of scope

Client animation/actionbar consumption, innkeeper UI, unrelated dummy spell scripts and full rotations.
