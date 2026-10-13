# Mythic Mentor party keystones

Server-authoritative party key summaries for the native [Keys panel](../../../game-engine/docs/specs/mythic-mentor.md). Source: installed `BossHelper/Modules/KeystoneTracker.lua:33-98`, `UI/KeystoneWidget.lua:501-600`. [Runtime](../wiki/systems/mythic-plus.md#party-key-sharing).

## What it must do

- [x] Authenticated in-world clients receive complete recipient-specific `PartyKeystoneSnapshot` replacements on reliable ordered InstanceChannel when data changes (one-second refresh); no addon messaging.
- [ ] Existing native post-InWorld `ChallengeRequest::Query` also returns the complete party list even when the change cache matches. This is the readiness handshake, not retry polling.
- [x] Recipient first, then party roster; solo receives self, raids no rows, outsiders never disclosed. Include members without keys.
- [x] Join persisted bag item IDs/GUIDs to persisted keystone records, labelled by imported MapChallengeMode summaries from the existing [Mythic+ catalog](mythic-plus.md), independently of full run qualification. Records without matching bags are not keys. Never disclose peer item GUIDs.
- [ ] Include every imported challenge's authored dungeon/time for real owned keys; current installed-addon eight-map regression plus Azure401 must pass. Catalog-only administrative issuance does not enable unsupported challenge runs.
- [x] Include roster identity/name/class/leader, challenge/level/authored dungeon/time. Runtime NPC companions use roster identities, not stored characters, and have no bag keys.
- [x] Roster/leader/bag changes, depletion/upgrades, logout and character switching replace earlier state. Client cannot request arbitrary characters' keys.

## How it works

- [Runtime and party-key integration](../wiki/systems/mythic-plus.md#party-key-sharing).
- [Both-skin private5811 proof](../../../game-engine/docs/wiki/systems/mythic-mentor.md).

## Implementation inventory

- `crates/server/src/mythic_mentor.rs` — persisted bag/roster projection and changed snapshot delivery.
- `crates/server/src/mythic_plus.rs` — existing key storage and plugin wiring.
- `crates/server/src/group/registry.rs` — authoritative human/companion roster identities.
- `shared-protocol/src/protocol/challenge_messages.rs` (sibling repository) — summary schema and registration.

## Tests asserting this spec

- `crates/server/src/mythic_mentor.rs` — real persisted bags and GroupRegistry: recipient/outsider, bag removal, leader, raid, leave and four-companion party.
- Client `godot/tests/mythic_mentor_live.gd` plus real UDP peer — private two-account8/11 receive and open-panel uninvite/leave pruning, both skins.
- Client `godot/rust/src/account_mythic_mentor_tests.rs` — logout transport-stop clears earlier rows.

## Known gaps (current cycle)

Full playable run qualification remains Azure401. Imported display summaries are independent of that gameplay boundary; no active realm season is inferred from historical DB2 rows. Eight installed-addon teleport spells have no local TDB target-position rows.

## Out of scope

Addon messages, peer bag GUID disclosure, invented seasonal/dungeon acquisition policy, and unrelated party data.
