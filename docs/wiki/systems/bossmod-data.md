# Boss-mod data import

verified: 2026-10-11

[Contract](../../specs/bossmod-data.md). The importer parses Lua ASTs with pinned `luaparser==4.0.0` (Python tooling dependency, managed by uv) and never evaluates addon code. Source roots are read-only. Git SHA identifies cloned sources; installed sources carry their TOC version plus SHA256 of all Lua files.

## Lead command

Run from the feature server checkout. Choose **new** diagnostic output/report paths; input is opened read-only and backed up with SQLite's consistent backup API.

```text
uv run scripts/bossmod_data.py \
  --db /home/osso/Projects/world-of-osso/game-server/data/world.db \
  --output /path/to/private/world-bossmod.db \
  --dbm /home/osso/Repos/DBM-Retail \
  --bigwigs /home/osso/Repos/BigWigs \
  --littlewigs /home/osso/Repos/LittleWigs \
  --exboss-data '/syncthing/World of Warcraft/_retail_/Interface/AddOns/EXBossData' \
  --report /path/to/private/coverage.json
```

`bossmod_encounter` retains identities, `bossmod_rule` retains action/event/provenance rows, `bossmod_import_issue` retains underivable constructs, and `bossmod_import_meta` records the pinned build/parser. Validation requires both `db2_build` stamps and an ID join between `spell` and `spell_name`. Unknown IDs become report gaps and are not inserted.

## Supported data

Literal BigWigs/LittleWigs `OnEngage` bars; `SPELL_CAST_START`/`SPELL_CAST_SUCCESS` registered handlers; Retail-vs-Classic branch selection; literal stage/health conditions; simple source-creature predicates. Unknown conditions, role/target warnings, duration expressions and else-stage inference remain gaps. Warning color translates to built-in severity: red=High, orange/purple=Medium, cyan/green/blue/yellow=Low. This is a documented UI translation, not an authored numerical BigWigs enum. Warning text expressions remain inspectable keys, not executed format strings.

Literal DBM timer constructors/start calls and spell-dispatch warnings are parsed where their event/condition is known. EXBoss EncounterData supplies spell-bound warning text and native numerical severity. `durationRules` classifies already-existing Blizzard events; those durations are **not** initial/repeat cooldowns. Current EXBoss firstSeenSec/cdSeriesSec are absent. Current raid modules largely depend on Blizzard timeline callbacks; no fixed schedule is claimed for them.

## Development proof

```text
uv run --with luaparser==4.0.0 python -m unittest discover -s scripts/tests -p test_bossmod_data.py
```

The report separates encounters identified from encounters with timers. Identifying an encounter or importing warnings does not certify its schedule. Runtime/live gates remain open.
