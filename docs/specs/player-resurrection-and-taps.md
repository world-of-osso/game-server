# Player resurrection and creature taps

Player resurrection spell effects send consent offers; creature damage exposes tap eligibility without per-viewer creature state. Sources: `crates/server/src/death/resurrection.rs` and `creature_tap.rs`. [Mechanics](../wiki/systems/player-resurrection-and-taps.md).

## What it must do

- [ ] Resurrection spells offer a dead or ghost player the caster's identity, spell and a 60-second timeout without immediately reviving them.
- [ ] Accept restores the spell's percentage of maximum health/mana at the caster's cast-time position; death/corpse/ghost state clears and movement adopts the new position.
- [ ] Decline or expired acceptance leaves the target dead. Every response returns authoritative death state so refused acceptance cannot strand the client waiting.
- [ ] RESURRECT_WITH_AURA applies its triggered aura only after consent.
- [ ] Untapped creatures are available to everyone; a tapper and current group members are not denied. Other viewers are denied. Respawn/evade clears taps.

## How it works

- [Wire and server mechanics](../wiki/systems/player-resurrection-and-taps.md)

## Implementation inventory

- `crates/server/src/death/resurrection.rs`: offer lifetime, authenticated response reception, restoration and owner snapshot.
- `crates/server/src/death.rs`: installs offer response/expiry processing.
- `crates/server/src/spell_cast/{mod,effects}.rs`: dead-ally validation and effect registration.
- `crates/server/src/creature_tap.rs`: change-only tap membership replication and clearing.
- `crates/server/src/group/registry.rs`: stable character identity in group rosters.

## Tests asserting this spec

- `crates/server/src/spell_cast/rezrtap_tests.rs`: actual spell pipeline, host transport, restoration boundary before regen, decline/expiry, Retail 7328/2006/50769, effect 172 and tap projection/clear.
- Shared protocol `tests/rezrtap.rs`: viewer-relative tap predicate and wire round trips.

## Known gaps (current cycle)

- [ ] Targeted proof pending; native GPU/live realm proof is not part of the CPU tests.

## Out of scope

Self-resurrection, pets, corpse-object targeting, battle-resurrection encounter budgets and changes to existing spirit-healer behavior. No merge, push or deployment.
