# Spell-area data rollout

The server requires `tdb_spell_area` before startup. Existing canonical `world.db` had no such table on 2026-10-11. This branch never writes it.

## Lead-owned incremental import

Use the already extracted authentic TDB1210.26091 excerpt:
`/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/invis-detection-2026-10-11/spell_area.sql` (968 rows).
It came from local `data/reference/tdb1210/TDB_full_world_1210.26091_2026_09_09.sql`, not a fabricated detector table. Save it as `data/reference/tdb1210/spell_area.sql` for future full imports.

Back up canonical SQLite with its backup API, then from the server checkout run:

```text
python3 scripts/mysql_dump.py --db data/world.db --schema scripts/spell_area_schema.sql --prefix tdb_ spell_area=data/reference/tdb1210/spell_area.sql
```

Only `tdb_spell_area` is replaced; do not use `tdb_schema.sql` for this incremental import (it replaces unrelated tables). The existing importer performs schema+rows in one transaction and rolls back failures. Verify `SELECT COUNT(*) FROM tdb_spell_area` gives968 and the100616 row retains area5171, quest_end29401, flags3, end-status mask43. Full `import-azerothcore.sh` now requires the excerpt and imports this table separately through the same safe schema.

Jaga has89302/type35/value100; matching100616/type35/value1000 applies in Valley of Wisdom5171 while quest29401 is none/incomplete/complete, not rewarded. Both native skins have real loot/credit/turn-in and post-reward disappearance proof. Vin also has89302, but this TDB has no spell_area row for quest29412 or Stormwind's area; his live eligibility remains unproven. Rendan82343/type12/value100 is detected by82336/type12/value1000 in tests, but this source catalog has no authored spell_area/quest start/reward/SmartAI route for82336. Do not add guessed rows or grant the detector manually to claim completion.

Other ledger blockers: current content already has Core Fragment20043133/map230 difficulty1 (the old0-difficulty observation is stale); a real-catalog copy-spawn/interest regression covers this row's replication within its normal dungeon copy; map530 is outside `HOSTED_CONTINENTS`. No dungeon/world hosting expansion is included here.
