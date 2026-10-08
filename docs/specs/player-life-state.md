# Replicated player life state

- Every selected player exposes Alive, Dead (unreleased corpse), or Ghost to clients that can see it.
- Lethal damage, spirit release, all resurrection paths, and persisted ghost restoration update this state; health alone never identifies a ghost.
- Interest visibility combines existing distance/map/hidden-unit rules with shared `ghost_visible`. Alive observers lose Ghost targets; Ghost observers see Ghost targets. Each player still sees itself.
- A stationary observer or target changing life state updates visibility without needing movement. Resurrection restores visibility to nearby living observers.
- Owner death snapshots and corpse persistence remain unchanged. TrinityCore's group/corpse visibility exceptions are not new requirements; the existing shared policy governs.

Proof: targeted `ghoststate` server tests in `networking_tests/ghoststate.rs` (real UDP transitions and release/recovery handlers) and `interest_tests.rs` (stationary observer phase changes).
Implementation/source audit: [networking](../wiki/systems/networking.md#player-life-state).
