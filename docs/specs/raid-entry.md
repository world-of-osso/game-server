# Raid entry and lockouts
Retail Mainline raid entry extends the existing instance manager and saved-instance protocol. Both skins use the same behavior. Sources: TC a352b1fa Player::GetDifficultyID, Map::PlayerCannotEnter and MiscHandler::HandleSetRaidDifficultyOpcode; local Retail UnitPopupSharedButtonMixins and RaidInfoFrameSocialView; DB2 12.1.0.69933.

## What it must do
- [ ] Admit supported legacy raids solo; require a raid group for current-expansion raids.
- [ ] Keep separate primary and legacy raid selections, persist them, and apply leader selection to group members. Only the leader changes grouped difficulty; no change inside an instance.
- [ ] Use MapDifficulty and Difficulty legacy/default flags for the actual copy, without changing dungeon difficulty.
- [ ] Boss deaths save affected players through the existing lock manager; weekly reset Tuesday 15:00 UTC, extension and unextension preserve saved progress.
- [ ] Retail difficulty menu and Raid Info saved-instance list work in both skins.

## How it works
- Existing [instances](instances.md) contract owns map copies and lock persistence.

## Implementation inventory
- `crates/server/src/maps/raid.rs`: persistent raid selections.
- `maps/instances.rs`, `maps/store.rs`, `maps/transfer.rs`: admission and DB2 difficulty resolution.
- `maps/locks.rs`, `scripted_ai/instance_script.rs`: existing lock/credit lifecycle.

## Tests asserting this spec
- `crates/server/src/maps/tests.rs`: authentic legacy raid copy and bind resume.

## Known gaps (current cycle)
- [ ] Raid boss-to-encounter script bindings, client UI and private both-skin proof remain pending.

## Out of scope
Encounter mechanics and raid rewards are separate slices.
