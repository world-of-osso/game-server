# Transmog application

Retail Mainline Transmogrifier service in `crates/server/src/transmog.rs`, using account-collected visuals and authentic ItemModifiedAppearance source rows. [Client contract](../../../game-engine/docs/specs/transmog-apply.md); [system](../wiki/systems/transmog-apply.md).

## What it must do

- [x] Authored transmogrifier flags and gossip open the Transmogrify service, not a vendor.
- [x] Offer locally supported collected appearances valid for the equipped item's class, armor/weapon type, slot and character restrictions.
- [x] Stage pending changes; validate all requested GUIDs/choices before changing anything.
- [x] Charge authentic locally derivable target cost atomically with per-item appearance modifiers and binding; original appearance restoration is free.
- [x] Replicate self/observer equipment appearance and preserve through relog and gear swaps.
- [ ] Permit hide options only for Retail-supported slots; illusions require authentic local data.
- [x] Account-scoped saved outfits use authored automatically awarded entries2/3. Save validates appearance choices against the existing paid slot list and illusions against authored weapon slots/collected choices, but never changes gold, binding or equipped visuals. Loading stages choices; paid Apply remains the unchanged application path.

- [x] Available sets are class-usable, condition-free sets with a collected member; each carries one primary appearance per transmog slot (collected member first; TrinityCore CollectionMgr::IsSetCompleted grouping). Members missing from world.db content_item are omitted, never invented.

- [x] Custom Sets are separate account-scoped player presets with server-allocated persisted IDs. Saving/updating validates the same eligible choices and never charges or applies equipment.

## How it works

- [Transmog system](../wiki/systems/transmog-apply.md)

## Implementation inventory

- `crates/server/src/transmog.rs` — service, eligible-source catalog, validation and atomic application.
- `crates/server/src/transmog_store.rs` — persisted per-GUID appearance modifiers, separate existing item ABI.
- `crates/server/src/persistence_inventory.rs` — atomic inventory/gold/appearance persistence.
- `crates/server/src/inventory_model.rs` — project owned modifiers onto equipped visuals.
- `crates/server/src/npc_interaction/gossip.rs`, `zones.rs` — preserve authored Retail flags and service routing.
- Sibling `shared-protocol/src/protocol/transmog_messages.rs` — source-qualified service and pending application messages.

## Catalogue inputs

`scripts/import_transmog_catalogue.py` embeds pinned local-CASC CSV exports in
`scripts/reference/retail-12.1.0.69933/transmog.json` (input SHA-256s included).
Regenerate after exporting seven tables (the six Transmog tables plus
SpellItemEnchantment for illusion names/icons) with the sibling engine's maintained
`export_db2_csv.py`; no world.db writes. Six unreadable sets and56 unreadable set
members are omitted, not fabricated. Source ownership remains visual-level.

## Tests asserting this spec

- `crates/server/src/transmog_tests.rs` — host-message application through real world.db gossip.
- `crates/server/src/npc_interaction/gossip.rs` — authored service/gossip regression tests.

## Known gaps (current cycle)

- [ ] Only items with `ITEM_FLAG2_OVERRIDE_GOLD_COST` have proven local exact costs. Other ItemSparse SellPrice values are not a substitute for dynamic pricing: ItemPriceBase, ImportPriceQuality/Armor/Weapon and ItemClass.csv are absent from audited local inputs. Return explicit cost-unavailable, not invented prices.
- [ ] Nonzero TransmogPlayerConditionID source requirements are not yet evaluated; such sources are not offered. Artifact override appearance policy is not implemented.
- [ ] Collection state records visual IDs, not source IDs; exact collected-source distinction remains the acquisition system's recorded gap.
- [x] Catalogue exposes local Retail69933 TransmogSet/TransmogSetItem, TransmogOutfitEntry/SlotInfo/SlotOption and all93 readable TransmogIllusion records. Seven authentic default illusions are collected (TrinityCore a352b1fa); other condition/acquisition grants remain unimplemented. Set primary appearances distinguish visual-level collected, uncollected and non-collectable ItemModifiedAppearance pieces without broadening paid choices. The state carries no item catalogue: it is resent after every Apply and save, and ~4.8k catalogue sources delayed each live reply by ~15 s.
- [x] Native picker/pending/paid-apply, self/observer rebuild and both-skin relog proven on private55087: target2850 GUID123, sources821/1181,1500copper/application; private-server restart preserved modifiers in the same redb. [Bounded native evidence](../../../game-engine/docs/wiki/systems/transmog-apply.md#native-proof--verified-2026-10-11).
- [ ] Native saved-outfit and catalogue proof remains pending; source/host-message tests are not native acceptance. Purchasable outfit entries are represented with authentic costs but purchasing is not implemented.

## Out of scope

- World.db writes, guessed prices/data and port5000 changes are prohibited by this task.
- Forever-only behavior/layout is prohibited; Forever changes art only.
