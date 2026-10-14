# Game objects

The gathering-nodes branch extends spawning with Retail type50 and authored pool/loot/skill/lifecycle metadata; [[gathering]] owns its implementation and incomplete native acceptance boundary.

World game objects from AzerothCore `gameobject` / `gameobject_template`. The import keeps a template only when it has no model or its `displayId` is in the Retail `GameObjectDisplayInfo` table; the result is `content_gameobject*` (see [content import](content-import.md)).

- **Spawning:** `game_objects::load_game_object_spawns` loads type 34 (`GAMEOBJECT_TYPE_GUILD_BANK`) type 19 (`GAMEOBJECT_TYPE_MAILBOX`, opens `NpcRole::Mailbox`) and type 23 (`GAMEOBJECT_TYPE_MEETINGSTONE`, casts its spell, see [[meeting-stones]]) of every map. Summoning portals (type 18) spawn at runtime in their caster's map copy. `spawn_continent_game_objects` (PostStartup) spawns maps 0 and 1; `spawn_map_game_objects` spawns a new dungeon copy's ([[zone-architecture]]).
- **Replicated components:** `Position` (Bevy space), `Rotation` (same facing convention as creatures) and `GameObjectInfo { entry, go_type, display_id, name, scale }`.
- **Visibility:** `NetworkVisibility` puts the objects under interest management.
- **Server-only component:** `GameObjectBounds` holds the Retail `GeoBox` model box and the facing.
- **Reach:** the player's WoW-space position is rotated into object space by −orientation and must lie inside the box scaled by the object size and grown by 5 yd. This is AzerothCore `IsAtInteractDistance`, but with the blizzlike 5 yd; AzerothCore itself uses 10.
- **No grid activation:** game objects have no per-tick systems, so they don't take part in it.
- **Adding a type:** widen the spawn query, then add native rendering/cursor admission and a use handler. Shared-protocol `GameObjectInfo::is_usable` is a historical type-list helper, not a runtime admission check.
## Quest objects (2026-10-11)

Type3 chests and Type10 goobers spawn with model boxes and server-only `QuestObject` metadata. Validated `UseGameObject` commands recheck life, map copy, box reach, authored lock alternatives, active quest (only the goober questID; chest questID is not a use gate) and reuse time. Goobers call Type2 credit, shared at group reward distance; chests roll `content_gameobject_loot_template` and open the existing loot window. Item credit follows actual Retail-source inventory counts, never opening alone. Negative spawn timers require scripted respawn; positive consumable timers block repeat-use until expiry. Activation/despawn art, profession/spell-key opening, scripted GO events/conditions and GO spell execution remain incomplete.

`game_object_locks.json` beside the resolved world.db is a required, immutable runtime input. Generate it from local CASC FDID1343608 with `python3 scripts/import_game_object_locks.py --db2 <Lock.db2> --world-db data/world.db`. The exporter pins WoWDBDefs Lock layoutDA418152 (12.1.0.69933), decodes all eight alternatives, and writes no world.db rows. Current local export has1996 locks including copies; all181 locks referenced by spawned chests/goobers resolve. Item-key alternatives require the key in Retail-source bags; ordinary interaction alternatives require zero skill. Profession, aura-specific and spell-key alternatives are refused, not bypassed. Missing files/layouts/lock definitions fail explicitly.

Chest loot is stored as `CorpseLoot` on the GO but is not a corpse: `drop_decayed_loot` must exclude `GameObjectInfo`, otherwise opening succeeds but cleanup deletes the slots before the following loot request. Current real-data test9548 (Clopper's Equipment181746 → item23830) reproduces that process-order boundary. It replaced the historical11049 fixture after Retail template185915's authored `Data1=0` invalidated the assumed egg loot; production loot rules and source data were not altered. [Fixture refresh](../../quest-objectives-coverage.md#fixture-refresh--2026-10-14) records the source rows and the separate missing spell-target-condition fixture resource.

- **Decoration:** `GAMEOBJECT_TYPE_GENERIC` (5) and `GAMEOBJECT_TYPE_SPELL_FOCUS` (8) spawn (fires, bonfires, lamps) but are not usable: the client gives them no `WorldObjectInteraction`, and a `UseGameObject` for one fails with `InvalidTarget`. `GO_FLAG_NOT_SELECTABLE` is not replicated: no spawned usable object carries it (historical measurement 2026-09-26; new quest-object flag/condition coverage is not claimed).

## Sources

- [Quest contract](../../specs/quests.md) and [loot contract](../../specs/loot.md).
- `game_objects.rs`, `game_objects/quest_object.rs`, `game_objects/locks.rs`, `quest_objectives_tests.rs`, `scripts/import_game_object_locks.py`.
- TrinityCore a352b1fa `GameObject::Use`, `Player::KillCreditGO`, `Spell::EffectKillCreditPersonal`/`EffectKillCredit`; WoWDBDefs `Lock.dbd` layoutDA418152 and local CASC1343608.

## See Also

- [[quests]] — counted objectives and spell-credit events
- [[content-import]] — captured GO/quest/loot data
- [[meeting-stones]] — existing object spell use
