# Transmog application

Retail Mainline Transmogrifier service in `crates/server/src/transmog.rs`, using account-collected visuals and authentic ItemModifiedAppearance source rows. [Client contract](../../../game-engine/docs/specs/transmog-apply.md); [system](../wiki/systems/transmog-apply.md).

## What it must do

- [ ] Authored transmogrifier flags and gossip open the Transmogrify service, not a vendor.
- [ ] Offer collected appearances valid for the equipped item's class, armor/weapon type, slot and character restrictions.
- [ ] Stage pending changes; validate all requested GUIDs/choices before changing anything.
- [ ] Charge authentic locally derivable target cost atomically with per-item appearance modifiers and binding; original appearance restoration is free.
- [ ] Replicate self/observer equipment appearance and preserve through relog and gear swaps.
- [ ] Permit hide options only for Retail-supported slots; illusions require authentic local data.
- [ ] Saved outfits follow core application and use the same application validation.

## How it works

- [Transmog system](../wiki/systems/transmog-apply.md)

## Implementation inventory

- `crates/server/src/transmog.rs` — service, eligible-source catalog, validation and atomic application.
- `crates/server/src/transmog_store.rs` — persisted per-GUID appearance modifiers, separate existing item ABI.
- `crates/server/src/persistence_inventory.rs` — atomic inventory/gold/appearance persistence.
- `crates/server/src/inventory_model.rs` — project owned modifiers onto equipped visuals.
- `crates/server/src/npc_interaction/gossip.rs`, `zones.rs` — preserve authored Retail flags and service routing.
- Sibling `shared-protocol/src/protocol/transmog_messages.rs` — source-qualified service and pending application messages.

## Tests asserting this spec

- `crates/server/src/transmog_tests.rs` — host-message application through real world.db gossip.
- `crates/server/src/npc_interaction/gossip.rs` — authored service/gossip regression tests.

## Known gaps (current cycle)

- [ ] Only items with `ITEM_FLAG2_OVERRIDE_GOLD_COST` have proven local exact costs. Other ItemSparse SellPrice values are not a substitute for dynamic pricing: ItemPriceBase, ImportPriceQuality/Armor/Weapon and ItemClass.csv are absent from audited local inputs. Return explicit cost-unavailable, not invented prices.
- [ ] Nonzero TransmogPlayerConditionID source requirements are not yet evaluated; such sources are not offered. Artifact override appearance policy is not implemented.
- [ ] Collection state records visual IDs, not source IDs; exact collected-source distinction remains the acquisition system's recorded gap.
- [ ] TransmogIllusion local CSV absent; illusions disabled.
- [ ] Native picker/pending/apply UI, outfits and two-account both-skin live proof remain unverified.

## Out of scope

- World.db writes, guessed prices/data, master merges and port5000 changes are prohibited by this task.
- Forever-only behavior/layout is prohibited; Forever changes art only.
