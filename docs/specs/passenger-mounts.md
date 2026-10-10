# Passenger mounts

Contract: [engine passenger-mount spec](../../../game-engine/docs/specs/passenger-mounts.md). Server implementation: `crates/server/src/vehicle.rs`; mount activation: `auras/mount.rs`; behavioural fixtures: `vehicle_tests.rs` and `networking_tests/receiver_rate.rs`.

Authority stays on the mount-summoning player. Boarding validates living group membership, the authoritative `InMap { map_id, instance_id }` map copy, range, available enter/exit non-control seats and local model attachment data. Boarding fixtures use `auth_character::spawn_player_entity`, the login spawn path, without synthetic player `Zone` components. Map/instance transfers eject passengers; terrain area changes within a map copy do not. Passenger movement uses MovementControl and driver-relative M2 bind-pose offsets; server does not simulate animated mount bones. Current verification and remaining client/live acceptance are tracked in the linked contract.
