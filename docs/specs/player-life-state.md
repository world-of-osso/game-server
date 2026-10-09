# Replicated player life state

Visible players expose shared `DeathState` independently of their stand pose. Source: `crates/server/src/death{,_support}.rs` and `interest.rs`. [Implementation and TrinityCore a352b1fa audit](../wiki/systems/networking.md#player-life-state).

## What it must do

- [x] Another client observes Alive → Dead (unreleased corpse) → Ghost → Alive through lethal damage, release and corpse resurrection.
- [x] Ghost state is explicit, never inferred from health.
- [x] Interest combines existing map/distance/hidden rules with shared `ghost_visible`. Living observers never receive a Ghost-state frame; ghost observers see ghosts, and each player still sees itself.
- [x] Stationary observer/target life-state changes update visibility; resurrection restores nearby living observers' visibility.
- [ ] Spell/healer/dungeon resurrection and persisted-ghost restoration retain the same replicated-state projection; individual end-to-end fixtures are left to the lead gate.

## How it works

- [Changed death state and interest invalidation](../wiki/systems/networking.md#player-life-state).

## Implementation inventory

- `crates/server/src/auth_character.rs` — selected-player initial Alive component.
- `crates/server/src/death.rs`, `death_support.rs` — changed authoritative state projection, including restoration and existing handlers.
- `crates/server/src/interest.rs` — observer/target state invalidation and shared visibility policy.

## Tests asserting this spec

- `crates/server/src/networking_tests/ghoststate.rs` — real UDP transitions and production release/corpse handlers, no transient ghost frame for living observers.
- `crates/server/src/interest_tests.rs` — stationary observer death-phase changes.

## Known gaps (current cycle)

- [ ] Lead's wider integration gate; no live realm run was authorized.

## Out of scope

Owner snapshot/corpse persistence changes, new corpse entities, and TrinityCore group/corpse visibility exceptions. Existing shared policy governs this realm.
