# Authoritative transmog application

The Transmogrifier service resolves eligible collected Retail appearance sources and persists a batch of owned-item visual modifiers with gold and binding. [Contract](../../specs/transmog-apply.md); [native behavior and live proof](../../../../game-engine/docs/wiki/systems/transmog-apply.md).

## Service and validation

The tracked workspace retains the canonical shared-protocol sibling patch. Leased build/test runs select the matching protocol checkout with a local `.cargo/config.toml` override; remove that temporary override before release and never commit slot paths. The engine helper uses `DEPOT_SIBLING_SHARED_PROTOCOL` for the same purpose.

Merged Retail NPC flags survive zone import. Hashom's authored gossip menu21043 option0 opens the Transmogrifier role, not a vendor. Requests require the active in-range interaction and current equipped GUIDs. Every slot/source/cost is validated before mutation; invalid batches preserve the whole record.

Sources use ItemModifiedAppearance/ItemAppearance rather than treating collection visual IDs as display IDs. Eligibility covers supported slot aliases, armor/weapon groups, source class/race/level masks and zero PlayerCondition. The contract records remaining collection-source and conditional/artifact policy gaps.

## Prices and persistence

Pinned TrinityCore a352b1fa charges the equipped target's GetSellPrice and restores original appearance for free. Only explicit ITEM_FLAG2_OVERRIDE_GOLD_COST targets have derivable exact local prices; other quotes are unavailable because the audited inputs lack dynamic price tables. Never use source sell price or invent a floor.

`transmog_store` records modifiers by item GUID separately from physical item ownership. The inventory write transaction persists gold, binding and modifiers atomically. Projection restores base visuals explicitly when clearing a modifier and preserves unrelated cached visual overrides. Accepted equipment appearance follows normal self/observer replication and is rehydrated after login.

## Metadata and saved configuration — verified2026-10-12

Pinned local-CASC Retail69933 exports supply sets/memberships, outfit entries/slots/options and93 illusions, joined to authentic SpellItemEnchantment names/icons. Each source retains ItemModifiedAppearance.TransmogSourceTypeEnum (`Enum.TransmogSource`), independent of collected ownership. Primary set representatives group by transmog slot; visual-level ownership distinguishes collected/uncollected/non-collectable pieces without broadening paid choices. Unreadable records and members absent from local content are omitted, never invented.

Outfits and Custom Sets are separate account-redb stores. Saves validate available automatically awarded entries, collected eligible choices and collected weapon-illusion IDs. They do not modify gold, binding or equipment. Custom Set IDs come from a persisted server counter. The native Save Outfit action applies pending appearances through the unchanged paid batch, then sends the saved configuration after acceptance; direct save and paid Apply remain distinct operations.

Current33 transmog host/service tests pass, including catalogue metadata, refusal cases, account isolation, database reopen and paid authority. Both-skin private5926 save/load, real source/collection filters and set/preset proof are recorded in the [client evidence](../../../../game-engine/docs/wiki/systems/transmog-apply.md#retail121-native-proof--verified2026-10-12). No world.db or shared realm mutation. Illusion configuration proof does not certify visual enchant effects or illusion-price settlement. Dynamic prices, conditional/artifact acquisition, event eligibility and purchased entries remain explicit policy gaps.

## Historical application proof — verified2026-10-11

Server9d415a7 transmog_ behavioral filter10/10 includes authored service/gossip, actual application/gold/binding, atomic invalid batches, closure/range rejection, unavailable dynamic prices, free reset, gear swapping and cached-visual preservation. Native build and both-skin private paid application, observer rebuilds, process relog and private-server restart against the same redb are detailed in the linked native page. No world.db writes, master merge or UDP5000 mutation.

At this historical checkpoint, saved outfits and illusion metadata were unavailable; the2026-10-12 section supersedes those gaps. Independent verification then failed due expired Claude OAuth; the revamp used an independent read-only Astra verifier.

## Sources

- [Service](../../../crates/server/src/transmog.rs), [tests](../../../crates/server/src/transmog_tests.rs), [modifier store](../../../crates/server/src/transmog_store.rs).
- [Inventory transaction](../../../crates/server/src/persistence_inventory.rs), [projection](../../../crates/server/src/inventory_model.rs).
- Local pinned TrinityCore a352b1fa `TransmogrificationHandler.cpp`, `Item.cpp` and ItemTemplate sources; retained in game-engine canonical transmog diagnostics.

## See Also

- [Native transmog](../../../../game-engine/docs/wiki/systems/transmog-apply.md) — detached pending preview, shared-skin frame and native proof.
