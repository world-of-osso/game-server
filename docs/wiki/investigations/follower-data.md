# Retail follower data

Verified: 2026-10-11. Local Retail data proves follower dungeon identities and NPC appearances, but not a combat spell kit for any of the37 Expedition Escort templates. The [contract](../../specs/follower-dungeons.md) remains open: reference import is not working companion AI.

## Authored sources

The local install `.build.info` identifies wow build `dcfc90fffd79ba00406ae46f5f657592` as12.1.0.69933. The existing pinned CSV directory lacked LFGDungeons and companion/follower exports. Local `casc-local` extracted the following files into `game-engine/data/diagnostics/follower-ai-2026-10-11/db2/`, without Blizzard CDN requests:

| Source | FDID / layout | Finding |
|---|---|---|
| LFGDungeons |1361033 /34B02DE8 |1806 header records;27 difficulty205 entries |
| Difficulty.csv |Existing pinned CSV |205=`Follower`, min/max players5 |
| GarrFollower |949906 /1183F1B8 |985 records; no HordeCreatureID/AllianceCreatureID join to the37 escorts |
| GarrFollowerXAbility |968468 /11D80996 |Garrison ability references, not a dungeon NPC combat-spell mapping |
| GarrFollowerUICreature |1412636 /AED9E7F8 |Garrison presentation records |
| PlayerCompanionInfo |5922429 /55DA5E60 |Five seasonal Delves companion trait trees; not dungeon escort spell assignments |

CreatureSpellData3032554 is absent from this local CASC root. This is recorded once; it does not prevent catalog preparation. WoWDBDefs layout34B02DE8 pins the LFG field order/widths. The decoder's raw palette values can carry unused high bits: Ruby's raw map value8128985 must be truncated to uint16 map2521; role counts similarly use uint8. Do not classify a follower using a guessed flags mask: the pinned difficulty row names the mode.

The local TDB1210.26091 September9 archive contains9586 `creature_template_spell` rows, none for the37 templates whose subname is `Expedition Escort`. These templates also have empty AIName/ScriptName and zero local `tdb_smart_scripts` rows. Examples: Captain Garrick209057, Crenna Earth-Daughter209072, Meredy Huntswell209059, Austin Huxworth209065 and Shuja Grimaxe209053. Their authored model/difficulty rows are present. Names, `unit_class` (creature stat categories) and global SpellName rows do not authorize assigning a class/spec, rotation or player spell kit.

All13 requested class/spec dungeon kits therefore remain **unproved**, including the requested first tank/healer/DPS trio. Garrison mission followers cover many classes but cannot be substituted for dungeon NPCs. No role or spell coefficients were inferred.

## Dungeon selection and runtime boundary

`follower_import.py` emits all27 authored follower entries. Ruby Life Pools is a viable reference-data candidate: LFG2436/map2521 has306 `tdb_creature` rows in canonical world.db. The Rookery2653/map2648 has93; Algeth'ar Academy2464/map2526 has231. Full counts live in `catalog-report.json` in the diagnostics root.

Each has zero current `content_creature` rows. `world_data.rs` loads runtime `content_*`, not raw `tdb_*`; `retail_content.py` currently selects maps0,1,34,670. A reference spawn count is not a running dungeon. A later in-scope content-policy change must select an authentic dungeon, preserve phase/difficulty/tuning/model/loot data, and be applied by the lead. This importer does not promote spawns or create synthetic entrances.

The existing server LFG advances five-human role checks into match snapshots, without an NPC group/instance-entry pipeline. GroupRegistry uses character/player identities and player presence. Creature AI already provides navigation/chase/following; threat/taunt/healing threat and `Ctx::do_cast_at` are reusable. None supplies an absent authenticated kit.

## Prepare and apply (lead only)

Run from the server branch with `scripts/agent/agent-run follower-ai` from game-engine wrapping the command:

```text
python3 scripts/follower_import.py
  --lfg-db2 /home/osso/Projects/world-of-osso/game-engine/data/diagnostics/follower-ai-2026-10-11/db2/1361033.db2
  --world-sql /home/osso/Projects/world-of-osso/game-server/data/reference/tdb1210/TDB_full_world_1210.26091_2026_09_09.sql
  --world-db /home/osso/Projects/world-of-osso/game-server/data/world.db
  --output /home/osso/Projects/world-of-osso/game-engine/data/diagnostics/follower-ai-2026-10-11/follower-reference.sql
  --report /home/osso/Projects/world-of-osso/game-engine/data/diagnostics/follower-ai-2026-10-11/catalog-report.json
```

This prepares SQL only. `--world-db` opens SQLite `mode=ro`. The SQL atomically replaces only `follower_lfg_dungeons` and `tdb_creature_template_spell`, with source spell column names preserved; unrelated tables remain intact. It deliberately contains **no runtime follower profiles** or fabricated ability list. The JSON receipt records the LFG binary SHA256/layout, counts, reference/runtime spawn counts and candidate spell/SmartAI counts.

After inspecting the artifact, the lead may back up and apply:

```python
from contextlib import closing
from pathlib import Path
import sqlite3

world = Path("/home/osso/Projects/world-of-osso/game-server/data/world.db")
source = Path("/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/follower-ai-2026-10-11/follower-reference.sql")
with closing(sqlite3.connect(world)) as db:
    with closing(sqlite3.connect(world.with_name("world.db.pre-follower-20261011"))) as backup:
        db.backup(backup)
    db.executescript(source.read_text())
```

Applying reference SQL does not enable queueing/fighting. Agents must not run the apply snippet.

## Proof and gaps

Behavioral fixtures read the actual extracted DB2 and TDB dump and apply generated SQL only to an in-memory database. They assert Ruby's map/role composition,27 entries,9586 source assignments, concrete NPC48 spell rows, zero assignments for the first candidate trio, idempotent reapplication and preservation of unrelated state. Rejected-layout/missing-source and database-output collision cases publish no partial import. A read-only canonical audit confirms37 candidates, no assigned kits/SmartAI, and the reference/runtime spawn split.

Combat AI, NPC party membership, solo queue/instance entry, class/spec extensions and private both-skin fighting proof remain missing. No server/client was started, no UDP5000 was touched, and no posed screenshot was substituted for combat evidence.

## Sources

- [Importer](../../../scripts/follower_import.py), [behavioral tests](../../../scripts/tests/test_follower_import.py)
- Local canonical world.db opened read-only; local TDB `data/reference/tdb1210/TDB_full_world_1210.26091_2026_09_09.sql`
- Local pinned Retail `Difficulty.csv`; extracted DB2s and WoWDBDefs schema snapshots in the diagnostics path above
- WoWDBDefs `definitions/{LFGDungeons,GarrFollower,GarrFollowerXAbility,GarrFollowerUICreature,PlayerCompanionInfo}.dbd`
- Local Blizzard Retail `Blizzard_APIDocumentationGenerated/LFGInfoDocumentation.lua`: `IsLFGFollowerDungeon`/`IsInLFGFollowerDungeon`
- [Runtime world loader](../../../crates/server/src/world_data.rs), [Retail content policy](../../../scripts/retail_content.py), [LFG](../../../crates/server/src/lfg.rs), [groups](../../../crates/server/src/group/mod.rs)

## See Also

- [Content import](../systems/content-import.md)
- [Follower contract](../../specs/follower-dungeons.md)
- [Navigation](../../specs/navigation.md), [Creature aggro](../../specs/creature-aggro.md), [Groups](../../specs/groups.md)
