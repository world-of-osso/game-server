# Character customization storage

The server preserves shared character appearance through creation, persistence and login rosters. [Persistence internals](../wiki/systems/persistence.md) describe storage ownership.

## What it must do

- [x] Preserve sex, all six core selectors and ordered additional option/choice pairs through creation, save, database reopen and login roster construction.
- [x] Decode frozen historical character schemas: no appearance; six-byte appearance without/with equipment; seven-byte appearance without/with equipment; seven-byte appearance with equipment and durability.
- [x] Upgrade historical appearance to an empty additional-selection vector while preserving every existing core value, equipment entry and durability value.
- [x] Rewrite known old records to the current schema at startup once; reopening current records must not alter their bytes or manufacture records/selections.
- [x] Encode only the established pre-item-source V7 disk schema through a private stored DTO. Runtime `CharacterData` cannot derive bitcode serialization; wire equipment changes must not change current or legacy equipment bytes.
- [x] Hydrate item-based cached appearance from matching owned slot/ItemID and global GUID source markers before roster/login/runtime use. Unmarked historical GUIDs and pre-ownership caches are Retail; display-only entries have no source.
- [x] Reject Forever cache writes without matching owned GUID/source instead of dropping identity on reopen. Creation persists inventory ownership before the source-free character/name record.
- [x] Unknown/corrupt records fail explicitly instead of silently resetting appearance. Keep established legacy decoder order; add no item-source compatibility decoder.

## How it works

- [Shared appearance contract](../../../shared-protocol/docs/specs/character-customization.md).
- [Persistence](../wiki/systems/persistence.md).

## Implementation inventory

- `crates/server/src/character_data.rs`: current and frozen historical schemas and conversions.
- `crates/server/src/persistence.rs`: startup migration.
- `crates/server/src/persistence_character.rs`: character creation, save, load and roster source.
- `crates/server/src/persistence_types.rs`: live component snapshots.
- `crates/server/src/auth.rs`, `auth_character.rs`: appearance in creation and login responses.

## Tests asserting this spec

- `crates/server/src/persistence_character_abi_tests.rs`: independent pre-source V7/V6 nonempty gear, exact V7 bytes, temporary redb reopen, GUID hydration independent of race, wire source serialization and unsupported unowned Forever writes.
- `crates/server/src/skyborne_kit_tests.rs`: all ten authored race/class pairs preserve Forever owned sources through actual creation/save/reopen.
- `crates/server/src/persistence_customization_tests.rs`: frozen pre-change equipment/durability records and temporary-database reopen.
- `crates/server/src/persistence_tests.rs`: earlier appearance schemas.
- `crates/server/src/auth.rs`: nonempty additional choices across creation/reopen/roster.
- `crates/server/src/barber_shop.rs`: core-only barber changes preserve existing additional choices.
- `crates/client/src/bin/game_cli.rs`, `crates/server/src/bin/admin.rs`: existing core-only CLI defaults remain empty for additional choices.

## Known gaps (current cycle)

- [ ] Accidentally written source-bearing wire character layouts are unsupported; no new decoder or live-data migration policy is authorized.
- [ ] Engine integration owns catalog validation and disjoint core/non-core option authoring.

## Out of scope

- Live database migration/deployment in this change; tests use temporary databases only.
- New account unlock/catalog services or live compatibility with old client wire payloads.
