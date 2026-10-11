# Group loot

Authoritative grouped corpse distribution in `crates/server/src/loot.rs` and `loot/group.rs`, using shared roll/window primitives. Retail Mainline `GroupLootFrame.lua`, `LootHistory.lua`, `UnitPopupButtons.lua`, `LootConstantsDocumentation.lua`, local GlobalStrings.csv, and TrinityCore a352b1fa Loot.cpp ground the contract. Forever is an art skin only.

## What it must do
- [ ] Capture entitled group members at death within the same map copy and existing group reward distance; generate one common drop, not one shared item per tapper. Quest-required items remain personal.
- [ ] Start rolls at the leader's quality threshold (2–4), deliver Need > Transmog > Greed winner exactly once; prefer appropriate/equippable classes within Transmog (Blizzard Warbands Preview, article24061008). Auto-pass at60 seconds and expose all-pass items for one shared claim.
- [ ] Validate Need eligibility server-side. Preserve failed delivery on the winner's corpse loot rather than destroying it.
- [ ] Retain authoritative loot history for all eligible members, including replay after reconnect.
- [ ] One coin click splits a single money drop across eligible members (integer division, TrinityCore coin split); repeat clicks cannot pay twice.
- [ ] Only the leader changes Group Loot/Personal method or quality threshold; legacy policies are rejected. Below-threshold items rotate to one eligible member.

## How it works
- [Shared contract](../../../shared-protocol/docs/specs/group-loot.md)

## Implementation inventory
- `crates/server/src/loot/group.rs`: roll lifecycle, entitlement, shared pool, history and delivery.
- `crates/server/src/loot.rs`: generation and corpse-window/coin integration.
- `crates/server/src/group/registry.rs`: leader policy, threshold and common-item rotation.
- `crates/server/src/group/mod.rs`: policy messages.

## Tests asserting this spec
- `crates/server/src/loot/group_tests.rs`: deterministic Need/all-pass/timeout and shared coins; authentic Rageclaw Boots (15379, appearance 2478), Transmog 1 > Greed 99, account-known appearance eligibility, once-only delivery and completed history on a replaced connection; authentic leather class preference makes DruidTransmog1 beat WarriorTransmog99.

## Scoped proof — 2026-10-11

Two privateUDP54300 accounts prove actual winner inventory deltas, numeric results and history in Modern and Forever; party/raid leader method UI also observed. Deterministic cases cover Need/Transmog/Greed/Pass priority, all-pass sharing, timeout/decay, eligibility and reconnect history. Twelve existing personal-loot regressions and the leader method/threshold authority case pass. [Detailed consumer ledger](../../../game-engine/docs/wiki/systems/group-loot.md).

## Known gaps (current cycle)
- [ ] Full Mainline Need-policy certification (powerful-owned variants and complete proficiency/profession cases).
- [ ] Winner bag-full manual pickup history delivery-status refresh.
- [ ] Independent final verification; verifier backend authentication expired.

## Out of scope
- Master loot; legacy distribution policies; world database mutation.
