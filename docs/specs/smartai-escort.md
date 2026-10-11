# Data-driven SmartAI escorts

Creatures whose TDB template selects SmartAI and has source-0 script rows can use those rows even when legacy content names a C++ script. The generic private-copy importer changes only AIName and ScriptName. Unrelated templates, stats, spawns and quest relations remain unchanged. Source-9 lists alone do not select SmartAI. Repeated imports are idempotent; canonical world.db is refused.

## Lead instructions

Create a consistent SQLite backup of `data/world.db` in an owned diagnostics directory, then run:

```text
python3 scripts/import_smartai_overlay.py --db <private-copy> --tdb-world-sql data/reference/tdb1210/TDB_full_world_1210.26091_2026_09_09.sql
```

`--tdb-world-sql` imports the dump's exact creature_summon_groups rows through the existing MySQL tokenizer; malformed/missing group rows fail before mutation. It preserves escaped comments and all authored positions/types/lifetimes. Group import replaces only its TDB support table, never spawns or template stats. Template-only imports omit that explicit flag; SUMMON_CREATURE_GROUP needs the support table.

Start the private server with that copy, fresh private redb, UDP5780 and fb_* credentials. Never apply this importer to canonical data during agent verification. Lead may integrate reviewed data changes separately.

## Movement authority

Authored point movement must exclude idle wandering/patrol. An active escort retains movement ownership during pauses and node delays; finishing/stopping releases idle movement. An escort's point generator suspends while combat chase or evade-home owns movement; it must not advance the path concurrently. Other explicitly requested point movement remains unchanged. While walking its path, an escort updates its home to the last out-of-combat path position. Combat leash and evade return use that dynamic home without changing the original respawn position. Escort distance failure remains a separate acceptance boundary.

## Runtime acceptance (pending)

Escort acceptance must execute authored paths, pauses, summons and event credit, including death/distance failure and waypoint-stop quest/despawn parameters. Timed lists honor combat gates, chance and nested-list semantics; waypoint movement honors delays, orientation and repeat. Both Retail and Forever require inspected accept/path/ambush/completion/turn-in evidence. Reference: TrinityCore a352b1fa SmartAI.cpp and SmartScript.cpp, archived under engine `data/diagnostics/smartai-coverage-2026-10-11/`.
