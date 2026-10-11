# Raid entry and lockouts
Retail Mainline raid entry extends the existing instance manager and saved-instance protocol. Both skins use the same behavior. Sources: TC a352b1fa Player::GetDifficultyID, Map::PlayerCannotEnter and MiscHandler::HandleSetRaidDifficultyOpcode; local Retail UnitPopupSharedButtonMixins and RaidInfoFrameSocialView; DB2 12.1.0.69933.

## What it must do
- [x] Admit supported legacy raids solo; require a raid group for current-expansion raids.
- [x] Keep separate primary and legacy raid selections, persist them, and apply leader selection to group members. Only the leader changes grouped difficulty; no change inside an instance.
- [x] Use MapDifficulty and Difficulty legacy/default flags for the actual copy, without changing dungeon difficulty.
- [ ] Boss deaths save affected players through the existing lock manager; weekly reset Tuesday 15:00 UTC, extension and unextension preserve saved progress.
- [x] Retail difficulty menu and Raid Info saved-instance list work in both skins.

## How it works
- [Raid implementation and authentic data derivation](../wiki/systems/raid-entry.md).
- Existing [instances](instances.md) contract owns map copies and lock persistence.

## Implementation inventory
- `crates/server/src/maps/raid.rs`: persistent raid selections.
- `maps/instances.rs`, `maps/store.rs`, `maps/transfer.rs`: admission and DB2 difficulty resolution.
- `maps/locks.rs`, `scripted_ai/instance_script.rs`: existing lock/credit lifecycle.

## Tests asserting this spec
- `crates/server/src/maps/tests.rs`: authentic legacy raid copy and bind resume.

## Known gaps (current cycle)
- [ ] Multi-actor/shared-display/absent-source encounter completion remains underivable; broad boss-saving contract is not universally satisfied.
- [x] Source-covered Lucifron/Magmadar deaths save both real accounts, persist through private restart, and mapped killed bosses stay absent.
- [x] Exact reset boundary and extended-copy scheduling have behavioral tests; private weekly reset/reactivation/unextension have native proof.

## Out of scope
Encounter mechanics and raid rewards are separate slices.

## Authenticated encounter derivation

[Server wiki](../wiki/systems/raid-entry.md#authentic-death-credit) owns the derivation,106 accepted bindings/331 current gaps, and copy-only import procedure. No canonical world.db writes or fabricated credit.
