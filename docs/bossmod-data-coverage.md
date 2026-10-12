# Boss-mod data coverage

verified: 2026-10-11

[Contract](specs/bossmod-data.md), [import/runtime](wiki/systems/bossmod-data.md). Counts describe the current read-only addon roots, not every released addon package or complete encounter certification. No addon Lua executes; no SmartAI-derived alert cooldowns are substituted.

## Extraction

| Addon | Identified encounters | With schedulable timers | Audited timer calls | Literal lookup tables | Native input configs | Accepted rules | Issues |
|---|---:|---:|---:|---:|---:|---:|---:|
| DBM | 24 | 0 | 1064 | 4 | 0 | 5 | 1127 |
| BigWigs | 20 | 1 | 176 | 8 | 0 | 12 | 209 |
| LittleWigs | 446 | 343 | 3782 | 1 | 0 | 2785 | 3288 |
| EXBossData | 37 | 0 | 0 | 0 | 28 | 5 | 764 |

344 sole authorities and191 authenticated hosted bindings. Zero syntax/parse failures in the final import. Issues include unsupported conditions, unavailable enUS/dynamic warning arguments and absent pinned spell IDs; they are not all syntax errors. BigWigs gains Sentinels' authored1284588/46s engage bar. Identifying an encounter, auditing a call, or retaining a lookup table does not establish an executable schedule.

`bossmod_static_source` preserves call expressions, exact source lines, spell/phase/duration lookup paths and EXBoss native timeline declarations as inspectable JSON. These are not silently converted into engage/repeat rules. `bossmod_import_issue` and the report record reasons with addon/file/line; the report groups them by encounter.

## Actual input boundaries

- DBM's current raid/lair modules predominantly call `TLStart(timerExact,eventID,...)` from `ENCOUNTER_TIMELINE_EVENT_ADDED`. Constructor20.5s defaults are not initial/repeat schedules. Custom `NewTimer` uses explicit spellId argument14; its name argument2 is not a spell ID.
- BigWigs raid bars predominantly read `eventInfo.duration`. Gap tables indexed by received duration/stage require that authored input; the values are retained, not started arbitrarily.
- EXBoss `EncounterData.lua` has absent `firstSeenSec`/`cdSeriesSec` for its events. `FixedTimelineBosses.lua` supplies28 matching native-input configurations, not fixed pull clocks. Chinese labels without enUS declarations and dynamic count/target formats remain warning gaps, not invented translations.
- Parsed lexical constants, literal table lookups, arithmetic, Retail/enUS branches and DBM spell dispatch are supported. Mutable counters/tables, unknown early-return guards and unregistered/contradictory spell branches cannot become unconditional timers.

The whole broader schedule goal remains incomplete: DBM/EXBoss initial/repeat clocks need actual native timeline input or additional authored fixed schedules. The server currently has no producer for those native timeline events. Both-skin legacy hosted proof below does not close this boundary.

## Reproducible source receipts

| Addon | Revision/version |
|---|---|
| DBM | `31e47d6a23f4ee96a9ad65ff25ad4eb2b7bdf498` |
| BigWigs | `258d73479ac7d686187936127b70306ab831e775` |
| LittleWigs | `2777a9d51418934b5fdd5b92d937b1af6070bcad` |
| EXBossData | `1.0.0`, Lua-tree SHA256 `1a72ede6db0333a1300ee3029da933bb6332af2335b65faa6a6152dcd98507be` |

Spell and SpellName must both carry build12.1.0.69933 and every accepted spell ID must join both tables. Resolved warning text uses that SpellName row plus actual enUS/default locale text; unresolved expressions are reported and rejected as warnings.

## Proof

| Scope | Result | Evidence |
|---|---|---|
| Static importer | 17/17 PASS; copy preservation and non-execution, lexical/guard/locale regressions, correct custom DBM constructor | `continue-newtimer-green.log` |
| Server timeline | 7/7 PASS after master merge; real Zevrim CastState/effect, Hydrospawn effect, authored warning/reset, copy isolation/evade and no AI fallback | `postmerge-server-timeline.log` |
| Native model/view | 4 focused tests PASS | `continue-ui-green.log` |
| Actual UDP message order and native geometry | 1/1 each PASS | `postmerge-network-wire.log`, `postmerge-native-geometry.log` |
| Changed protocol | Full1729 PASS; check/fmt PASS | `continue-protocol-full.log`, `postmerge-protocol-check.log` |
| Server/client | Locked check and cargo fmt checks PASS | `postmerge-server-check.log`, `postmerge-engine-check.log`, proof ledger |
| Independent static verifier | Original eight counterexamples remediated or explicitly rejected; artifact/source inspection, no independent command execution | `verify-static-final.log` |

Diagnostics root: `/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/bossmod-data-2026-10-11/`. Final copy/report: `world-bossmod-final-current.db`, `coverage-final-current.json`. `proof-ledger-current.json` records revisions, commands, results and binary/catalog hashes. No full server/client suite ran; the protocol suite was explicitly required because its wire schema changed.

## Private native acceptance

UDP5906, fresh private redb, `fb_bossmod_data`, owned Bossmodern30/Bossforever31. Both logged in on map0 and used actual online world ports into Dire Maul429. Modern/Forever each show Hydrospawn344 addon bars plus Riptide/Massive Geyser warnings and Zevrim343 addon bars plus **Casting Intense Pain**. Real admitted casts22419/22421/22478 appear in server logs; no timing or warning events were injected. Repeated private revive calls sustained captures; these are alert proofs, not damage/balance/actor-art certification.

Eight inspected1280×720 PNGs: `/syncthing/AgentShared/2026-10-11/bossmod-data/`. `live/published-pngs-final.json` links each to current post-master samples and hashes. Logs/JSON remain in diagnostics, not AgentShared. The replay waits for completed map transitions before another port; an earlier one-second wait sampled map0 instead of the second encounter. Same-severity warnings replace each other by existing Retail behavior; overlapping Hydrospawn casts can briefly replace Zevrim's line.

All owned clients/private servers stopped; no UDP5906 listener remains. UDP5000, canonical world/player databases, master service and source addon trees were untouched. Consumer builds use private sibling protocol overrides only; tracked `[patch]` blocks, master's shared dependency lines and lockfiles remain intact.
