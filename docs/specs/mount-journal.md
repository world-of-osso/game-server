# Mount Journal

Initial world entry publishes the complete authentic catalog. Later account/context responses send changed rows only on the ordered reliable collection channel; a new player entity resets the complete-catalog boundary. authority

User-approved2026-10-11 Retail account Mount Journal. [Implementation](../wiki/systems/mounts.md#mount-journal); [native UI contract](../../../game-engine/docs/specs/mount-journal.md).

## What it must do

- [x] Use real Mount/SourceSpell/MountXDisplay metadata from read-only Retail12.1.0.69933 world.db; no invented starter mounts or display-ID mounting. Carry source text, authored flags and capability-derived ground/flying/aquatic/skyriding/passenger categories.
- [x] ITEM_SPELLTRIGGER_LEARN_SPELL items teach their authentic mount and consume one physical item in the same durable transaction. Duplicate learning preserves inventory; normal learned source spells grant account ownership.
- [x] Account ownership/favourites survive reopen, remain isolated by account, and update every connected character of that account. Unowned entries cannot be favourited.
- Favourite requests must not wait for database writes on the tick; validate fresh durable ownership through FIFO and publish account cache/success/error snapshots after the receipt. Replies remain bound to the same authenticated account, character and player. A random-favourite request captures committed ownership in FIFO order after preceding favourite changes; capability checks and the normal cast run after that read receipt.
- [x] Every journal reader/request sees initialized ownership across deferred auth/world entry. Summon/random use existing spell validation and mount capability rules; combat/unowned requests refuse; dismount cancels the aura. Random candidates must be owned, favourited and usable.
- [x] Native privateUDP5901 learning/summon/favourite/random/alternate-character proof in both skins.

### Acquisition/game integration — user extension2026-10-11

- [x] Authentic vendor mount teaching items retain authored Money, item/currency ExtendedCost, faction and minimum reputation authority and are proved through buy→learn in both skins. TDB1210 has no mount direct-learn vendor entries; none are fabricated.
- Mount purchases multiply every cost by purchase count, reject insufficient payment/reputation/space without changing player state, and commit gold, token destruction, currency and delivery in one redb transaction. Authored CurrencyTypes IDs are not aliases of legacy wallet IDs. Missing cost metadata disables the affected offer, never makes it free.
- Minimum reputation uses ItemSparse rank0–7 and actual CharacterReputation standing; vendor usability reflects the same authority. No reputation discount is introduced.
- [x] Retail AH Miscellaneous→Mounts lists/trades teaching items; authoritative purchase/delivery/use grants ownership, with both-skin live proof.
- [x] Ownership drives Already known item/vendor/AH presentation; mount teaching items grant no transmog appearance.

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
- `crates/server/src/merchant_mount_cost_tests.rs`, `merchant_tests.rs` — paid authentic vendor purchase, costs and reputation.
- Native `godot/tests/mount_journal_live.gd`, `mount_auction_live.gd` in game-engine.

## Known gaps (current cycle)

None open. Final post-merge proof2026-10-12: [engine contract](../../../game-engine/docs/specs/mount-journal.md#known-gaps-current-cycle).

## Out of scope

New flight physics, canonical world.db mutation, sharedUDP5000 operations and unrelated full suites.
