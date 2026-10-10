# Spell teleports and overrides

Data-driven server displacement and matrix aura overrides in `crates/server/src/spell_cast` and `auras`. [Implementation](../wiki/systems/spell-casting.md).

## What it must do

- [x] Blink 1953 moves 20 yards forward from target55's radius and snaps to ground.
- [x] Blink stops before real map0 wall geometry and bumps the movement teleport epoch.
- [x] Hearthstone 8690 returns to a persisted homebind, including NewWorld/WorldPortAck across maps.
- [x] New characters persist actual creation binds, preserved after logout-position changes and reopening redb.
- [ ] Existing characters without a stored bind must fail explicitly; no inferred migration location.
- [x] Target17 teleports consume authored spell_target_position rows, never infer destination coordinates from spell IDs. Matrix18960 fixture proves loader/routing, not Retail Moonglade coordinates.
- [x] Harpoon190925's child186260 and Feral Lunge196884 move the caster, not the enemy target. Infernal Strike189110 accepts a finite, in-range explicit ground point.
- [x] Aura332 selects its data-specified replacement spell while active and restores the base after removal/expiry.
- [x] Aura312 exposes its animation replacement set through aura replication; client consumption is a separate task.

## How it works

- [Spell casting](../wiki/systems/spell-casting.md)
- [Map transfer](../wiki/systems/zone-architecture.md)

## Implementation inventory

- `spell_cast/teleport.rs`: LOS displacement and destination validation.
- `spell_cast/teleport_positions.rs`: authored destination loader.
- `spell_cast/homebind.rs`: separate persisted homebind ABI and login initialization.
- `spell_cast/effects.rs`, `targets.rs`: dispatch and target selection.
- `auth_character.rs`, `persistence.rs`: homebind login/table lifecycle.
- `auras/overrides.rs`, `auras/mod.rs`: live spell substitution and aura wire values.
- `scripts/spell_target_position_schema.sql`: authored destination import schema.

## Tests asserting this spec

- `crates/server/src/spell_cast/teleport_tests.rs`
- `crates/server/src/maps/tests.rs`: Hearthstone cross-map handshake.
- `crates/server/src/auras/override_tests.rs`: substitution, removal and expiry.
- `crates/server/src/spell_cast/override_queue_tests.rs`: re-resolve on queued release.
- Companion `shared-protocol/src/protocol_spell_tests.rs`: aura overrides and ground-point wire round trips.

## Known gaps (current cycle)

- [ ] Local world.db has no spell_target_position table/rows; import pinned Retail-authored destinations before claiming fixed-spell gameplay proof.
- [ ] Client must consume AuraView.overrides and supply SpellCastIntent.destination for ground-point leaps. Companion shared-protocol branch required; existing clients need protocol literal initializers updated.

## Out of scope

Client animation/actionbar consumption, innkeeper UI, unrelated dummy spell scripts and full rotations.
