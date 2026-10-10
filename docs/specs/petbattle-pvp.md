# PvP pet battles

Retail Find Battle queues two account-owned three-pet loadouts and runs them through the existing pet battle engine. Local Retail sources: Blizzard_Collections/Shared/Blizzard_PetCollection.lua and .xml; Blizzard_StaticPopup_Game/Mainline/StaticPopupSpecial.xml.

## What it must do

- [x] Join once, leave queue, reject duplicate queue entries.
- [x] Propose compatible slotted teams; decline requeues the other player; reject stale acceptance.
- [x] Wait for both player decisions, preserve each player's ally/enemy orientation, reject stale rounds and duplicate decisions.
- [x] Resolve engine victory/defeat, forfeit and disconnected opponents without trapping PvP pets.
- [ ] Native Find Battle/Leave Queue and Accept/Decline use the same Retail elements in both skins.

## How it works

- [Existing battle engine](../pet-battle-effect-coverage.md).

## Implementation inventory

- `crates/server/src/pvp_pet_battles.rs`: matchmaking and PvP runtime.
- `crates/server/src/wild_pet_battles.rs`: shared catalog, snapshots and transport.

## Tests asserting this spec

- `crates/server/src/pvp_pet_battles_tests.rs`.
- Sibling protocol `tests/petbattle_pvp.rs`.

## Known gaps (current cycle)

- [ ] Initial active-pet selection, proposal/round deadlines and PvP XP rewards remain unimplemented.
- [ ] Draw delivery has no dedicated production-boundary assertion.
- [ ] Native controls and private both-skin proof.
- [ ] Exact Retail rating/level tolerance, countdown/penalty and reward policy require source evidence; exact equal level profiles currently match.

## Out of scope

- Unsupported engine effects remain rejected; no fabricated data or world.db mutation.
