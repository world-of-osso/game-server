# New-Character Kit

A new character gets the items and action buttons TrinityCore a352b1fa gives it in `Player::Create`. `player_create_info.rs` loads them at startup (`PlayerCreateInfos`). Character creation, from the client or from the admin `create-character` command, places the items and stores the bar. [Installed eligibility and race bits](race-class-eligibility.md) select supported pairs; kit contents and authentic starting locations retain their existing source contracts.

## Starting location

Normal starts load from `tdb_playercreateinfo` by **race and class**, including authored facing. Creation stores the map and source orientation in the initial homebind; login applies that facing while the saved position still equals the bind. Existing saved positions are not migrated. A missing ordinary row fails creation before character persistence; no Human/default-coordinate fallback remains. TDB1210 lacks a Void Elf Demon Hunter(29,12) row. The documented DH workaround applies to every DH pair: use the same race's lowest-ID non-DH class start until Mardum1481 is implemented. Never substitute another race. Only documented unimplemented-intro capital/race-start overrides and approved Skyborne95/96 reconstruction remain. NPE and intro fields are retained by the importer, not used to enable an intro flow.

The2026-10-10 no-install Mulgore target(-2917,-257,53) duplicated the old rounded Tauren start table. Root ADT784900 has terrain59.429 there: the target was6.429yards underground. Collision correctly rejects terrain above step reach; map1 data is present and nav-bakes do not supply player ground. Authentic TDB Tauren start is(-2915.55,-257.347,59.2693), orientation0.302378. The creation→real-ground→10seconds gravity regression reproduces the old failure without changing physics or admin teleport semantics. The all-pairs kit regression includes the no-install246-pair creation-catalog receipt (31race entries), validates every offered pair, and exercises all260 server-declared pairs (the extra14 are neutral/Horde Pandaren).

Import with the pinned complete TDB dump, never directly into live `world.db`:

```text
python3 scripts/import-player-starts.py --db data/world.db.new --copy-from data/world.db --dump data/reference/tdb1210/TDB_full_world_1210.26091_2026_09_09.sql
```

The tool snapshots read-only; re-importing an existing scratch destination first writes `<destination>.player-starts.bak`. It replaces only `tdb_playercreateinfo`, preserves all16 source columns including NPE/intro metadata, and refuses live basenames and symlink aliases. Caller owns publication.

## Data

| What | Source | TrinityCore |
|---|---|---|
| Items | DB2 `character_loadout` rows with Purpose 9, joined to `character_loadout_item` (ID order). A row applies to a race when its `RaceMasks` has the race's PlayableRaceBit. | ObjectMgr.cpp:3938-3994, `IsForNewCharacter` |
| Item count | `VendorStackCount`. Food is 4, or 10 for a Death Knight. Drink is 2. Food and drink are class 0 / subclass 5 with a first effect (`item_x_item_effect` → `item_effect.SpellCategoryID`) of 11 or 59. The count is capped at `Stackable`. | ObjectMgr.cpp:3966-3984 |
| Item overrides | `tdb_playercreateinfo_item`. Race or class 0 means all. A positive amount adds the item; -1 removes it. The only row is (0, 6, 40582, -1): no Death Knight gets the Scourgestone. | ObjectMgr.cpp:3788-3812, 3996-4062 |
| Action buttons | `tdb_playercreateinfo_action`. A button is dropped with a warning when its type is not spell (0) or item (0x80), its spell is not in `spell_name`, its item is not in `content_item`, or it is not below 120. This drops Death Knight button 2 (45477, not in Retail SpellName) and Troll Druid button 84 (type item, action 6603). | ObjectMgr.cpp:4205-4247, `IsActionButtonDataValid` Player.cpp:6047-6080 |

Retail 12.x has no CharStartOutfit DB2; wago.tools serves none for 12.1.0.69933. No Retail kit has a shirt. A Human Mage gets Bent Staff 35, the six Mage's pieces and the Hearthstone 6948.

