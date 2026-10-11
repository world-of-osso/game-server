# Data-driven SmartAI escorts

Creatures whose TDB template selects SmartAI and has source-0 script rows can use those rows even when legacy content names a C++ script. The generic private-copy importer changes only AIName and ScriptName. Unrelated templates, stats, spawns and quest relations remain unchanged. Source-9 lists alone do not select SmartAI. Repeated imports are idempotent; canonical world.db is refused.

## Lead instructions

Create a consistent SQLite backup of `data/world.db` in an owned diagnostics directory, then run:

```text
python3 scripts/import_smartai_overlay.py --db <private-copy>
```

Start the private server with that copy, fresh private redb, UDP5780 and fb_* credentials. Never apply this importer to canonical data during agent verification. Lead may integrate reviewed data changes separately.

## Runtime acceptance (pending)

Escort acceptance must execute authored paths, pauses, summons and event credit, including death/distance failure and waypoint-stop quest/despawn parameters. Timed lists honor combat gates, chance and nested-list semantics; waypoint movement honors delays, orientation and repeat. Both Retail and Forever require inspected accept/path/ambush/completion/turn-in evidence. Reference: TrinityCore a352b1fa SmartAI.cpp and SmartScript.cpp, archived under engine `data/diagnostics/smartai-coverage-2026-10-11/`.
