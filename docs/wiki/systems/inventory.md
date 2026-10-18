# Inventory

Requirements and data gaps: [inventory spec](../../specs/inventory.md).

Item-use scaling and regression evidence: [[item-healing-scaling]].

## mc548b regression recovery — targeted only (2026-10-18)

Lead mc548b server2644c24f2 is RED771/5/13ignored816.87s; protocol stage passed. Exact requested later-master reproduction mergedcfdfc3ee6 into the feature as356d4493a with protocol999b52e: all five named cases failed,0/5. Repairs6ac4b2afc/3a6b3ea7c/f97c118e4 pass the unchanged five tests5/0/25.95s. This is targeted recovery, not a broad-gate or landing certificate.

| Root | Repair/proof boundary |
|---|---|
| Three item-use error-count failures | Shared spell Harness inbox is the one InventoryError drain; Eater's per-frame adapter consumes that inbox instead of draining the receiver a second time. Toy diagnostics use the same capture. All manual drains were audited; separate map/group harnesses keep their own collectors. Bread/combat, companion fourth-item refusal and duplicate mount-learning expected counts are unchanged. This was introduced test plumbing, not proof production stopped refusing. |
| Valid later quest references | QuestDb keeps authentic `content_item` metadata instead of pruning to the initial reference list. Source2947/2512/2101 all have MaxCount0; no default metadata, unknown-definition success or ownership-limit bypass was added. Existing source-qualified progress/turn-in/abandon test is unchanged. |
| XP-only turn-in | Empty creation batches return their input without item/catalog/model work; empty rewards also skip capacity validation. Actual92460 has no item rewards, so its captured XP turn-in no longer depends on unrelated inventory templates. Real item batches retain all ownership/storage checks. |

Current five-case proof is atf97c118e4, after the last source change; later documentation changes do not invalidate it. No own broad rerun; lead reruns coordinated server/protocol/client integration. [Quest contract](../../specs/quests.md).

## Ownership MaxCount — implementation and targeted proof (2026-10-18)

`ItemTemplate.max_count` reads signed `content_item.MaxCount` in both source namespaces; non-positive values are unlimited, independently of `max_stack` and equipped-only limits. `inventory::load_model`/`load_model_from_record` load a passive personal-bank ownership snapshot. `InventoryModel.bank` stays unattached unless the existing authorized bank-move path attaches it; ownership visibility never enables bank moves. `owned_count` now includes source-qualified equipment, bags, overflow and personal bank. Same-GUID moves exclude their incoming instance, and a bank snapshot does not double-count a GUID already moved into carried storage.

All three creation/storage entries (`store_item`, `store_item_at`, `store_new_item_in_best_slots`) check MaxCount before mutation. Existing loot/static-merchant/economic-merchant/charter prechecks remain and gain bank-aware counts. Quest starters/rewards now stage their entire grant through this model instead of pushing into `InventoryData.items`; a cap failure preserves inventory, gold, bank and quest-log state and sends only the typed owner inventory error. Missing item definitions are explicit errors, not fabricated quest metadata.

Profession `Bags::store` preserves the typed error. CheckItems maps only `ItemMaxCount` to `CastFailure::Inventory`; all other store failures keep their existing `InventoryFull` mapping, and missing metadata/reagents/tools keep their existing codes. The cast-reporting boundary selects owner `InventoryChannel` errors versus ordinary `CombatChannel` cast failures. Normal create-item preflight therefore sends `ItemMaxCount` without fabricated `CastFailed::InventoryFull`; a true full bag retains that old spell error. Full-mask triggered creation still reaches the shared effect/storage cap. The appended shared error uses Retail GlobalStrings10918 `ERR_ITEM_MAX_COUNT`; native account dispatch forwards that text. This is CPU dispatch, not native pixels.

Source: pinned TC `Player.cpp:9490–9512,9928–9954` (personal-bank ownership, non-positive unlimited, moving-item exclusion), `Spell.cpp:7648–7656` (SendEquipError/SPELL_FAILED_DONT_REPORT), `SpellEffects.cpp:1426–1440` (effect-path storage bound). [Contract](../../specs/inventory.md), [quest contract](../../specs/quests.md), [Doomsday acceptance](../../specs/toys.md).

