# Data-driven SmartAI coverage

Interpret TDB `smart_scripts` rather than hand-written quest scripts. Semantics follow TrinityCore a352b1fa `SmartScript.cpp`, `SmartScriptMgr.h` and `SmartAI.cpp`. Current implementation is incomplete; see the [boss-script architecture](../wiki/systems/boss-scripts.md).

## What it must do

- [x] Quest-accept rows invoke timed action lists, store targets, remove questgiver flags and start the authored TDB waypoint path (Erland 1978 / quest 435 / path 15825).
- [x] Advance authored waypoint nodes; execute reached events and pause/resume without skipping the next node.
- [x] Support WP_STOP, waypoint paused/resumed/stopped/ended events and quest accept/reward hooks.
- [x] Support tested summon/follow, faction/NPC flag changes, quest completion, despawn and escort failure on death/distance subsets. Feero4484 additionally exercises counters63/event77, immunity145 and attack49; [Alliance escort proof and native gaps](escort-quests.md). This is not full SmartAI parity.
- [x] Report unsupported action/event/target types once per type, including timed lists.
- [x] Broaden support by script-row and quest-linked impact rankings (orientation66/emote17 and summon hooks; [census](../smartai-census.md)).
- [x] Prove a complete private UDP 5780 legacy escort with both client skins and inspected PNGs; exact authentic source-fixture boundary in [census](../smartai-census.md#live-proof-boundary).

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
- [x] Full quest863 GUI flow is proved for the authentic legacy private fixture in both skins.
- [ ] Mixed Retail-scaling/legacy-escort source reconciliation remains unproved; never turn that data drift into invented actor stats or parser behavior.

## Out of scope

- Canonical database writes, master merge and UDP 5000 changes are prohibited.
- New client protocol messages only if required by observable behavior.
