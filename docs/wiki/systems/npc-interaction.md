# NPC Interaction

Right-clicking an NPC opens a gossip menu or a role frame (vendor, trainer, auction house, …), validated by the server. Code: `crates/server/src/npc_interaction.rs` (validation, session, systems) and `npc_interaction/gossip.rs` (world.db gossip loader, what an NPC opens). Protocol: shared-protocol `protocol/interaction_messages.rs`, `InteractionChannel`.

## NPC mailbox service (2026-10-16)

Pinned TC a352b1fa `WorldSession::CanOpenMailBox` accepts creature GUIDs only through `GetNPCIfCanInteractWith(..., UNIT_NPC_FLAG_MAILBOX, ...)`. The ordinary server `InteractNpc` path now opens the existing Mailbox role when that flag is present, even with Katy132969's GOSSIP bit; it does not manufacture an unbound gossip option. Existing same-map/live/range/session guards and mail request persistence remain unchanged. [Toy source and acceptance boundary](../../toy-effects14-coverage.md#stampwhistle66-source-assistant-and-mailbox-2026-10-16) distinguish real summon identity from mailbox service, delivery timing and pending native proof.

## NPC flags

Every NPC replicates `NpcFlags(u64)`: bits 0-31 Retail `UNIT_NPC_FLAG_*` (TrinityCore `NPCFlags`), bits 32-63 `UNIT_NPC_FLAG_2_*`. Set at spawn (`zones.rs`) by `NpcFlags::from_azerothcore(creature_template.npcflag)`.

AzerothCore → Retail mapping: AzerothCore bits 0x1 (GOSSIP) through 0x0400_0000 (MAILBOX) have the same value and meaning in Retail (AzerothCore SPIRITGUIDE 0x8000 = Retail AREA_SPIRIT_HEALER), so they pass through unchanged; higher bits are masked off. AzerothCore has no `NPCFlags2`, so the high word is always 0.

## Messages

| Direction | Message | Meaning |
|---|---|---|
| C→S | `InteractNpc { npc }` | Right-click (Retail `CMSG_GOSSIP_HELLO` and role hellos). `npc` = server entity bits, like `SetTarget`. |
| C→S | `SelectGossipOption { npc, option_id }` | Pick an option of the open gossip menu. |
| C→S | `CloseInteraction { npc }` | Client closed the frame. |
| S→C | `InteractionOpened { npc, kind }` | `kind` = `Gossip(GossipMenu { menu_id, text, options })` or `Role(NpcRole)`. |
| S→C | `InteractionFailed { npc, error }` | `InteractionError::message()` gives the Retail error text. |
| S→C | `InteractionClosed { npc }` | Server ended the interaction. |

## Validation

TrinityCore `Player::GetNPCIfCanInteractWith`, in this order (error text):

1. Player alive — `You can't do that when you're dead.` (ERR_PLAYER_DEAD)
2. NPC alive — `Your target is dead` (SPELL_FAILED_TARGETS_DEAD)
3. NPC reaction to the player not Hostile ([[factions]]) — `Target is hostile` (SPELL_FAILED_TARGET_ENEMY)
4. NPC not in combat (`CombatStatus`) — `Target is in combat`
5. Center distance ≤ `INTERACTION_DISTANCE` 8.5 yd — `You are too far away.` (ERR_USE_TOO_FAR)

Unknown entity, or an NPC with no role — `Invalid target`.

Distance: TrinityCore allows the creature's combat reach + 4 yd (comment: "taken from CGGameUI::SetInteractTarget"), checked with `IsWithinDistInMap`, which adds both units' combat reach. Combat reach is not imported, so the default 1.5 is used for both: 1.5 + 4 + 1.5 + 1.5 = 8.5. The client pre-checks 5 yd (game-engine `target.rs` `INTERACT_RANGE`).

Player dead with a spirit healer is not handled here (see `death.rs`).

## What an NPC opens

- No GOSSIP flag and exactly one role: that role directly (`Role`).
- GOSSIP flag, or several roles: gossip menu `creature_template.gossip_menu_id`, or AzerothCore default menu 0 (one option per role) when the template has none.
- No GOSSIP flag and no role: `Invalid target`.

