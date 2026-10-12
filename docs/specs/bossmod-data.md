# Boss-mod alert data

Boss alerts use authored DBM, BigWigs/LittleWigs and EXBoss data, not inferred SmartAI cooldowns. Source Lua is parsed as data only. [Import and runtime notes](../wiki/systems/bossmod-data.md).

## What it must do

- [x] Preserve Retail initial and cast-triggered repeat delays, warning keys/severity, encounter/NPC identity and addon/version/file/line provenance.
- [x] Extract literal health thresholds and stage changes without inventing an initial timer.
- [x] Record unsupported conditions rather than executing Lua or treating them as unconditional.
- [x] Require Spell and SpellName rows stamped `12.1.0.69933`; reject unvalidated spell IDs.
- [x] Import into a new copy of the input world database, never modify the input or overwrite an existing output.
- [ ] Server bars follow imported schedules; actual cast events reset/advance timers using the authored event boundary.
- [ ] Boss warnings use the same source authority as timers; lifecycle, phase and health transitions clear/restart the appropriate timers.
- [ ] Private UDP5906 proof: at least two hosted encounters, Modern and Forever skins, addon-sourced bars and accepted-cast warnings.

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
- [ ] Complete server/client integration and native acceptance. Hosted Hydrospawn/Zevrim reconciliation test passes: real Zevrim CastState and completed effects, authored repeat delay and warning severity, isolated copy and evade retirement. Native live proof remains required; this branch is not ready.
- [ ] DBM/BigWigs extraction coverage beyond literal supported constructs.
- [x] Render statically resolvable enUS warning expressions with pinned SpellName substitution; transport/render exact warning_text. Unresolved dynamic format arguments or absent enUS strings remain per-encounter gaps.
- [ ] Complete independent verification; the available delegation backend cannot refresh its OAuth session.

## Out of scope

Canonical database writes and running addon Lua are prohibited by the feature goal.
