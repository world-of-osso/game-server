# Appearance collection

Server-side account-wide transmog appearance collection (Retail `C_TransmogCollection`). The contract is shared-protocol `transmog::AppearanceCollection` and `protocol/tooltip_messages.rs` `AppearanceCollectionUpdate` (`TooltipChannel`). The client uses it to mark tooltip items collected (game-engine `docs/specs/unit-tooltip.md`).

## What it must do

- [x] An appearance is the item's Retail `ItemAppearance` id: its `item_modified_appearance` row with the lowest `ItemAppearanceModifierID` (`ItemTemplate::appearance_id`). Items without such a row (cloth, potions, rings, trinkets, bags) have no appearance.
- [x] The account learns an item's appearance when a character of the account has it equipped or soulbound (Retail: learned on equip or bind). Bind-on-pickup and quest items are soulbound as soon as they are owned; bind-on-equip items once equipped.
- [x] Learned appearances are account-wide: `account_appearances` redb table, account id -> serde_json `AppearanceCollection`, written as soon as something new is learned.
- [x] The owning client gets the whole collection (`AppearanceCollectionUpdate { appearances }`, sorted) when a character enters the world and again whenever an inventory change teaches a new appearance.
- [ ] Wardrobe UI, applying transmog, collection from vendor purchase refunds, unlearning.

## Tests asserting this spec

- `crates/server/src/appearance_collection_tests.rs`: a Worn Shortsword in the bags is not learned; equipping it teaches `ItemAppearance` 154, which is saved for the account and sent; a second character of the account enters the world with it; owning the bind-on-pickup Verigan's Fist teaches 2288 at world entry.
- `crates/server/src/item_templates_tests.rs`: Worn Shortsword 154, Verigan's Fist 2288, Linen Cloth none.
