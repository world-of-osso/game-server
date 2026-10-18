# Sourced jump-charge movement

Spell effect254 must use the actual server `jump_charge_params` referenced by its effect, initially covering Telash387981/388082 and source713/714. Authority is pinned TDB1210 server SQL and TC a352b1fa `ObjectMgr::LoadJumpChargeParams`, `Spell::EffectJumpCharge` and `MotionMaster::MoveJump`, not a client DB2. [Source/proof boundaries](../wiki/investigations/follower-data.md#telash-first-energy-phase--2026-10-17).

## What it must do

### Producer
- [x] Preserve the exact source table's columns, row values and NULLs in `tdb_jump_charge_params`; no invented speed, height, visual, curve or trigger values.
- [x] Preparation accepts independent copies only, is idempotent, refuses malformed source atomically and changes no unrelated tables, actors, placements or difficulty rows.
- [x] Fresh-import schema/table registration retains the same source table. COPY receipts identify the source dump, code/input/output hashes and complete schema/row delta; canonical publication remains lead-owned. Schema plus real excerpt-loader behavior is proved; a complete fresh-world import is not.

### Initial runtime subset
- [x] Apply only pinned TC SpellMgr.cpp4940–4956 load-time corrections:387981/388082 effect0 TargetA87, retaining authored destinations. Raw SQL remains unchanged; no generic target46 support.
- [x] Actual Telash387981/388082 effect254 casts move asynchronously to their exact source-authored destinations, including elevated endpoints; no launch teleport, ground snap or LOS-clamped replacement destination.
- [x] Source time mode, applicable speed cap and nullable-height semantics determine the supported motion. Missing parameters or unsupported options (including special facing) fail explicitly, never by teleport fallback.
- [x] Reuse real movement ownership, interpolation and arrival1004 dispatch; effect41 and existing Gordoth movement remain unchanged. Natural Telash landing reactions remain separate.
- [x] Native tests observe launch, mid-flight arc and source-derived landing time/position without constructing movement components or dispatching synthetic landing hooks.

## How it works
- [Telash source and proof coverage](../wiki/investigations/follower-data.md#telash-first-energy-phase--2026-10-17)
- [Spell processing](../wiki/systems/spell-casting.md)

## Implementation inventory
- `scripts/encounter_serverside_spells.py --encounter telash-jumps` — independent-copy server-source producer for all source jump rows, retaining nullable options and rejecting duplicate identities before writes.
- `scripts/tdb_schema.sql`, `scripts/import-azerothcore.sh` — fresh-import schema and required `jump_charge_params.sql` excerpt registration.
- `crates/server/src/spell_cast/jump_charge.rs`, `effects.rs` — dedicated254 source-table consumer through the existing `WorldData` connection; exact destination, explicit unsupported/missing refusal. Canonical source table publication is complete; native movement GREEN is pending.
- `crates/server/src/scripted_ai/jump_movement.rs` — shared asynchronous server motion/arrival; sourced time/unlimited/height options added while Gordoth keeps its speed/min-height defaults. Native254 jumping animation is unproved.

## Tests asserting this spec
- `scripts/tests/test_jump_charge_params.py` — real-copy producer CLI/source preservation and refusal boundary.
- `crates/server/src/scripted_ai/tests/azure_jump_charge.rs` — actual387981/388082 source-destination launch/midpoint/landing GREEN at5aa9f58a; enabled afterward. No movement components or synthetic arrival hooks.
- `spell_cast/jump_charge.rs` tests — missing-source refusal/no movement and actual unsupported source options. Native pair+missing control3PASS138.15s, retained unsupported1PASS; not a fresh4-case run.
- `spell_cast/dummy10_tests.rs`, `dummy14_tests.rs`, `teleport_tests.rs` — named player-displacement refusals preserve parent success, bite/transform/damage/CC, cooldowns and native zero resource costs. Reconciled old teleport-placeholder assertions;7 retained affected passes at22e59598 plus corrected Feral1PASS30.55s at74080cef. No fresh8-case run.
- Existing Gordoth arrival/death/lower-point ownership3 and Harpoon41 controls pass in the retained affected batch.

## Known gaps (current cycle)
- [x] Producer first: source-backed COPY preparation and scoped proof at fba6fb2b, pushed2399bcd06; lead gate510 landed and published5f225535 with byte-identical whole-database delta49b238f5.
- [x] Merge repaired masterefd627da1 (gate511) and record separate source-target planning RED:2FAIL60.53s at1b5370e7, missing destination versus wrong caster destination. Initial test compilation failure is not behavioral proof.
- [x] After exact source target fixes, observe genuine launch-teleport RED:2FAIL58.95s at985d91f3 after planning/native inputs/floor/LOS prerequisites. Outbound also ground-snapped to0; return to340.57086 instead of340.5709.
- [x] Bounded254 movement and missing/unsupported controls, preserved41/Gordoth/class effects, scoped server check0/38.20s/zero compilation warnings, fmt0 and independent read-only source/wiring verification. Composite11 distinct cases across retained scopes, not a fresh11-case run; full Azure is still open.

## Named unsupported displacement gaps

Lead-approved finite713/714 publication does not retain the old effect254 teleport placeholder. Only effect254 displacement refuses; parent casts and their other effects, trigger chains except the jump arrival trigger, resources and cooldowns remain independent.

- `JUMP254_PROGRESS_CURVE`:191427 (Havoc Metamorphosis), effect2/param9/progress1717;189110 (Infernal Strike), effect1/param2/progress1636. No transform, damage, CC or parent-cast refusal.
- `JUMP254_PARABOLIC_CURVE_TRIGGER`:196884 (Feral Lunge), effect1/param1211/parabolic79789/arrival215802. Its existing dummy-effect bite215802 still executes; no additional jump-arrival bite is fabricated.

Audit: all1539 installed native effect254 spell IDs checked against tracked server Rust test literals. Old movement assertions were in the Infernal Strike matrix and mixed Harpoon/Feral matrix; Feral bite and Meta damage/CC tests now also assert unchanged caster position and preserved parent effects.439683 matches only an aura-restriction loader fixture, not movement acceptance.

## Out of scope
- Parent387928 hit, Telash landing reaction, AfterCast return selection, post-phase counters and rune scripts: later independent REDs.
- Jump animation/client pixels, ordinary combat budgets and full Azure completion: require separate native acceptance.
- Unsupported254 option combinations and effect41: no guessed coverage or compatibility fallback.
