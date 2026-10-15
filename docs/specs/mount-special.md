# Mounted special animation authority

The existing emote transport carries one MountSpecial action. [Client contract and source boundaries](../../../game-engine/docs/specs/mount-special.md) define stationary jump, mount/rider animation and supported visuals. TC a352b1fa `MiscHandler.cpp:1173-1180` identifies the sending player and relays the special animation to its observer set.

## What it must do

- [ ] Mount aura application publishes the exact Mount.db2 identity with the selected display; NPC/taxi mounts without a catalog row retain identity0.
- [ ] `/mountspecial` and `EmoteIntent::MountSpecial` identify only the authenticated, entered-world sender, requiring an entered-world mounted actor. The native client selects stationary versus moving jump; the relay does not reclassify input from a separately delivered movement stream.
- [ ] Deliver one action to the owner and current replicated observer set; hidden/foreign players receive none. Do not fabricate a social chat line or achievement TextEmote ID.

## How it works

- [Client MountSpecial contract](../../../game-engine/docs/specs/mount-special.md).
- [Shared wire contract](../../../shared-protocol/docs/mount-special.md).

## Implementation inventory

- `crates/server/src/auras/mount.rs` — authoritative Mount identity on the replicated component.
- `crates/server/src/messaging.rs` — unified emote request validation and actual NetworkVisibility audience.

## Tests asserting this spec

- `crates/server/src/auras/mount_tests.rs::mount_special_identity_comes_from_the_authoritative_mount_aura` — real Golden Gryphon cast publishes Mount129.
- `crates/server/src/messaging_mount_special_tests.rs` — native Lightyear HostClient request/event, actor identity, owner/observer audience, no chat/criteria and unmounted/not-entered refusal.

## Known gaps (current cycle)

- [ ] Baseline identity and dispatch REDs observed; GREEN and client native proofs pending.
- [ ] Client procedural/rope source gaps remain explicitly bounded by the linked client contract.

## Out of scope

Mount acquisition/persistence, unrelated social emotes, new flight physics and production deployment.
