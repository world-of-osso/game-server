# World-static AreaTriggers

TDB static area spawns are map-owned entities, distinct from `maps/area_triggers.rs` DB2 teleport volumes and spell-owned `SpellAreaTrigger`/aura395 fields. [Contract](../../specs/world-area-triggers.md).

## Source and lifecycle

TC a352b1fa `AreaTriggerDataStore.cpp398–506` joins spawn `(AreaTriggerCreatePropertiesId, IsCustom)` to create properties, whose `(AreaTriggerId, IsAreatriggerCustom)` independently selects the template. Static properties require no flags, curves, scale time or orbit movement. The source catalog rejects missing definitions and invalid static modes explicitly; no guessed geometry or replacement template is supplied. Installed TDB has291 static spawns with complete property/template joins.

`MapStore` loads the immutable catalog. Hosted continent creation and the shared transfer/login content queue create entities with their authored pose, template/property/custom identities, nullable visual-spell identity, shape data, source script priority and phase metadata. Script priority follows `AreaTrigger.cpp1073–1089`: nonempty spawn script, otherwise properties script. The visual spell is metadata, not a gameplay cast.

`AreaTrigger.cpp363–376` uses duration-1 with no caster. Static entities therefore have no aura/periodic lifetime; ordinary map-copy teardown owns their removal. The realm's existing `followers::population_difficulty` applies205→1 just as for creatures/game objects; this is project population policy, not a new TDB205 row or a TC claim. Source rows remain read-only.

## Behavioral proof

Four ignored RED tests atc6ace74c2 fail actual missing populations after successful normal entry: eleven concrete map0 spawns, Ruby login spawn20 and Orator barrier98 in Normal1/Follower205. The barrier's source keys are84/custom1→35803/custom0, sphere20, visual449122. Fixtures use actual NewWorld/ACK and the authored barrier pose, not a fabricated portal entrance. Tests also require occupied-copy isolation, native manager-driven empty-copy expiry and fresh reentry. GREEN pending.

## Boundaries

This slice establishes generic static object spawning and map ownership. Conversation unit-entry behavior follows separately. Phase metadata is retained; phase visibility/action filtering, generic template action execution and client visual rendering are not certified by this spawn gate. Orator's actual death-removal hook follows in its encounter integration, not this isolated generic branch.

## Sources

- TC a352b1fa `src/server/game/Globals/AreaTriggerDataStore.cpp` and `src/server/game/Entities/AreaTrigger/AreaTrigger.cpp`; pinned snapshots under engine `data/diagnostics/follower-dungeons-2026-10-21/sources/`.
- Installed read-only TDB1210 `tdb_areatrigger`, `tdb_areatrigger_create_properties`, `tdb_areatrigger_template`.
- `crates/server/src/world_area_triggers{.rs,/catalog.rs}`, `maps/{store,transfer,mod}.rs`.

## See Also

- [Map architecture](zone-architecture.md)
- [Spell-owned areas](spell-casting.md)
- [Orator encounter](boss-scripts.md#tww-city-of-threads-orator--2026-10-21)
