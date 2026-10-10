# Wild pet battles

Server-authoritative wild PvE encounters in `crates/server/src/wild_pet_battles.rs` use the [bounded turn engine](pet-battle-engine.md) and account journal.

## What it must do
- [ ] Start only for a living, nearby wild species on the player's map copy, with an owned three-slot loadout and exclusive encounter claim.
- [ ] Validate battle/round identity and admit supported ability, swap, pass, trap and forfeit decisions.
- [x] Trap requires strictly below 35% enemy health, a living active ally and journal capacity; captured instances and participating-pet XP persist per account.
- [x] Deliver final authoritative health/state before the terminal outcome, including defeated wins and captures.
- [x] Release encounter claims and use the existing creature despawn/respawn lifecycle after wins.

- [x] Supply authoritative species portrait/quality/XP, ability family, aura icon and per-event target/amount feedback for the client HUD; terminal rewards include the new level XP threshold. No client-side combat inference.

## How it works
- [Turn catalog and semantics](../wiki/systems/pet-battle-engine.md).
- [Native client contract](../../../game-engine/docs/specs/wild-pet-battles.md).

## Implementation inventory
- `crates/server/src/wild_pet_battles.rs` — admission, decisions, transport and rewards.
- `crates/server/src/battle_pets.rs` — account journal and loadout persistence.
- `scripts/import-pet-battle-tables.py` — authenticated local-export table producer.

## Tests asserting this spec
- `crates/server/src/wild_pet_battles_tests.rs` — real Rabbit, replay, swap, strict trap/capacity, forfeit, XP and Lightyear-delivered terminal state with disk reopen/account isolation.
- Native private proof ledger: `/home/osso/.worktrees/handoff-petbattle-wild.md`.

## Known gaps (current cycle)
- [x] Final server workspace gate: `55b823a`,1,753 passed/71 ignored/one requested skip, exit0.
- [x] Final-build both-skin native win/capture evidence: private UDP5592, account fb_pbwild_proof, each skin defeats Rabbit then captures it, enemy0HP on defeated ends, captured instances10/11 and account XP preserved. [Client acceptance](../../../game-engine/docs/specs/wild-pet-battles.md#acceptance--2026-10-10) owns screenshots and exact proof.

`terminal-transport-red-100-percent.log` reproduces147HP delivered instead of1HP on guaranteed capture;6 wild tests pass after sending terminal state before End. Live defeated-win RED14HP then GREEN0HP separately validates the lethal-round boundary. Independent verifier OAuth remains unavailable.

## Out of scope
PvP and unsupported inherited effect semantics; no direct-damage approximation for unsupported effects. No full Retail-parity certification.
