# Headless combat dependency boundary

`server-combat` owns shared combat gameplay. `server-simulation` retains its simulator and DPS tests and consumes combat without the native server. See [crate boundaries](../wiki/systems/server-crates.md).

## What it must do

- [ ] Simulation depends on combat, not `server`.
- [ ] Simulation normal/dev dependencies exclude Lightyear, Replicon, Bevy asset/camera/image/mesh/window crates, wgpu-types, image, image-blp and png.
- [ ] Native game behavior, payloads and existing simulation test filters remain unchanged.
- [x] Outgoing queues preserve payload order, channel identity and priority.
- [x] Receiving drains payloads; frame cleanup discards unread messages.

## How it works

- [Server crate boundaries](../wiki/systems/server-crates.md).

## Implementation inventory

- `crates/server-combat/src/`: moved combat modules and fixtures.
- `crates/server-runtime/src/messaging.rs`: transport-free typed queues.
- `crates/server-simulation/src/`: unchanged simulator/DPS test ownership.
- `crates/server/src/`: native assembly, transport and persistence adapters.

## Tests asserting this spec

- `crates/server-runtime/src/messaging/tests.rs`: queue ordering, channels, priorities, draining and clearing.
- Existing moved combat tests and `server-simulation` tests: gameplay behavior.
- `cargo tree -p server-simulation -e normal,dev`: dependency boundary.

## Known gaps (current cycle)

- [ ] Finish native adapter and crate wiring.
- [ ] Pass workspace/all-targets check, formatting and moved tests with zero warnings.
- [ ] Report source/dependency counts and the warmed opt2 simulation build after a spell_cast edit.

## Out of scope

- Gameplay fixes, protocol changes, new agents and generated forwarding tests.
