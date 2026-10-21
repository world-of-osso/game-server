# Boss-mod data import

verified: 2026-10-11

[Contract](../../specs/bossmod-data.md), [coverage and proof ledger](../../bossmod-data-coverage.md). The importer uses pinned `luaparser==4.0.0`, never evaluates addon Lua, and writes only a new SQLite backup copy. Each accepted rule retains addon/version/file/line provenance. Spell and SpellName must both be stamped12.1.0.69933; spell IDs must join both tables.

## Lead command

Choose new diagnostic output/report paths; input and addon roots are read-only.

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

Existing output and same-path writes are refused. SQLite's consistent backup API copies the input before new catalog tables are inserted. Canonical world data is never edited.

## Extraction boundary

The AST extractor supports understood engage/cast handlers, Retail branches, lexical literal bindings, arithmetic, table indexing, health/stage predicates and DBM spell dispatch/aliases. Custom DBM `NewTimer` reads spellId argument14, not its name argument2. Mutable counters/tables, unknown conditions/early returns and unreachable registered-spell branches remain gaps. Selected branch assignments update their outer binding without leaking local declarations; simultaneous assignments evaluate all RHS values first. Selected Lua method declarations use last-definition-wins, retaining the final declaration's lexical bindings. Only `OnEncounterStart` produces BigWigs Retail engage actions: current BossPrototype.lua dispatches legacy `OnEngage` only outside Retail.

Golden Serpent's current LittleWigs module replaces `OnBossEnable` with an empty Retail definition and consumes native timeline events from `OnEncounterStart`. Combining both definitions admitted obsolete legacy cast registrations; treating legacy `OnEngage` as Retail supplied an unauthorized fixed pull schedule. Neither the preserved legacy cooldowns nor the timeline duration-classification values authorize a Golden Retail schedule. Missing native timeline input stays an explicit gap.

`bossmod_encounter` stores identities. `bossmod_rule` stores only eligible runtime actions. `bossmod_static_source` separately retains audited timer expressions, literal lookup paths and original EXBoss native timeline configuration declarations. Runtime normalization is never executed. `bossmod_import_issue` and the encounter-grouped report retain explicit reasons for missing trigger/duration/locale inputs; `bossmod_import_meta` records the pinned build/parser.

Current DBM/BigWigs raid sources largely consume Blizzard timeline durations, not fixed initial/repeat cooldowns. EXBoss `firstSeenSec`/`cdSeriesSec` are absent; durationRules classify received native events. Raw defaults, lookup leaves and observed durations cannot substitute for those inputs. [Coverage](../../bossmod-data-coverage.md#actual-input-boundaries) records the outstanding whole-goal boundary.

## Locale text

BigWigs common enUS and module default-locale declarations, plus DBM enUS warning templates, are read statically. Locale guards select enUS, not whichever translation appears last. Resolvable formats substitute pinned SpellName rows. Unknown count/target arguments and unavailable enUS labels are rejected as warnings with reasons; neither a locale key nor an invented translation is transported.

BigWigs color→severity is the documented UI translation cyan/green/blue/yellow=Low, orange/purple=Medium, red=High. DBM special warnings/announces map to High/Low; these are UI translations, not authored numerical Blizzard enums. EXBoss numerical severity retains its native0/1/2 value.

## Runtime authority

`bossmod_authority` selects the sole addon with schedulable timers for an encounter. Conflicting schedules abort import rather than choosing an arbitrary priority. `bossmod_binding` joins declared map/NPC/encounter IDs against DB2 and local TDB spawns only in the copied database.

Server `scripted_ai::bossmod` loads that catalog, maintains independent alert countdowns and consumes central authoritative CombatLog CastStart/CastSuccess at the authored boundary. Explicit current-Retail boss-mod-Lua-backed scripts instead project their real EventMap requests through `scripted_alerts`: Kokia, Melidrussa, Kyrioss, Doragosa, Azureblade and Umbrelskul opt in through `CreatureAi::encounter_schedule`. Their actual repeat/cancel/dispatch state supplies snapshots; their independent imported clocks are bypassed, including stale legacy catalog rows. TDB SmartAI-only clocks remain excluded without a sourced mapping; no blanket Low warnings are emitted. Cast messages carry required rendered `warning_text`; the client renders that text verbatim with the validated spell icon. Lifecycle messages and snapshots retain the ordered, copy-scoped encounter channel.

## Scripted clock continuation — 2026-10-21

The current-Lua opt-in preserves addon warning colors, counted labels and enUS rename constants, sending warnings only at actual accepted cast admission (native CastStart for timed casts, CastSuccess for instant casts). Missing dynamic/native text remains absent: Kokia Ritual target-message text, Kyrioss444034 phase warning and Umbrelskul role/target-dependent Strike warning. This is not full addon option/role-filter parity.

Canonical world.db had no catalog tables; importer documentation/receipts prescribe private copies, not automatic publication. Retained2026-10-11 Kokia catalog uses legacy7.3/14.5/21.6s clocks; it must not overwrite current scripted8/19/28s requests. RED proves the missing actual Kokia snapshots; post-master2/2 behavioral tests pass (all six initial schedules plus real Kokia cast/count/severity/repeat/copy isolation/evade). Preliminary private5981 difficulty205 Kokia→native bars/Molten Boulder(1) warning works at1280×720; requested both-skin1920×1080 native acceptance remains pending.

## Zevrim root cause and native proof

Authentic Zevrim SmartAI rows use target5 (`SMART_TARGET_HOSTILE_RANDOM`). The old target resolver returned no targets, so it never attempted22478. Threat-list random resolution now respects distance/player/primary-power filters; the real-combat test proves admitted CastState22478, resolved effects, authored warning/reset, copy isolation and evade retirement. A cast-start warning is observed at admission, not inferred merely from completed-effect delivery.

[Current acceptance](../../bossmod-data-coverage.md#private-native-acceptance) records private5906 Modern/Forever Hydrospawn and Zevrim proof, eight inspected PNGs, post-master binary/data hashes, owned process cleanup and exclusions. Logged in on map0; entered instances by online port. No timing/warning injection or shared-realm operation occurred. Rendering/actor/balance completeness is not claimed.

## Development proof

```text
uv run --with luaparser==4.0.0 python -m unittest discover -s scripts/tests -p test_bossmod_data.py
```

Current targeted importer17/17 and server timeline7/7 pass; client model/view, actual UDP order/native geometry, check/fmt and the changed protocol suite pass. Independent verification is artifact/source inspection, not independent command execution. Exact counts, revisions and logs live in the [coverage ledger](../../bossmod-data-coverage.md#proof).

## Sources

- `scripts/bossmod_data.py`, `scripts/tests/test_bossmod_data.py` — static extraction/copy/locale contracts.
- `crates/server/src/scripted_ai/bossmod.rs`, `tests/data_bosses.rs`, `tests/bossmod_diremaul.sql` — real combat/copy boundary and authored fixtures.
- DBM `DBM-Core/modules/objects/Timer.lua:1192` — custom timer spellId parameter; source revisions and addon file/line receipts in [coverage](../../bossmod-data-coverage.md#reproducible-source-receipts).
- [Client warning/timeline contract](../../../../game-engine/docs/specs/encounter-timeline.md) — native presentation and shared wire boundary.
