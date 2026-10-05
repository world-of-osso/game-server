# Owned item definition sources

Item definition identity belongs to the item GUID, not its owner or race. Server persistence uses `shared::item_data::ItemDefinitionSource` without changing the serialized inventory or soulbound-instance layouts. Mechanism: [inventory storage](../wiki/systems/inventory.md#definition-source).

## What it must do

- [x] Distinguish Retail and Forever70205 GUIDs sharing item ID 2947, including database close/reopen.
- [x] Preserve both identities when two owners exchange those GUIDs through the existing inventory-record write transaction.
- [x] Interpret absent legacy markers as Retail by representation, without querying another catalog.
- [x] Reject malformed markers and attempts to change an existing GUID's source explicitly.
- [x] Preserve source metadata through inventory-model record conversion and copy it when the model mints a duplicate or split GUID.
- [x] Resolve owned definitions and equipped scaling by `(source, authored item ID)`, independent of owner race; missing selected catalogs fail explicitly.
- [x] Never merge stacks with different sources, including splits into occupied slots and bank deposits.
- [ ] Preserve source through actual trade, mail, bank, guild-bank, auction and buyback transfers and emitted item records.
- [x] Isolate authored Forever creation: the factory-selected definition source is attached to every new kit GUID and survives all ten race/class creation/reopen pairs.
- [ ] Keep every known Retail content minting boundary explicitly Retail.
- [ ] Retail profession reagents, explicit tools and tool categories must count/consume only Retail GUIDs; Forever-only bags cannot satisfy those requirements.
- [ ] Retail quest item progress, completion, turn-in and provided-item removal must ignore same-ID Forever GUIDs.
- [ ] Merchant and loot unique limits must count the producer-selected source only, including owned equipment and overflow; numeric IDs remain unchanged.
- [x] Preserve the pre-source character disk ABI with nonempty equipment; hydrate runtime appearance from owned-slot GUID markers after reload, never from recipient race.

## How it works

- [Inventory definition-source storage and API](../wiki/systems/inventory.md#definition-source).

## Implementation inventory

- `crates/server/src/persistence_inventory.rs` — GUID marker helpers and owned-record source map.
- `crates/server/src/persistence.rs` — additive table initialization.
- `crates/server/src/inventory_model.rs` — source-qualified validation, stacking, snapshots and minted-GUID provenance.
- `crates/server/src/item_templates.rs` — explicit catalog lookup and separate Retail-only/runtime loaders.
- `crates/server/src/stats.rs`, `item_stats.rs` — source-qualified equipped definitions and separate scaling resources; Forever loading excludes Retail's squish curve.
- `crates/server/src/player_create_info.rs` — factory-selected source on new kit GUIDs.
- `crates/server/src/character_data.rs`, `persistence_character.rs` — frozen disk DTO and GUID-hydrated runtime appearance.
- `crates/server/src/{trade_support,mail,bank,guild_bank,merchant}.rs`, `auction_house/`, `economy/store.rs` — owner-transfer boundaries.

## Tests asserting this spec

- `crates/server/src/persistence_item_source_tests.rs` — concrete redb reopen, exchange, legacy and invalid-marker cases.
- `crates/server/src/inventory_source_tests.rs` — completed trade/reopen/equipment/stat/snapshot boundary; mixed-source merges/splits; source-qualified counts/removal and persisted quest-removal collisions.
- `crates/server/src/profession/source_requirement_tests.rs` — real cast checks/reagent removal, explicit and category tool requirements with same-ID Forever/Retail bags.
- `crates/server/src/{quest_tests,merchant_tests,merchant_economy_tests,loot_tests}.rs` — message-path collision fixtures for quest progress/turn-in/abandon and unique limits; IDs 2947/2512/2101 remain unchanged.

- `crates/server/src/persistence_character_abi_tests.rs` — independent historical nonempty V7/V6 bytes, exact V7 save/reopen bytes and GUID hydration across races.
- `crates/server/src/skyborne_kit_tests.rs` — actual ten-pair kit creation/reopen and source assertions.

Retained development proof: `target/owned-source-runtime/ledger.md` records 5/5 targeted GREEN at patch-equivalent `4fea2d0`; `target/character-abi-proof/ledger.json` records 26 passing cases at `a7fbb98` in an invocation that exited 101 on the then-stale kit fixture, followed by the corrected creation test passing at `e405232`. These are bounded development proofs, not a passing full suite or native E2E gate.

## Known gaps (current cycle)

- [ ] Production source-table provisioning and independent integration gate remain main-owned.
- [ ] Unowned `QuestTemplate` reward IDs/`QuestItemInfo`, `VendorSlot`/`VendorGoods`, `LootRow` and `LootContent::Item` lack authored source fields. Current Retail-backed minting stays Retail; no Forever origin is inferred.
- [ ] Auction exact-item search still filters by bare ID despite source-qualified result records; mixed-product selection is not established.
- [ ] Original Classic combat/inventory parity remains unsupported: thrown DPS and Classic-only equipment slots are not implemented. Source-local scaling retains existing formulas, not original Classic mechanics.

## Out of scope

General dual-realm gameplay, market redesign, live database migrations, serialized-layout changes and race-based transfer inference.
