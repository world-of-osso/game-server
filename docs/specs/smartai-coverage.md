# Data-driven SmartAI coverage

Interpret TDB `smart_scripts` rather than hand-written quest scripts. Semantics follow TrinityCore a352b1fa `SmartScript.cpp`, `SmartScriptMgr.h` and `SmartAI.cpp`. Current implementation is incomplete; see the [boss-script architecture](../wiki/systems/boss-scripts.md).

## What it must do

- [x] Quest-accept rows invoke timed action lists, store targets, remove questgiver flags and start the authored TDB waypoint path (Erland 1978 / quest 435 / path 15825).
- [x] Advance authored waypoint nodes; execute reached events and pause/resume without skipping the next node.
- [x] Support WP_STOP, waypoint paused/resumed/stopped/ended events and quest accept/reward hooks.
- [ ] Support summon, follow, faction/NPC flag changes, quest completion, despawn and escort failure on death/distance.
- [x] Report unsupported action/event/target types once per type, including timed lists.
- [ ] Broaden support by script-row and quest-linked impact rankings.
- [ ] Prove a complete private UDP 5780 escort with both client skins and inspected PNGs.

## How it works

- [Creature script architecture](../wiki/systems/boss-scripts.md).

## Ranked continuation

SET_ORIENTATION66 (1,210 rows/740 quest-linked) and SET_EMOTE_STATE17 (788/376) extend supported actions. Orientation affects the actor: SELF restores home facing, POSITION sets its authored facing, selected unit faces the actor toward that unit. Emote state affects selected units and preserves stance/sheath. Counts indicate interpreter admissions, not quest completion or unsupported-target coverage.

## Implementation inventory

- `crates/server/src/scripted_ai/smart_ai.rs`: event/action interpreter and timed lists.
- `crates/server/src/scripted_ai/smart_paths.rs`: read-only TDB waypoint loading.
- `crates/server/src/scripted_ai/smart_escort_tests.rs`: authentic minimal SQLite fixtures.

## Tests asserting this spec

- `crates/server/src/scripted_ai/smart_escort_tests.rs`.

## Known gaps (current cycle)

- [x] Canonical content Erland selects an unported C++ ScriptName; TDB selects SmartAI. Prepare authentic template overlay on a database copy, plus importer instructions; never alter canonical data.
- [ ] Full escort and live proof remain incomplete.

## Out of scope

- Canonical database writes, master merge and UDP 5000 changes are prohibited.
- New client protocol messages only if required by observable behavior.
