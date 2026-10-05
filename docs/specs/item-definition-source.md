# Owned item definition sources

Item definition identity belongs to the item GUID, not its owner or race. Server persistence uses `shared::item_data::ItemDefinitionSource` without changing the serialized inventory or soulbound-instance layouts. Mechanism: [inventory storage](../wiki/systems/inventory.md#definition-source).

## What it must do

- [ ] Distinguish Retail and Forever70205 GUIDs sharing item ID 2947, including database close/reopen.
- [ ] Preserve both identities when two owners exchange those GUIDs through the existing inventory-record write transaction.
- [ ] Interpret absent legacy markers as Retail by representation, without querying another catalog.
- [ ] Reject malformed markers and attempts to change an existing GUID's source explicitly.
- [ ] Preserve source metadata through inventory-model record conversion and copy it when the model mints a duplicate or split GUID.
- [ ] Resolve owned definitions and equipped scaling by `(source, authored item ID)`, independent of owner race; missing selected catalogs fail explicitly.
- [ ] Never merge stacks with different sources, including splits into occupied slots and bank deposits.
- [ ] Preserve source through actual trade, mail, bank, guild-bank, auction and buyback transfers and emitted item records.
- [ ] Keep known Retail content minting explicitly Retail; isolate authored Forever creation.

## How it works

- [Inventory definition-source storage and API](../wiki/systems/inventory.md#definition-source).

## Implementation inventory

- `crates/server/src/persistence_inventory.rs` — GUID marker helpers and owned-record source map.
- `crates/server/src/persistence.rs` — additive table initialization.
- `crates/server/src/inventory_model.rs` — source-qualified validation, stacking, snapshots and minted-GUID provenance.
- `crates/server/src/item_templates.rs` — explicit catalog lookup and separate Retail-only/runtime loaders.
- `crates/server/src/stats.rs` — source-qualified equipped definitions and scaling resources.
- `crates/server/src/{trade_support,mail,bank,guild_bank,merchant}.rs`, `auction_house/`, `economy/store.rs` — owner-transfer boundaries.

## Tests asserting this spec

- `crates/server/src/persistence_item_source_tests.rs` — concrete redb reopen, exchange, legacy and invalid-marker cases.
- `crates/server/src/inventory_source_tests.rs` — completed trade/reopen/equipment/stat/snapshot boundary; mixed-source merges, splits and missing catalog.

## Known gaps (current cycle)

- [ ] Creation callers must explicitly mark new Forever GUIDs in their creation transaction.
- [ ] Production source-table provisioning and independent integration gate remain main-owned.
- [ ] Unowned `QuestTemplate` reward IDs/`QuestItemInfo`, `VendorSlot`/`VendorGoods`, `LootRow` and `LootContent::Item` lack authored source fields. Current Retail-backed minting stays Retail; no Forever origin is inferred.
- [ ] Original Classic combat/inventory parity remains unsupported; this identity boundary does not implement it.

## Out of scope

General dual-realm gameplay, market redesign, live database migrations, serialized-layout changes and race-based transfer inference.
