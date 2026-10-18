# Sourced jump-charge movement

Spell effect254 must use the actual server `jump_charge_params` referenced by its effect, initially covering Telash387981/388082 and source713/714. Authority is pinned TDB1210 server SQL and TC a352b1fa `ObjectMgr::LoadJumpChargeParams`, `Spell::EffectJumpCharge` and `MotionMaster::MoveJump`, not a client DB2. [Source/proof boundaries](../wiki/investigations/follower-data.md#telash-first-energy-phase--2026-10-17).

## What it must do

### Producer
- [x] Preserve the exact source table's columns, row values and NULLs in `tdb_jump_charge_params`; no invented speed, height, visual, curve or trigger values.
- [x] Preparation accepts independent copies only, is idempotent, refuses malformed source atomically and changes no unrelated tables, actors, placements or difficulty rows.
- [x] Fresh-import schema/table registration retains the same source table. COPY receipts identify the source dump, code/input/output hashes and complete schema/row delta; canonical publication remains lead-owned. Schema plus real excerpt-loader behavior is proved; a complete fresh-world import is not.

### Initial runtime subset
- [ ] Actual Telash387981/388082 effect254 casts move asynchronously to their exact source-authored destinations, including elevated endpoints; no launch teleport, ground snap or LOS-clamped replacement destination.
- [ ] Source time mode, applicable speed cap, nullable-height semantics and facing options determine the motion. Missing parameters or unsupported options fail explicitly, never by teleport fallback.
- [ ] Reuse real movement ownership, interpolation and arrival1004 dispatch; effect41 and existing Gordoth movement remain unchanged.
- [ ] Native tests observe launch, mid-flight arc and source-derived landing time/position without constructing movement components or dispatching synthetic landing hooks.

## How it works
- [Telash source and proof coverage](../wiki/investigations/follower-data.md#telash-first-energy-phase--2026-10-17)
- [Spell processing](../wiki/systems/spell-casting.md)

## Implementation inventory
- `scripts/encounter_serverside_spells.py --encounter telash-jumps` — independent-copy server-source producer for all source jump rows, retaining nullable options and rejecting duplicate identities before writes.
- `scripts/tdb_schema.sql`, `scripts/import-azerothcore.sh` — fresh-import schema and required `jump_charge_params.sql` excerpt registration.
- `crates/server/src/spell_cast/effects.rs`, `teleport.rs` — current254 dispatch still uses teleport; replacement is gated on producer publication.
- `crates/server/src/scripted_ai/jump_movement.rs` — existing speed-based server motion/arrival path; sourced254 subset pending.

## Tests asserting this spec
- `scripts/tests/test_jump_charge_params.py` — real-copy producer CLI/source preservation and refusal boundary.
- `crates/server/src/scripted_ai/tests/azure_jump_charge.rs` — test-first actual387981/388082 source-destination launch/midpoint/landing cases; ignored until the repaired-master RED queue. No movement components or synthetic arrival hooks. Existing Gordoth controls remain required regression proof.

## Known gaps (current cycle)
- [x] Producer first: source-backed COPY preparation and scoped proof at fba6fb2b, pushed2399bcd06; lead gate510 landed and published5f225535 with byte-identical whole-database delta49b238f5.
- [ ] Native RED queue waits the master two-caller compilation repair; then genuine no-teleport RED, bounded motion implementation and scoped controls/check/fmt.

## Out of scope
- Parent387928 hit, Telash landing reaction, AfterCast return selection, post-phase counters and rune scripts: later independent REDs.
- Jump animation/client pixels, ordinary combat budgets and full Azure completion: require separate native acceptance.
- Unsupported254 option combinations and effect41: no guessed coverage or compatibility fallback.