Roles from flags: QuestGiver (0x2), Vendor (any vendor bit or REPAIR), Trainer (any trainer bit), FlightMaster, Innkeeper, Banker, Petitioner, TabardDesigner, Battlemaster, AuctionHouse, StableMaster. Spirit healers, spellclick, vehicles and mailbox are not roles here.

Menu contents: text from `gossip_menu` (lowest `TextID`) → `npc_text` slot with the highest probability → `broadcast_text` Male/FemaleText (or the inline text); menus without text use `npc_text` 68 "Greetings, $n." (`$n` is left for the client). Options from `gossip_menu_option` where `OptionNpcFlag & npcflags != 0` (AzerothCore rule), text from `broadcast_text` or `OptionText`. Option types routed: GOSSIP (submenu via `ActionMenuID`, else close), VENDOR, TAXIVENDOR, TRAINER, INNKEEPER, BANKER, PETITIONER, TABARDDESIGNER, BATTLEFIELD, AUCTIONEER, STABLEPET. Hidden: NONE (script-only), QUESTGIVER (quests belong to the quest system), spirit healer/guide, ARMORER (AzerothCore hides it), talent unlearn / dual spec / outdoor PvP.

Gap: the `conditions` table is not imported, so conditional options show always (e.g. Innkeeper Farley's Hallow's End "Trick or Treat!") and a menu with several conditional texts uses the lowest `TextID`.

## Session

An open interaction is `ActiveInteraction { npc, role, auction_house, gossip_actions }` on the connection entity. Opening another NPC replaces it (the previous NPC gets `InteractionClosed`). Every frame `close_invalid_interactions` re-runs the validation; on failure the component is removed and `InteractionClosed` is sent. So walking out of 8.5 yd, the NPC dying or entering combat, or the player dying closes the frame. TrinityCore instead re-validates on each auction/vendor/bank packet; the per-frame check gives the same result and also tells the client.

## Auction house

Auction requests need an `ActiveInteraction` whose role is AuctionHouse (TrinityCore checks `GetNPCIfCanInteractWith(auctioneer, UNIT_NPC_FLAG_AUCTIONEER)` on every `CMSG_AUCTION_*`). Without one: `OpenAuctionHouse` answers `success: false`; `QueryAuctions`, `QueryAuctionInventory`, `QueryOwnedAuctions`, `QueryBidAuctions`, `CreateAuction`, `PlaceBid`, `BuyoutAuction` and `CancelAuction` answer `AuctionOperationResponse { success: false, "not interacting with an auctioneer" }`. Auction results arrive as mail, taken at a mailbox ([mail](mail.md)).

The house comes from the auctioneer's FactionTemplate `FactionGroup` (TrinityCore `AuctionHouseMgr::GetAuctionHouseEntry`): Alliance bit 0x2 → 1, Horde bit 0x4 → 2, otherwise neutral → 3. Searches use the session house (the client's `AuctionSearchQuery.faction` is overwritten) and new auctions store it in `AuctionData.faction`. Auctions saved earlier with faction 0 stay visible in every house.

## Tests

`npc_interaction_tests.rs` (real world.db): Auctioneer Fitch 8719 (Stormwind, guid 79705) opens the Alliance house at 3 yd, fails beyond range; Auctioneer Wabang 8724 (Orgrimmar, auctioneer + questgiver) opens default menu 0 with "Auction!" routing to the Horde house; Innkeeper Farley 295 gossip menu 1291 with the vendor option opening Vendor and submenu 1221; Innkeeper Gryshka 6929 is hostile to Humans; NPC in combat; walking away closes the session; auction requests without a session are rejected.

## Known gaps

- Orgrimmar NPCs are not spawned live: Orgrimmar is on map 1, which spawns now, but no Horde character has been checked there.
- Role frames other than the vendor ([merchant](merchant.md)) and the auction house (trainer lists, bank, flight map, binder) have no server content yet; only the open/route/close contract exists.
- `shared::death::NPC_FLAG_SPIRIT_HEALER` is 0x0800 (VENDOR_REAGENT); the real spirit healer bit is 0x4000 (Spirit Healer 6491 has npcflag 16385), so the spirit-healer check in `death.rs` does not match real spirit healers.
