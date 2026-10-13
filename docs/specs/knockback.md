# Source knockback

Effect speeds, targeting and immunity use TC a352b1fa SpellEffects.cpp:4037 / Unit.cpp:12565 and local SpellEffect DB2. Server-authoritative trajectories, collision and fall damage are existing realm policy, not TC player-movement parity. [Arena use](arenas.md).

## What it must do

- [ ] Effects98/144 use MiscValue×0.1 horizontal speed and modified BasePoints×0.1 vertical speed; destination144 requires its actual destination. Coincident origins use target facing+pi; signed horizontal speeds reverse direction. NPC negative vertical speeds become positive (MotionMaster.cpp:797–798); player vertical speed retains its sign.
- [ ] Reject rooted, not merely stunned, targets and sourced player-cast world/dungeon-boss exceptions; honor NPC flags_extra knockback immunity. Zero impulses do not move targets.
- [ ] Realm policy: server owns the ballistic trajectory with source19.291103yd/s² gravity. Controlled/epoch and Position replication drive owning native clients and observers; owner reports cannot override flight or replay after landing.
- [ ] Terrain/WMO/LOS collision bounds movement. Missing source ground/collision services produce explicit errors, never guessed floors. Map transfer/death ends the current lease without stopping a newer movement owner.
- [ ] Landing releases ownership and preserves realm fall-damage/feather-fall rules. Creature AI cannot override flight. Successful impulses emit recipient-side PROC_FLAG_2_KNOCKBACK2.
- [ ] Source7/8 area-entry selectors honor same-copy spherical range and authored SpellTargetConditions, not guessed racial hostility. SPELL_ATTR1_EXCLUDE_CASTER filters before target caps, including triggered entry-area searches; explicit self-targets fail with bad-targets and environmental spouts cannot push themselves.

## How it works

`KnockbackFlight` owns `MovementControl` until landing or lease invalidation; spell handlers supply sourced impulses and immunity.

## Implementation inventory

- `crates/server/src/knockback.rs` — movement ownership, trajectory, collision and landing.
- `crates/server/src/spell_cast/knockback.rs` — sourced effect98/144 speeds, immunity and proc event.
- `crates/server/src/spell_cast/targets.rs` — authored entry-area selectors7/8.
- `crates/server/src/creature_ai.rs` — forced-movement ownership exclusion.
- `crates/server/src/world_data.rs` — literal creature_template.flags_extra.
- `shared-protocol/src/components.rs` — existing controlled movement contract.

## Tests asserting this spec

- `crates/server/src/spell_cast/knockback_tests.rs` — actual58873, fixed-tick trajectory and HostClient owner-input lifetime.
- Native controlled movement assertions: game-engine `godot/rust/src/world.rs`.

## Known gaps (current cycle)

- [ ] Finish source effect/lifetime and native integration proof; ledger: /home/osso/.worktrees/handoff-arena-coverage.md.

## Out of scope

- Unsourced alternate impulses or movement-policy compatibility fallbacks.
