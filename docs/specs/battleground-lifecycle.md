# Battleground lifecycle

## Contract

Finish one Retail Mainline match: queue → invite/accept → isolated map entry → map objective → scoreboard → sourced honor → return to origin. Extend `pvp.rs` and shared `battleground*.rs`; Modern and Forever share behavior. No synthetic identifiers, timings or rewards; no agent world.db writes.

## Coverage expansion — active 2026-10-12

- [ ] Extend the complete lifecycle to random/epic Arathi Basin, Battle for Gilneas, Twin Peaks, Eye of the Storm, Temple of Kotmogu, Silvershard Mines, Seething Shore, Deephaul Ravine, Alterac Valley and Isle of Conquest. Source-backed objectives, WorldState IDs, victory rules, map-specific scoreboard columns and end/leave are required per map.
- [ ] Deepwind Gorge remains a source-content gap: local Map2245/WDT2992913 exists, but TDB1210 has zero creature/gameobject placements, no battleground_template1037/1039, and pinned TC has no Deepwind script. Do not invent placements or substitute an older layout.
- [ ] Behavioral server proof and private real-UDP Modern/Forever native proof per implemented map. Lightweight NetworkBridge peers may fill the sourced minimum roster. Travel acceleration must preserve authored positions; never inject objective scores/outcomes.

Sources: local BattlemasterList FDID1360265/layout3414CBD7 and Map CSV12.1.0.69933; read-only TDB1210 catalogs; local CASC WDT extraction; TC a352b1fa9017138cc2f1dcfc0a51afaa17391ecf. BattlemasterList's current layout has no MapID field; association is verified through TDB templates and their safe-location maps. Resource/flag IDs must retain this distinction. WorldState FDID1374383 is absent from the local CASC root; TDB world_state and pinned TC references remain available, but independent DB2 WorldState row verification is a recorded gap.

Current proof/source ledger: `/home/osso/.worktrees/handoff-bg-coverage.md`; durable receipts in engine `data/diagnostics/bg-coverage-2026-10-12/`. This section supersedes WSG-only scope, not previously proved WSG behavior.

## Selection and source evidence — 2026-10-11

Mainline Warsong Gulch map2106 is the smallest local source-backed candidate: TDB1210 has28 creatures,14 gameobjects and2 capture area triggers. AB2107 has308/36/4; IoC628 has242/158; AV30 has1138/265. Classic maps489/529 have no source spawn rows in this archive. Counts are source coverage, not live proof or all selectable battleground modes.

world.db already contains WSG source creatures/objects in `tdb_creature`/`tdb_gameobject`. The content policy excludes2106, so runtime `content_*` is empty. Promotion must use that existing pipeline, preserving authored difficulty0, coordinates, templates, models and its GUID mapping; never overlay fabricated positions.

Authentic WSG details:
- TDB `battleground_template`1014 uses Alliance/Horde safe locations7051/7050 (already present in `tdb_world_safe_locs`).
- Flag gameobjects227741/227740, GUID7000010/7000009: Alliance `(1540.45,1481.22,353.577)`, Horde `(917.311,1434,346.134)`. Source templates have type36, displays55436/55437 and spells156621/156618.
- Capture triggers35/36 reference custom create-properties30/31, sphere radius5, same authored flag centers, script `areatrigger_action_capture_flag`.
- `world_state`17303 has DefaultValue3 and MapIDs `489,726,2106`: maximum captures. Timer states4247/4248 default0 are not evidence of a duration.
- `battleground_scripts` assigns `battleground_warsong_gulch` to map2106.

Pinned DB2 directory has `GameObjectDisplayInfo.csv`, not `GameObjects.csv`. TDB provides the actual missing placements. Numeric capture/drop/invite/start/exit timers and honor policy remain gaps unless sourced separately. Existing shared synthetic template IDs/durations/rewards do not establish Retail authority.

## Lead-owned application

After merging the server branch, lead backs up world.db and applies:

```
python3 scripts/import-battlegrounds.py --dump data/reference/tdb1210/TDB_full_world_1210.26091_2026_09_09.sql --db data/world.db
python3 scripts/content_policy.py --db data/world.db
```

The importer transactionally imports verbatim `tdb_battleground_template`, `tdb_battleground_scripts`, `tdb_spawn_group`, `tdb_spawn_group_template`, `tdb_world_state` and area-trigger catalogs. It does not replace spawn or template tables, invent runtime rows, run scripts, or rebuild content. Existing incompatible metadata schemas fail explicitly. Missing source tables are reported. Rebuilding the existing content policy promotes map2106; validate counts/coordinates/templates after policy filtering. No user realm restart is authorized by this spec.

## Proof boundary

Implemented: authored AB1500-resource correction (shared), multiplayer server queue/score unit regression, WSG source selection/audit, metadata importer and WSG content-policy inclusion. Matched rosters now remain server-owned and deliver typed invitation snapshots through existing PvP updates. Responses validate match identity, membership and Waiting phase; accept records that player, decline removes only that player, stale/outsider/duplicate replies fail. Acceptance is entry-pending, not InProgress. Neither BG nor rated queue can overwrite a pending invitation. Expiry is unsourced and absent, not fabricated. Acceptance now creates one roster-reserved BG copy and uses existing NewWorld/WorldPortAck transfer. Both teams enter their authored safe locations; arbitrary portals/outsiders cannot enter, dungeon group/lock bindings do not determine BG copies, and original positions are retained server-side. No invented copy expiry is used. LeaveBattleground validates the current match, accepted membership and arrived map; it returns through the existing transfer/ack to each retained original position, clears that participant and releases the empty match copy after the last leave. No early-leave penalty or automatic-return timer is invented; those policies remain gaps. The existing UseGameObject consumer routes authored WSG type36 flags to the match controller. Opposing flag pickup, own dropped-flag return, carrier-owned capture within authored radius5 and own-flag-at-base validation use shared FlagObjective/BgInstance. Three captures end the match and expose roster/flag counters/outcome in the PvP snapshot; no legacy honor/marks grant is called. Roster readiness starts this untimed prototype; preparation/match/reset/drop timers remain explicit parity gaps. Native popup/scoreboard and honor rewards remain missing. Combat counters are initialized but not yet instrumented; do not claim combat-score parity. Full channel objective proof is pending at this revision. No live clients/servers or PNG proof yet.
