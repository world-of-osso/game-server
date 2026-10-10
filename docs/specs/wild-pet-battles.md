# Wild pet battles

Server-authoritative wild PvE encounters in `crates/server/src/wild_pet_battles.rs` use the [bounded turn engine](pet-battle-engine.md) and account journal.

## What it must do
- [ ] Start only for a living, nearby wild species on the player's map copy, with an owned three-slot loadout and exclusive encounter claim.
- [ ] Validate battle/round identity and admit supported ability, swap, pass, trap and forfeit decisions.
- [ ] Trap requires strictly below 35% enemy health, a living active ally and journal capacity; captured instances and participating-pet XP persist per account.
- [ ] Deliver final authoritative health/state before the terminal outcome, including defeated wins and captures.
- [ ] Release encounter claims and use the existing creature despawn/respawn lifecycle after wins.

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
- [ ] Final server workspace gate and final-build both-skin native win/capture evidence.

## Out of scope
PvP and unsupported inherited effect semantics; no direct-damage approximation for unsupported effects. No full Retail-parity certification.
