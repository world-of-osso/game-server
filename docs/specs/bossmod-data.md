# Boss-mod alert data

Boss alerts use authored DBM, BigWigs/LittleWigs and EXBoss data, not inferred SmartAI cooldowns. Explicit current-Retail boss-mod-Lua-backed scripted encounters (Kokia, Melidrussa, Kyrioss, Doragosa, Azureblade, Umbrelskul) publish selected EventMap requests rather than independent imported clocks; source warning colors/text remain authoritative. TDB SmartAI-only timers stay excluded without explicit sourced mapping. Source Lua is parsed as data only. [Import/runtime](../wiki/systems/bossmod-data.md), [current coverage and proof](../bossmod-data-coverage.md).

## What it must do

- [x] Preserve Retail initial and cast-triggered repeat delays, warning keys/severity, encounter/NPC identity and addon/version/file/line provenance. BigWigs Retail engages through `OnEncounterStart`, never legacy `OnEngage`; the last selected Lua method definition replaces earlier bodies and registrations.
- [x] Extract literal health thresholds and stage changes without inventing an initial timer.
- [x] Record unsupported conditions rather than executing Lua or treating them as unconditional.
- [x] Require Spell and SpellName rows stamped `12.1.0.69933`; reject unvalidated spell IDs.
- [x] Import into a new copy of the input world database, never modify the input or overwrite an existing output.
- [x] Server bars follow eligible imported schedules; real cast events reset/advance timers at the authored boundary (Hydrospawn/Zevrim acceptance).
- [ ] Boss warnings use the same source authority as timers; lifecycle, phase and health transitions clear/restart the appropriate timers.
- [x] Private UDP5906 proof: Hydrospawn344/Zevrim343, Modern/Forever, addon-sourced bars and real accepted-cast warnings; eight inspected PNGs.

## How it works

- [Static extraction and copy-only lead command](../wiki/systems/bossmod-data.md).

## Implementation inventory

- `scripts/bossmod_data.py`: non-executing AST extraction, pinned spell validation and SQLite copy import.
- `scripts/tests/test_bossmod_data.py`: concrete Lua fixtures and database preservation tests.
- `crates/server/src/scripted_ai/bossmod.rs`: imported authority catalog, independent alert clocks and actual cast/health event reconciliation.
- `crates/server/src/scripted_ai/tests/bossmod_diremaul.sql`: source-pinned Hydrospawn/Zevrim schedule fixtures.
- `crates/server/src/scripted_ai/tests/data_bosses.rs`: real creature/combat/copy-isolation acceptance tests.

## Tests asserting this spec

- `scripts/tests/test_bossmod_data.py`.
- `crates/server/src/scripted_ai/bossmod.rs` scheduler tests.
- `crates/server/src/scripted_ai/tests/data_bosses.rs` imported encounter reconciliation test.

## Known gaps (current cycle)

- [ ] Dynamic Blizzard timeline callbacks, observed duration classification and variable/count-indexed timer arrays need their real triggering data; they cannot be substituted with invented intervals.
- [x] Eligible literal-rule server/client integration and native acceptance. Real Zevrim CastState/effects, authored resets/rendered text, isolated copies and evade retirement pass; both-skin native proof is recorded in the coverage ledger.
- [ ] Complete DBM/BigWigs/EXBoss schedules. Static calls/lookups/native declarations and per-encounter reasons are retained; missing native timeline input cannot become a fabricated pull/repeat clock.
- [x] Render statically resolvable enUS warning expressions with pinned SpellName substitution; transport/render exact warning_text. Unresolved dynamic format arguments or absent enUS strings remain per-encounter gaps.
- [x] Independent static artifact/source verification of eight unsafe extraction cases; regressions pass. This is not independent command execution or full-encounter certification.

## Out of scope

Canonical database writes and running addon Lua are prohibited by the feature goal.
