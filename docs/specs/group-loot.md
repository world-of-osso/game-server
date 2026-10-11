# Group loot

Authoritative grouped corpse distribution in `crates/server/src/loot.rs` and `loot/group.rs`, using shared roll/window primitives. Retail Mainline `GroupLootFrame.lua`, `LootHistory.lua`, `UnitPopupButtons.lua`, `LootConstantsDocumentation.lua`, local GlobalStrings.csv, and TrinityCore a352b1fa Loot.cpp ground the contract. Forever is an art skin only.

## What it must do
- [ ] Capture entitled group members at death within the same map copy and existing group reward distance; generate one common drop, not one shared item per tapper. Quest-required items remain personal.
- [ ] Start rolls at the leader's quality threshold (2–4), deliver Need > Transmog > Greed winner exactly once, auto-pass at 60 seconds, and expose all-pass items for one shared claim.
- [ ] Validate Need eligibility server-side. Preserve failed delivery on the winner's corpse loot rather than destroying it.
- [ ] Retain authoritative loot history for all eligible members, including replay after reconnect.
- [ ] One coin click splits a single money drop across eligible members (integer division, TrinityCore coin split); repeat clicks cannot pay twice.
- [ ] Only the leader changes Group Loot/Personal method or quality threshold; legacy policies are rejected. Below-threshold items rotate to one eligible member.

## How it works
- [Shared contract](../../../../shared-protocol/docs/specs/group-loot.md)

## Implementation inventory
- `crates/server/src/loot/group.rs`: roll lifecycle, entitlement, shared pool, history and delivery.
- `crates/server/src/loot.rs`: generation and corpse-window/coin integration.
- `crates/server/src/group/registry.rs`: leader policy, threshold and common-item rotation.
- `crates/server/src/group/mod.rs`: policy messages.

## Tests asserting this spec
- `crates/server/src/loot/group_tests.rs`

## Known gaps (current cycle)
- [ ] Complete Need-policy parity (powerful-owned variant, proficiency/profession restrictions) and live proof.
- [ ] Reconnect/history and timeout integration gates.
- [ ] Winner bag-full manual pickup history status.

## Out of scope
- Master loot; legacy distribution policies; world database mutation.
