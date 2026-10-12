# Mount Journal authority

User-approved2026-10-11 Retail account Mount Journal. [Implementation](../wiki/systems/mounts.md#mount-journal); [native UI contract](../../../game-engine/docs/specs/mount-journal.md).

## What it must do

- [x] Use real Mount/SourceSpell/MountXDisplay metadata from read-only Retail12.1.0.69933 world.db; no invented starter mounts or display-ID mounting. Carry source text, authored flags and capability-derived ground/flying/aquatic/skyriding/passenger categories.
- [x] ITEM_SPELLTRIGGER_LEARN_SPELL items teach their authentic mount and consume one physical item in the same durable transaction. Duplicate learning preserves inventory; normal learned source spells grant account ownership.
- [x] Account ownership/favourites survive reopen, remain isolated by account, and update every connected character of that account. Unowned entries cannot be favourited.
- [x] Every journal reader/request sees initialized ownership across deferred auth/world entry. Summon/random use existing spell validation and mount capability rules; combat/unowned requests refuse; dismount cancels the aura. Random candidates must be owned, favourited and usable.
- [ ] Native privateUDP5901 learning/summon/favourite/random/alternate-character proof in both skins.

## How it works

- [Server mounts](../wiki/systems/mounts.md#mount-journal).

## Implementation inventory

- `crates/server/src/mount_journal.rs` — read-only catalog, account synchronization and spell requests.
- `crates/server/src/persistence_mounts.rs` — account state and atomic item learning.
- `crates/server/src/inventory.rs`, `item_templates.rs` — authentic UseItem routing.
- `crates/server/src/spell_cast/mod.rs`, `collection.rs`, `auth_character.rs` — account cast authority and removal of legacy fabricated mount paths.
- Protocol `src/protocol/mount_messages.rs` — journal catalog and favourite/random requests.

## Tests asserting this spec

- `crates/server/src/mount_journal_tests.rs`, `persistence_mounts.rs`.
- `crates/server/src/item_use_tests.rs`, `item_templates_tests.rs`, `collection_tests.rs`.
- `crates/server/src/auras/mount_tests.rs` — existing riding/flight/aura boundaries.
- Native `godot/tests/mount_journal_live.gd` in game-engine.

## Known gaps (current cycle)

- [ ] Both-skin live acceptance and final targeted/check/fmt/protocol gate.

## Out of scope

New flight physics, canonical world.db mutation, sharedUDP5000 operations and unrelated full suites.
