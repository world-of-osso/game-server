# GO_NOT_SELECTABLE_FLAG_UNMODELLED

Native GameObject addon flag `0x10` is not represented or consumed by the live server/client interaction path. Fel Iron chest `MakeActive` is blocked on this global flag-semantics gap. No chest, protocol, client or flag-consumer implementation was attempted.

## Read-only census

Snapshot: local TDB1210 and promoted `content_*` tables, queried read-only during the encounter-coverage investigation.

| Source | Placements | Template entries | Maps |
|---|---:|---:|---:|
| TDB placements whose template-addon flags contain0x10 | 1,581 | 1,008 | 103 |
| Promoted content placements with that flag | 1,314 | 984 | 96 |

1,306 promoted placements match TDB by GUID+20000000, entry and map. All1,314 census placements have a matching display row. These are authored placements, not a claim that every map currently has a live server copy.

The complete attached census is `game-engine/data/diagnostics/encounter-coverage-2026-10-12/go-not-selectable-content-census.json` (original evidence checkout: `/home/osso/.worktrees/game-engine-s10/`). It records every promoted flagged spawn's GUID, entry, name, type, map, difficulties, flags, script and relevant use fields; it is not staged from the separate data/cache tree.

- Chests/type3:61 placements/50 entries; goobers/type10:345/234. All406 reach the existing quest-object use route, subject to its ordinary life/copy/reach/lock/quest/cooldown checks. This is a route census, not406 verified successful uses or quest completions.
- Doors/type0:492 placements/358 entries. Generic door use is already refused by the current player interaction dispatcher; loading flags alone would not implement doors.
- Buttons/type1:63 placements/63 entries. Two are implemented Isle keep banners; the flagged set has no registered Serpentine Seal script. Other buttons are not admitted merely because a TDB script name exists.
- Other admitted-type candidates include chair181640 and ritual187359; runtime eligibility still requires their existing consumer data. Remaining types are not evidence of working player use.

## Concrete existing-content hazard

Isle of Conquest/map628 banners195393 (Horde Keep, contentGUID20166879) and195396 (Alliance Keep,20166899) both have flags48 (`0x30`, including NOT_SELECTABLE). Shared objective rules refuse them before their keep is breached and allow the opposing team afterward. The server regression `isle_channel_source_demolition_breaches_gate_and_releases_keep_banner` performs the real195393 use/eight-second capture after breach.

Existing breach handling changes gate/keep-door collision and GO state, not native banner flags: those flags are currently unmodelled. A new unconditional server refusal/client selectability bit populated directly from addon flags would therefore keep these working objectives unusable after breach unless their unlock consumer is ported too. This conclusion joins concrete data, current code and an existing behavioral regression; no fresh battleground test was run as part of this read-only census.

The affected set is not small or uniformly inert. Global loading/consumption must be treated as a separate project with existing unlock consumers and regression coverage, not silently added to the chest slice.

## Blocked Fel Iron chest boundary

Authoritative TDB spawns are185168 in difficulties1,2 and185169 only in1; preserve both actual Normal placements, not the legacy one-chest-per-difficulty mapping. Both addon flags are16. TC maps both entries to the same instance object-data slot, whose last actual registration wins.

TC `MakeActive` only removes `GO_FLAG_NOT_SELECTABLE`; it does not set GO state ACTIVE, open loot, disable collision or respawn the object. Clearing an unconsumed flag is not observable activation. The chest remains blocked until `GO_NOT_SELECTABLE_FLAG_UNMODELLED` is addressed with its server/client consumers. Loot, persistence and broader encounter work remain separate.

## Sources

- [GameObject loading/state](../../../crates/server/src/game_objects.rs) and [player-use dispatch](../../../crates/server/src/npc_interaction.rs).
- [Quest-object consumer](../../../crates/server/src/game_objects/quest_object.rs).
- [Existing Isle behavioral test](../../../crates/server/src/maps/tests_battleground_nodes.rs) and [breach consumer](../../../crates/server/src/pvp_isle_siege.rs).
- Pinned TC `a352b1fa`: cached `GameObject.cpp2497–2499`, `SharedDefines.h3233–3239`, `InstanceScript.cpp323–338`, and Ramparts instance34–38/89–98 under the evidence directory's `tc/` tree.
- [Herald contract](../../specs/ramparts-herald.md).

## See Also

- [[boss-scripts]] — bounded Ramparts implementation and remaining encounter gaps.
- [[game-objects]] — live GameObject representation and interaction consumers.
