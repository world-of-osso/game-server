# Loot

Server-side creature loot: corpses, personal loot rolls, and the loot requests. The contract is shared-protocol `protocol/loot_messages.rs` (`LootChannel`). The client frame is game-engine `docs/specs/loot-frame.md`.

## What it must do

- [x] A dead NPC keeps replicating as a corpse through its AzerothCore corpse decay (`m_corpseDelay`: 60 s normal, 300 s elite/rare, 600 s boss; `CorpseDelay`, `Corpse`). When the decay runs out the corpse stops replicating until it respawns (`RemoveCorpse`). An NPC without a spawn's `CorpseDelay` loses its corpse at once.
- [x] Respawn time stays `spawntimesecs + m_corpseDelay` (`RespawnDelay`).
- [x] When nobody has loot left on the corpse, the remaining decay shrinks by `Rate.Corpse.Decay.Looted` (0.5) and the respawn moves earlier by the same amount (`Creature::AllLootRemovedFromCorpse`, Creature.cpp:2954-2984). This also applies at death when nobody got any loot (Unit.cpp:14312).
- [x] Looters: the killer (`LastDamager`) and its group members within `GROUP_REWARD_DISTANCE`, the same rule as quest kill credit. Each gets their own roll (Retail personal loot) and is sent `CorpseLootable { lootable: true }`.
- [x] Loot tables: `content_creature_loot_template` and `content_reference_loot_template` (Retail-mapped), `LootMode & 1` rows only. The roll follows `LootTemplate::Process` (LootMgr.cpp:1697):
  - ungrouped rows each roll their `Chance` (percent); a reference row that hits processes the referenced template `MaxCount` times;
  - each group yields at most one row: one roll over the explicitly chanced rows, else one random equal-chance row; an item that already dropped 3 times (1 time if equippable) in the same group is left out;
  - `QuestRequired` rows drop only while one of the looter's quests still needs the item (`Player::HasQuestForItem`);
  - counts above the stack size split into stacks; items without an item template are skipped; at most 16 normal and 32 quest items.
- [x] Coins: `Loot::generateMoneyLoot` over `creature_template.mingold`/`maxgold`, per looter, only for a creature with a loot table: **a creature without one (`lootid` 0 or no `content_creature_loot_template` rows) drops nothing, not even coins** (user rule; AzerothCore would still give its `mingold`/`maxgold`). Slots are the coins first, then the items.
- [x] **Deviation from Retail (user request): junk becomes gold.** A poor-quality (grey, `content_item.Quality` 0) item a roll yields is not put in a slot; its `SellPrice` × count is added to that looter's coins, shown in the normal money slot. Applies to every loot the server generates; creature and chest loot both use this conversion. Grey items are also never listed in unit tooltips (game-engine `docs/specs/unit-tooltip.md`).
- [x] Tooltip drop chances (`LootTables::drop_chances`, answered in `CreatureTooltip`, see `appearance-collection.md` and game-engine `docs/specs/unit-tooltip.md`): the percent chance that one kill drops the item at least once, from the same tables as the roll, without the quest filter. An ungrouped row counts its `Chance`. A group row counts its share of the group's one roll: explicitly chanced rows in order until 100 % is used up, the rest split evenly among the equal-chance rows. A reference row multiplies its chance by the chance that one of its `MaxCount` passes drops the item. Several rows of one item combine as independent rolls. Grey items are left out (they become coins). Example, Defias Thug (38): Red Burlap Bandana 60 %, Darnassian Bleu 30 % × 67 % = 20.1 %, Refreshing Spring Water 9.9 %, Pitted Defias Shortsword 2 %, five pouches 0.2 % / 5 = 0.04 % each.
- [x] `LootUnit { corpse, auto }` answers `LootResponse` with the looter's remaining slots. Refused with `LootFailed`: `NoPermission` (no loot for this player, not a corpse), `PlayerDead`, `TooFar` (beyond `INTERACTION_DISTANCE`, AzerothCore `IsWithinDistInMap(INTERACTION_DISTANCE)`). Opening another corpse closes the previous window.
- [x] `LootSlotRequest { corpse, slot }` for the open window stores the slot: coins with `set_gold` (refused `TooMuchGold` past `MAX_MONEY`), items with `store_item` (refused `CantCarryMore` past `MaxCount`, `InventoryFull` when it does not fit; the slot stays). Each taken slot answers `LootSlotRemoved`; the save reaches the player through `Gold` and `InventoryDelta`.
- [x] Taking the last slot sends `CorpseLootable { lootable: false }` and `LootClosed`.
- [x] `auto` takes every slot at once; the first refusal is reported and its slot stays in the open window.
- [x] `LootRelease` closes the window (`LootClosed`). The window also closes when the looter dies, walks out of range or the corpse decays.
- [ ] Group loot methods (round robin, free for all, need/greed rolls, master loot), coin splitting and tapping by first damage: `SetGroupLootMethod` is stored in the roster only.
- [x] Validated chest uses open the existing loot window from `content_gameobject_loot_template`; quest-required loot is filtered by active item objectives. Taking a chest item uses the same inventory transaction as corpse loot; opening alone grants no item credit. Slot requests recheck life, map-copy and object-box reach.
- [ ] Full chest locking, scripted conditions and visual lifecycle; skinning loot; bind-on-pickup confirmation; loot history.

## Tests asserting this spec

- `crates/server/src/loot/tables_tests.rs`: Lumberjack (no loot table, 40-57 gold columns) drops nothing; Defias Thug drop chances through references and groups; Kobold Vermin 1-5 copper, Diseased Young Wolf beast parts and no coins, the quest pelt only with Wolves Across the Border, equal-chance group of Grey 1-5, stack split, money rule.
- `crates/server/src/loot_tests.rs`: a grey Melted Candle a Kobold Vermin rolls becomes 1 copper per candle in the money slot, and no grey item is left; killing a Kobold Vermin makes it lootable for the killer; looting both slots moves coins and candle into the bags, ends lootability and halves the decay; auto-loot; a full bag keeps the item; a stranger is refused; out of reach is refused and walking away closes; a corpse without loot decays in half the time.
- `crates/server/src/combat_tests.rs`: a lethal swing leaves a replicated corpse; the corpse stops replicating after its decay; the looted halving.
- `crates/server/src/creature_tooltip_tests.rs`: the Defias Thug tooltip lists its drops with chances and without the 23 greys; Corina Steele's lists her 8 weapons with their appearances.
- `crates/server/src/quest_rules_tests.rs`: `quest_needs_item`.
- Live evidence: game-engine `data/diagnostics/npcloot-20260925/`.
