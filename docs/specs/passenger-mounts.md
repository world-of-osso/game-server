# Passenger mounts

Contract: [engine passenger-mount spec](../../../game-engine/docs/specs/passenger-mounts.md). Server implementation: `crates/server/src/vehicle.rs`; mount activation: `auras/mount.rs`; behavioural fixtures: `vehicle_tests.rs` and `networking_tests/receiver_rate.rs`.

Authority stays on the mount-summoning player. Boarding validates living group membership, map copy/zone, range, available enter/exit non-control seats and local model attachment data. Passenger movement uses MovementControl and driver-relative M2 bind-pose offsets; server does not simulate animated mount bones. Current verification and remaining client/live acceptance are tracked in the linked contract.