| Targeted scope | Proof and limit |
|---|---|
| Shared model at1a811a183 | RED0/2→GREEN5/0: bags/closed bank, correct error/original GUID/atomic record, non-positive unlimited, targeted/starter storage, owned-bank move. |
| Quest requests at23749fa05 | RED0/2→GREEN3/0: starter/turn-in cap rejection, bank/inventory/gold/log retention, initial-grant positives. Synthetic references hydrate real product metadata; no canonical data edit. |
| Create-item atc67ea2596 | Production ProfessionData RED2/1→GREEN4/0: direct refusal/typed owner error, true full-bag error unchanged, first-product positive, triggered-effect cap. One intermediate compile failure exposed the former string-erasing Bags adapter; fixed rather than parsed. |
| Shared/client | Protocol2a5307c wire1/0; engined73fa72e6 account dispatch1/0. Append-only enum, source wording; no native presentation certificate. |
| Integration | Lead owns the broad toys/inventory/item/loot/mail/vendor/quest_reward/profession/bank/check/fmt gate across all three repos. No broad/final-green or landing claim; the three-item2513 batch individually passed mc548b, whose aggregate remains RED. |

## Storage

`InventoryData.items` (table `inventories`) remains the owned bag-item list that trade, auction, mail and profession code edit. Two tables added beside it:

| Table | Key | Value |
|-------|-----|-------|
| `inventory_layouts` | `CharacterId` | `Vec<ItemPlacementData{item_guid, bag, slot}>` |
| `item_instances` | item guid | `ItemInstanceData{soulbound}` |

Instance state is keyed by guid, so it follows an item through trade, mail and auctions. Item rules (inventory type, `Stackable`, required level, class/race masks, bonding) come from source-qualified world.db catalogs in the `ItemTemplates` resource (`item_templates.rs`); details below. Visual equipped items stay in `character_equipment` (visual-slot keyed, see [[persistence]]). `character_nonvisual_equipment` stores neck, paired fingers/trinkets and legacy relics as `(Retail inventory slot id, InventoryItemData)`; startup creates the additive table, so existing characters load an empty non-visual list without rewriting their visual or character ABI. Non-visual GUIDs participate in source/binding loading and the same atomic inventory transaction, snapshots/deltas and `EquippedItems` stat input. `persistence_inventory.rs` loads and saves all of these plus `CharacterData` in one transaction.

## Definition source

Contract: [item definition source](../../specs/item-definition-source.md). The additive global `item_definition_sources` table stores `GUID -> u8` (0 Retail, 1 Forever70205), independently of owner rows. Missing legacy markers mean Retail; invalid tags and changes to existing tags panic with the GUID. `InventoryData` and `ItemInstanceData{soulbound}` encodings are unchanged.

`load_item_definition_source(db, guid)` reads identity. `write_item_definition_source(txn, guid, source)` marks a newly minted GUID in its creation transaction, accepting repeated identical writes. Creation callers must tag Forever explicitly. `InventoryRecord.definition_sources` loads identities for bag, equipment and equipped-bag GUIDs. Record saves only add/validate markers; they never erase markers when ownership changes. The inventory model preserves the map and copies provenance for GUIDs it mints. Incoming transfer GUIDs keep their global marker even when the receiving pre-transfer record lacks that map entry.

`ItemTemplates { retail, forever }::get(source, id)` probes only the selected map. `load_item_templates` is Retail-only; startup uses `load_runtime_item_templates` for both namespaces. `load_model` rejects any owned GUID without a selected-source definition. Trade captures source before partial-stack GUID minting; incoming storage receives explicit source before bag/equip validation. Mail, banks and buyback read global GUID identity. Bank deposit source maps preserve mixed-source stacks and write minted markers with the inventory transaction. Auctions clone source onto their new listing GUID through `MarketChanges.definition_sources` in the market transaction; query transforms receive a pure GUID-source callback backed by one read table per request batch.

