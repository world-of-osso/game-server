# Authoritative cast presentation

Server-side overhead cast state originates in `crates/server/src/cast_presentation.rs`. This is presentation timing, not spell-effect execution. See [networking](../wiki/systems/networking.md#cast-presentation).

## What it must do

- [x] Accept valid timed spell requests only for the authenticated in-world connection's player.
- [x] Advance active casts and remove them on expiry, explicit stop, or disallowed movement.
- [x] Reject unknown spells, dead/moving casters, and requests without a player owner.
- [x] Preserve the server-resolved spell name through serialization.
- [x] Apply existing resource, cooldown and GCD validation.
- [ ] Apply existing target and range validation (implemented; receiver-boundary proof pending).
- [x] A connection's stop request must not cancel another entity's cast.
- [x] Expiry must not pretend to apply damage/healing.

## How it works

- [Networking: cast presentation](../wiki/systems/networking.md#cast-presentation)

## Implementation inventory

- `crates/server/src/cast_presentation.rs`: receiver ownership, validation, start/stop/tick.
- `crates/server/src/combat.rs`: schedule registration.
- `crates/server/src/networking.rs`: exposes movement ordering boundary.
- `crates/server/src/spell_db.rs`: explicit ID or unambiguous exact-name resolution.
- `../shared-protocol/src/casting.rs`: serializable named `CastState`.
- `../shared-protocol/src/protocol.rs`: component replication registration.

## Tests asserting this spec

- `crates/server/src/cast_presentation_tests.rs`: HostClient sender/receiver boundary, progress/expiry, stop/movement, rejection, ownership, validation, serialization.

## Known gaps (current cycle)

- [ ] Engine worker snapshot capture/application and rendered overhead lifecycle require separate engine integration.
- [ ] End-to-end wire component insertion/update/removal remains unverified here; receiver tests do not prove replication delivery.

## Out of scope

- Spell damage/healing, resource spending, cooldown triggering, NPC spell AI, and new channel metadata: user restricted this slice to presentation lifecycle.
- Learned-spell permissions and faction rules are not modeled by existing cast validation. Current relationship classification treats players as friendly and other health-bearing units as hostile; no faction-system claim.
- Instant casts are rejected with a server diagnostic because there is no timed presentation or effect executor.
