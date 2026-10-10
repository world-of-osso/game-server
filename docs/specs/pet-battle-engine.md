# Pet battle turn engine

Pure deterministic server library in `crates/server/src/pet_battle/`, exported through `server::pet_battle`. [Mechanics, schema and source provenance](../wiki/systems/pet-battle-engine.md); [full property coverage](../pet-battle-effect-coverage.md).

## What it must do

- [x] Load the ten requested battle DB2 tables through its own strict SQLite loader, with test-only local-CASC CSV fixtures and no journal-code dependency/runtime CSV fallback.
- [x] Derive stats from species/breed states, DB2 quality and level; preserve float32 rounding boundaries and fail invalid inputs explicitly.
- [x] Model three pets per team and an active slot; ability/swap/pass/forfeit decisions; speed order and injected tie RNG; faint replacement without combat-round advancement.
- [x] Dispatch common damage/heal/state/aura properties, including accuracy, variance, critical/type modifiers, timed ticks/expiry, refresh, multi-turn continuation and cooldowns.
- [x] Trigger Beast, Flying, Critter, Magic, Mechanical, Undead, Humanoid, Aquatic and Dragonkin passives through BattlePetState names; use actual ability descriptions/effect values for numeric rules.
- [x] Reject unsupported properties, parameters, events and special aura states explicitly; test rejected actions preserve RNG.
- [x] Assert concrete real-ID behavior: Bite110, Burn113, Healing Wave123, Poison Fang152, Burrow159, Deep Breath169, Curse of Doom218, Grasp249, Wish273, Time Stop600 and Recovery726.
- [x] Publish104-ID row/implementation/test coverage, distinguishing static admission from behavioral proof.
- [ ] Prove exact damage rounding/type weak multiplier and control/event timing against captured Retail runtime or current client game tables.
- [ ] Extend property coverage beyond the current12 validated-dispatch handlers as primary runtime evidence establishes their semantics.

## How it works

- [Engine mechanics, schema contract and source boundaries](../wiki/systems/pet-battle-engine.md).
- [Property matrix, proof levels and reproduction](../pet-battle-effect-coverage.md).

## Implementation inventory

- `src/lib.rs`, `pet_battle/mod.rs` — standalone library exports, no server startup wiring.
- `pet_battle/catalog.rs` — own typed SQLite loader, raw unresolved-state retention and admission inventory.
- `pet_battle/stats.rs`, `model.rs` — stats and battle/team/pet/aura/action/event representation.
- `pet_battle/rules.rs`, `engine.rs`, `effects.rs` — opcode admission, deterministic round sequencing and effect execution.
- `pet_battle/tests.rs` — real CSV→SQLite fixture and23 behavioral/inventory tests.

## Tests asserting this spec

`crates/server/src/pet_battle/tests.rs`; native `agent-run petbattle cargo test --locked -p server pet_battle`. Set `PET_BATTLE_DB2` to authenticated export directory. Existing server master requires shared master75fe6a4; current sibling3facd18 and stale declared8c16175 each have unrelated compile incompatibilities. Exact isolated dependency override and successful all-target filtered command are in the coverage report.23/23 pass, format/check pass at code2f99296; no full suite or live runtime claim.

## Known gaps (current cycle)

- [ ] pets-server-fix/DB producer must import ten battle tables under the documented schema; world.db currently contains species metadata only.
- [ ] Independent verification could not run because Claude OAuth expired; no paid substitute used.
- [ ] Elemental negative-weather immunity has no executable weather path or behavioral proof yet.

## Out of scope

Networking, wild encounter spawning, UI, capture/XP, journal persistence, deployment, merges and UDP5000 operations belong to later steps. This engine is not wired into runtime battles while effect/parity coverage remains bounded.