`EquippedItems.items` stores `(slot, source, id)`. Stats choose Retail `ItemScalingTables` or `ForeverItemScalingTables` explicitly; missing selected definitions/tables panic with context. Scalar formulas remain unchanged. Wire stacks, appearances, trade, auction browse/listings and buyback retain source. Unowned content-source and Classic-parity limits are tracked only in the [source contract](../../specs/item-definition-source.md#known-gaps-current-cycle).

## Bounded owned-commerce and requirement proof (2026-10-05)

[Owned-commerce ledger](../../../target/owned-commerce-source-runtime/ledger.md) records four passing production-handler tests, zero failed/ignored, at server `2af05c1` / shared `48f3640`: mail send/return/take with four redb reopen boundaries and source-qualified wire/equip eligibility for 2947; guild-bank deposit/withdraw for both 2512 sources with app/connection/database recreation, permissions and ownership/log checks; partial auction listing GUID minting, persisted auction restore, buyout/mail claim and final reopen without cross-source merging; and sale/buyback of both 2101 sources with gold 10000→10002→10000. Buyback preserves the live connection across vendor/redb reopen: **live-session only**, not logout/server-restart persistence. Catalog values use captured importer fixtures, not invented sale prices. Storage reopen is not crash/network/process-death proof.

[Requirement ledger](../../../target/source-requirements-proof/ledger.md) records behavioral RED 11 failures at `46d9702`, then GREEN 11 passed/zero failed or ignored at implementation `1a1b049` plus the retained unowned scope patch. Coverage: source-qualified bag/owned counts and removal, quest progress/turn-in/abandon, profession reagents/explicit/category tools, static/economic merchant unique limits and loot unique limits. Initial helper/dependency failures receive no behavioral credit. These bounded receipts do not establish whole commerce parity, native UI acceptance, broad-suite success or current whole-HEAD acceptance; Main owns integration verification.

## Model

`InventoryModel::from_record` builds a grid (backpack 16 slots, bags 1-4 size 0). Items with a valid placement keep it; unplaced items (new loot, trade, mail) take the first free slot; a guid duplicated within one character (trade/auction partial stacks keep the source guid) gets a fresh guid; items with no free slot go to `overflow` so saving never drops them. Bind-on-pickup and quest items are marked soulbound on load; bind-on-equip items when equipped. Same-source, same-ID stacks merge up to the selected `Stackable`.

`bag_count`, `bag_item_ids`, `destroy_bag_items` and `owned_count` require an explicit source. Removal prechecks only that source and preserves bag order; insufficient selected-source counts leave every stack unchanged. `owned_count` includes bags, equipment, overflow and personal bank; [ownership-cap mechanism and current proof](#ownership-maxcount--implementation-and-targeted-proof-2026-10-18). Retail profession reagents/tools (including categories), static/economic merchant purchases and loot limits select Retail, matching their existing producer. Quest views and removal read each matching GUID's persisted source, counting/removing Retail only; snapshot, progress, giver markers, turn-in and abandon share these predicates. No numeric-ID remap or recipient-race lookup is involved. Unowned reward provenance remains a [contract gap](../../specs/item-definition-source.md#known-gaps-current-cycle).

Off-hand weapons follow TC `Player::CanEquipItem` (Player.cpp:10866-10886): one-hand (13) and off-hand (22) weapons need `Player::CanDualWield`, i.e. a learned SPELL_EFFECT_DUAL_WIELD spell (`stats::knows_effect`, set per request by the inventory systems). One-hand polearms never fit the off hand, and auto-equip offers the off hand only to dual wielders. No content off-hand weapon has ITEM_FLAG3_ALWAYS_ALLOW_DUAL_WIELD; TC's unlearn-time off-hand removal (`AutoUnequipOffhandIfNeed`) is not modelled ([spec](../../specs/class-completeness.md#dual-wield-and-parry-knowledge)).

### Titan's Grip hand rules — 2026-10-15

Each inventory request derives Titan's Grip from the player's learned effect155 spells, alongside Dual Wield40. TC a352b1fa `Spell::EffectTitanGrip` (SpellEffects.cpp4956–4962) supplies independent weapon/armor subclass masks from `SpellEquippedItems`; `Player::CanTitanGrip` (Player.cpp13142–13163) treats a zero mask as unrestricted. Native46917 has no equipped-items restriction; native120553 restricts weapons to mask322. No Fury/class-name shortcut or hard-coded axe/mace/sword list is used.

`FindEquipSlot` (Player.cpp9275–9281) admits inventory type17 to the off hand only with both capabilities and a matching subclass. `IsTwoHandUsed` (13205–13215) and hand displacement preserve the other hand for permitted two-hand weapons. Ranged inventory types retain their existing both-hands restriction. Existing equipment level/class/race checks and source-qualified ownership remain unchanged. New requests observe unlearning, but retroactive off-hand eviction, proficiency, ability acquisition and whole-Fury acceptance remain gaps. Native request→persistence tests live in `inventory_titans_grip_tests.rs`; initial behavioral RED: three admission/auto-equip failures, two negative controls pass. At9593985e all five native tests pass under four threads (73.94s); this is bounded request→persistence proof, not whole-Fury acceptance. Merged7ff7b5e4 adds native2504 ranged rejection and retains the three existing Dual Wield controls: nine pass/zero fail under four threads (7.62s tests); workspace/all-target check72.481s and fmt4.779s pass without warnings. Manual changed-Rust readability inspection found no in-scope issue. Independent/full-Fury/legal-loadout/rendered acceptance remains open.

Operations mutate the model; `with_appearance_sync` then rewrites the appearance entry of each equipment slot whose item changed, keeping `hidden`. `into_record` writes placements in bag order.

## Sources

- [Inventory persistence](../../../crates/server/src/persistence_inventory.rs)
- [Inventory model](../../../crates/server/src/inventory_model.rs)
- [Definition source tests](../../../crates/server/src/persistence_item_source_tests.rs)
- [Owned runtime source tests](../../../crates/server/src/inventory_source_tests.rs)
- [Owned-commerce ledger](../../../target/owned-commerce-source-runtime/ledger.md), [requirement ledger](../../../target/source-requirements-proof/ledger.md) — exact executed revisions, scope patches, commands and saved output.

## Non-visual equipment — verified 2026-10-11

`ItemTemplate.unique_equipped` reads ItemSparse.Flags_0 bit `0x00080000` (`ITEM_FLAG_UNIQUE_EQUIPPABLE`, TrinityCore a352b1fa ItemTemplate.h). `equip_limit` resolves ItemSparse.LimitCategory into the pinned `item_limit_categories.tsv` export: Retail12.1.0.69933 FDID1237440/layoutB0A784BD,457 records/zero dropped, content MD5`951698610c85e466fe3e1d061142115b`. `ItemLimitCategory.Quantity` is field1 (DB2Structure.h ItemLimitCategoryEntry). Distinct Alchemist Stones35748/35749 share category130/quantity1. `Player::CanEquipUniqueItem` (a352b1fa Player.cpp:25199-25226) checks both same-item flags and category quantity, excluding the replaced slot; `FindEquipSlot` (8480-8671) chooses first-empty then the lower-level valid paired occupant. Missing categories fail catalog loading rather than inventing a cap. The local catalog has211 non-visual items with categories; none use the three authored condition rows (categories353/357).

Native proof uses privateUDP5821, disposable fb_equip accounts and a copied world.db. Arred17512 is relocated near Northshire only in that copy; its original flags/inventory/prices remain unchanged. Mood Ring7338 is purchased through the native merchant; neck4112, ring1319 and trinkets1490/2820 are authentic admin bag grants. Both skins show all five slots, Stamina895→897 and Versatility0→1%, native ring tooltip and paired drag swap. Database reopen/migration and existing aura-stat composition have handler tests. On-equip ItemEffect triggers were not previously applied by this stat system and are not claimed here. Both fresh client processes after a private server restart retain exact item GUIDs/slots/stats. Each purchase costs10000copper. Detailed logs/receipts remain in engine `data/diagnostics/equip-slots-2026-10-11/`; twelve inspected1280px PNGs live in `/syncthing/AgentShared/2026-10-11/equip-slots/`. [Native investigation](../../../../game-engine/docs/wiki/investigations/nonvisual-equipment.md) owns the Forever pointer-order regression and client wiring.

## Networking

- `send_initial_inventory` (`Added<InWorld>`) sends both snapshots, stores the view in `SentInventoryView` on the connection, and persists new placements.
- Request systems load the model, apply the operation, persist, and call `GoldUpdateQueue::record_items`. Errors reply `InventoryError` on `InventoryChannel`.
- `GoldUpdateQueue::record` (trade, auction, barber, mail) also marks the character's inventory changed; profession craft/gather call `record_items`.
- `send_inventory_deltas` (PostUpdate, before replication send) drains the changed set, diffs the persisted view against `SentInventoryView`, sends `InventoryDelta`, and copies the persisted appearance onto the player's replicated `EquipmentAppearance` and the equipped item ids onto `EquippedItems` ([[character-stats]] gear input; `send_initial_inventory` inserts it on world entry).