## Placement

`InventoryModel::store_new_item_in_best_slots` follows `StoreNewItemInBestSlots` (Player.cpp:561-596):

1. Copies are equipped one at a time while a free slot takes them (`FindEquipSlot` with no slot and no swap).
   - The off hand is not free under a two-hander.
   - A two-hander moves an equipped off hand to the bags (`AutoUnequipOffhandIfNeed`).
2. The rest go to the backpack like `store_item`.

Then `equip_backpack_items` makes the second pass (Player.cpp:525-554). Each backpack item, in slot order, is equipped if it now has a free slot. Otherwise it moves to the first free backpack slot at or before its own.

The model's own rules decide what is equippable, so these Retail items stay in the backpack:

- **Bags:** the server has no bag containers (bags 1-4 have size 0). TrinityCore equips the kit bag in bag slot 1.
- **Neck, finger and trinket items:** the server has no storage for those slots.
- **A second one-hander:** the server does not track dual wield. Rogue daggers and Demon Hunter warglaives are affected.

`ItemTemplate::allows` checks `AllowableRaces` against PlayableRaceBit (`Player::CanUseItem`, Player.cpp:11221). Before this fix, Haranir 86/91 could not equip their race-locked kit.

## Spells and bar

Known spells come from the class line and from every default skill line ([[class-progression]] "Default skill spells"). The create buttons are stored as the character's `ProgressionData` with spec 0.

Entering the world rewrites buttons through spell overrides, then removes buttons for unknown spells. TrinityCore keeps such buttons. Below level 10, these are removed:

- Pandaren Warrior buttons 11 and 83: the TDB rows are spell 77273, which is the Small Rice Cake item id.
- Blood Boil 50842 for every Death Knight: SpellLevel 17, but the class starts at level 8.
- Felsaber 200175 for Demon Hunters: nothing teaches it.

Characters that start at level 10 lose more buttons, because the server resolves them to the class's first spec. TrinityCore keeps the Initial spec (`GetDefaultChrSpecializationForClass`). The removed buttons include:

- the allied-race mounts on button 11
- Priest Fade, Psychic Scream and Desperate Prayer
- Shaman Flame Shock and Flametongue Weapon
- Warlock Health Funnel
- Death Knight Mind Freeze

## Not covered

- Skill values: the server stores no skills, only the spells the lines teach.
- `playercreateinfo_cast_spell` auras and pets.
- Start money.
- Class start zones ([[class-progression]]).

## Data upgrade

`scripts/add_player_create_info.py --db <copy>` adds the six tables and the kit `content_item` rows to an imported world.db. It needs these files:

- `CharacterLoadout.csv`, `CharacterLoadoutItem.csv`, `ItemEffect.csv` and `ItemXItemEffect.csv` under the DB2 CSV root.
- `playercreateinfo_action.sql` and `playercreateinfo_item.sql` under `data/reference/tdb1210/`.

The server refuses to start without these tables.

## Sources

- [player_start.rs](../../../crates/server/src/player_start.rs), [start tests](../../../crates/server/src/player_start_tests.rs), [ground regression](../../../crates/server/src/ground_tests.rs): authored starts, explicit exceptions, missing-row failure and Tauren gravity
- [import-player-starts.py](../../../scripts/import-player-starts.py): TDB1210 full-world SQL source; [player-height contract](../../specs/player-height.md)
- [player_create_info.rs](../../../crates/server/src/player_create_info.rs): loading, kit and creation
- [player_create_info_tests.rs](../../../crates/server/src/player_create_info_tests.rs): real world.db characters, exact slots, backpack and bar
- [inventory_model.rs](../../../crates/server/src/inventory_model.rs): `store_new_item_in_best_slots`, `equip_backpack_items`

## See Also

- [[class-progression]]: known spells, action bar, spec resolution
- [[inventory]]: bag and equipment model
- [[content-import]]: content_item and TDB tables
