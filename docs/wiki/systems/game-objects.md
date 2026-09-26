# Game objects

World game objects from AzerothCore `gameobject` / `gameobject_template`. The import keeps a template only when it has no model or its `displayId` is in the Retail `GameObjectDisplayInfo` table; the result is `content_gameobject*` (see [content import](content-import.md)).

- **Spawning:** `game_objects::load_game_object_spawns` loads type 34 (`GAMEOBJECT_TYPE_GUILD_BANK`) type 19 (`GAMEOBJECT_TYPE_MAILBOX`, opens `NpcRole::Mailbox`) and type 23 (`GAMEOBJECT_TYPE_MEETINGSTONE`, casts its spell, see [[meeting-stones]]) of every map. Summoning portals (type 18) spawn at runtime in their caster's map copy. `spawn_continent_game_objects` (PostStartup) spawns maps 0 and 1; `spawn_map_game_objects` spawns a new dungeon copy's ([[zone-architecture]]).
- **Replicated components:** `Position` (Bevy space), `Rotation` (same facing convention as creatures) and `GameObjectInfo { entry, go_type, display_id, name, scale }`.
- **Visibility:** `NetworkVisibility` puts the objects under interest management.
- **Server-only component:** `GameObjectBounds` holds the Retail `GeoBox` model box and the facing.
- **Reach:** the player's WoW-space position is rotated into object space by −orientation and must lie inside the box scaled by the object size and grown by 5 yd. This is AzerothCore `IsAtInteractDistance`, but with the blizzlike 5 yd; AzerothCore itself uses 10.
- **No grid activation:** game objects have no per-tick systems, so they don't take part in it.
- **Adding a type:** another type (mailbox, chest) needs its spawn query widened, plus a role or handler in `npc_interaction::usable_object`.
