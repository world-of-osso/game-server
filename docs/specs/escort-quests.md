# Alliance escort quests

Extend the existing [SmartAI escort interpreter](smartai-escort.md), not a parallel quest-specific runtime. Acceptance target: Feero Ironhand4484, Supplies to Auberdine976, path35873 (48 nodes), selected from TDB1210 rows in the read-only supplied world.db.

## Contract

- Quest acceptance runs authored list448400, removes questgiver flags and NPC immunity, sets faction774, speaks group0 after1000ms and starts path after3000ms more.
- Waypoints19/27/43 pause and summon groups0/1/2: four Dark Strand Assassins3879; three Forsaken Scouts3893; Aligar3898, Balizar3899 and Caedakar3900. Positions, lifetimes, spells and texts come from TDB, never invented fixtures.
- Ambusher death actions63 add to the nearest same-copy Feero's counters1/2/3. Counter events77 require exact values4/3/3, with source cooldown/reset semantics. Earlier kills never resume the path. Lists448402/448404/448406 provide authored speech/delays/resume; the last credits976. Death fails976. Waypoint48 despawns after2000ms.
- Native Modern and Forever clients show walking, authored speech, ambushes and earned completion for a real private account on UDP5931; inspected1920x1080 PNGs are required. Server unit tests alone do not establish this acceptance.

## Coverage / gaps

| Boundary | State | Proof |
|---|---|---|
| Existing path/pause/summon/quest machinery | Reused | Previous quest863 proof, not Feero acceptance |
| Counter63/event77 cross-creature death flow | Server GREEN | Real four-death gate plus all three waves/credit |
| Attack-start49, immunity145 | Server GREEN | RED2→GREEN2 at appearance/acceptance hooks |
| Emote5 | Named cosmetic gap | Authored second-camp roar15 not yet delivered to client |
| Both-skin native complete/death run | Pending | No evidence yet |

No canonical DB writes, guessed missing actions, source replacement, master push or unrelated UI redesign. Other unsupported SmartAI actions/events remain named gaps; this is not full SmartAI parity.

## Sources

Read-only `/home/osso/Projects/world-of-osso/game-engine/data/skyborn-handoff/2026-10-07-c1cdf6e/server/world.db`: tdb_smart_scripts, tdb_waypoint_path/node, tdb_creature_summon_groups, tdb_creature_template, content_creature_text. Supplied DB includes all ten ambushers.

Pinned URLs and SHA-256 (retrieved2026-10-15; archived in engine data/diagnostics/escort-quests-2026-10-15/sources):

- https://raw.githubusercontent.com/TrinityCore/TrinityCore/a352b1fa/src/server/game/AI/SmartScripts/SmartScript.cpp — `957a0a95fb083039b08485ea89f83f2f0c31931c8a8acaf4d2a6a62bbfc6b3b4`
- https://raw.githubusercontent.com/TrinityCore/TrinityCore/a352b1fa/src/server/game/AI/SmartScripts/SmartAI.cpp — `ee01874e762becccf07c0be1e02fee6eb3927c6fa0b6b040e9bdf88a2af9380f`
- https://raw.githubusercontent.com/TrinityCore/TrinityCore/a352b1fa/src/server/game/AI/SmartScripts/SmartScriptMgr.h — `0488b4831b643e9a6ec3a48f55b77440357c265df891ec31ee93ffb8a3dd268e`

## Private live setup boundary

Supplied content_creature omits4484 although creature contains authentic spawn32783. Live fixture is a consistent SQLite backup and adds that exact creature row to its content table; canonical/source DB remains read-only. No template stats, quest rules or scripts change. Explicit ignored setup test creates two real fb_escort accounts with ordinary starting kits, level30 Night Elf druids at that source spawn. It seeds only authored prerequisite973 as rewarded;976 remains unaccepted. This setup does not prove prerequisite progression or mixed-Retail population reconciliation.
