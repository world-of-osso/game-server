# Group countdown

Server-authoritative Retail Mainline player countdowns; native parity is owned by the [engine group contract](../../../game-engine/docs/specs/group-frames.md).

## What it must do

- [ ] The group leader can start or replace a countdown using its selected duration, or cancel with zero.
- [ ] Every online group member, including the initiator, receives the same initiator/duration/remaining snapshot. Outsiders receive none.
- [ ] Ordinary members and solo players cannot start a group countdown.
- [ ] The server clock expires the countdown once and broadcasts zero remaining with the original positive duration. Cancellation instead has zero original duration.

## How it works

- [Shared wire contract](../../../shared-protocol/docs/specs/group-countdown.md).

## Implementation inventory

- `crates/server/src/group/mod.rs` — authenticated request draining and group-channel delivery.
- `crates/server/src/group/registry.rs` — permission checks, current countdown and deterministic clock.

## Tests asserting this spec

- `crates/server/src/group/tests.rs` — three-member host-client start/replace/cancel, outsider/ordinary-member rejection, server-clock expiry.

## Known gaps (current cycle)

- [ ] Assistant permissions, native UI/audio/manager control and live multi-account acceptance.
- [ ] Retail maximum-duration constant's numeric value is absent from available Lua; no guessed limit.
- [ ] Existing ready-check AFK summary and recipient notification.

## Out of scope

No world/reference database mutation or group persistence changes.
