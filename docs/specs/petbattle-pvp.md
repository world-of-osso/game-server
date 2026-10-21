# PvP pet battles

Retail Find Battle queues two account-owned three-pet loadouts and runs them through the existing pet battle engine. Local Retail sources: Blizzard_Collections/Shared/Blizzard_PetCollection.lua and .xml; Blizzard_StaticPopup_Game/Mainline/StaticPopupSpecial.xml.

## What it must do

- [x] Join once, leave queue, reject duplicate queue entries.
- [x] Propose compatible slotted teams; decline requeues the other player; reject stale acceptance.
- [x] Wait for both player decisions, preserve each player's ally/enemy orientation, reject stale rounds and duplicate decisions.
- [x] Resolve engine victory/defeat, forfeit and disconnected opponents without trapping PvP pets.
- [x] Native Find Battle/Leave Queue and Accept/Decline use the same Retail elements in both skins; [client acceptance](../../../game-engine/docs/specs/petbattle-pvp.md#acceptance--2026-10-10-petbattle-pvp2).

## How it works

- [Existing battle engine](../pet-battle-effect-coverage.md).
- [Explicit PvP project policies](../wiki/systems/petbattle-pvp.md).

## Implementation inventory

- `crates/server/src/pvp_pet_battles.rs`: matchmaking and PvP runtime.
- `crates/server/src/wild_pet_battles.rs`: shared catalog, snapshots and transport.

## Tests asserting this spec

- `crates/server/src/pvp_pet_battles_tests.rs`.
- Sibling protocol `tests/petbattle_pvp.rs`.

## Known gaps (current cycle)

- [x] Both owners select a living initial front pet (including the current pet); combat waits for both choices without spending a round or applying swap effects. Duplicate choices and pre-selection combat actions are rejected.
- [ ] Explicit project policy PET_BATTLE_PVP_TIMERS enforces proposal/initial/turn/replacement deadlines with the documented timeout actions; exact-boundary and stale-request behavioral proof required.
- [ ] Explicit project policy PET_BATTLE_PVP_XP persists winner-only XP for living participants, using existing wild formula against opponent levels.
- [ ] Draw delivery has no dedicated production-boundary assertion.
- [x] Native controls and private both-skin actual three-pet defeat/win proof on UDP5598; no production port or world.db writes.
- [ ] Explicit project policy PET_BATTLE_PVP_MATCHMAKING pairs FIFO average-level teams with widening tolerance. Exact Retail rating/timer/reward parity remains uncertified; named policies are not source discoveries.

## Out of scope

- Unsupported engine effects remain rejected; no fabricated data or world.db mutation.
