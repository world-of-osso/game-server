# Game objects

World game objects from AzerothCore `gameobject` / `gameobject_template`. The import keeps a template only when it has no model or its `displayId` is in the Retail `GameObjectDisplayInfo` table; the result is `content_gameobject*` (see [content import](content-import.md)).

- **Spawning:** `game_objects::load_game_object_spawns` loads type 34 (`GAMEOBJECT_TYPE_GUILD_BANK`) on map 0. `spawn_game_objects` (PostStartup) spawns each one.
- **Replicated components:** `Position` (Bevy space), `Rotation` (same facing convention as creatures) and `GameObjectInfo { entry, go_type, display_id, name, scale }`.
- **Visibility:** `NetworkVisibility` puts the objects under interest management.
- **Server-only component:** `GameObjectBounds` holds the Retail `GeoBox` model box and the facing.
- **Reach:** the player's WoW-space position is rotated into object space by −orientation and must lie inside the box scaled by the object size and grown by 5 yd. This is AzerothCore `IsAtInteractDistance`, but with the blizzlike 5 yd; AzerothCore itself uses 10.
- **No grid activation:** game objects have no per-tick systems, so they don't take part in it.
- **Adding a type:** another type (mailbox, chest) needs its spawn query widened, plus a role or handler in `npc_interaction::usable_object`.
