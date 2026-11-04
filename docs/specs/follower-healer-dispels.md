# Follower healer dispels

Healer APLs must recognize debuffs their actual native dispel spell can remove. Facts live in `server-combat/src/auras/dispellable.rs` and `simc_apl_native.rs`; see [auras](../wiki/systems/auras.md) and [healer replay](healer-tank-replay.md).

## What it must do

- [x] `target.dispellable_debuffs` evaluates the selected action target, not only the current healing target or the lowest-health party member.
- [x] Eligibility comes from native dispel effects and aura DispelType, with no debuff spell-ID allowlist. Helpful, passive and expired auras are not eligible.
- [x] Facts and real native dispel removal agree. Dispel logs retain the removed aura spell ID; unrelated debuffs and helpful auras remain.
- [ ] Healer action priority is labelled `INFERRED_WCL` and sourced from current-build observations across several players, with at least fifty successful dispels per healer.
- [x] Matched healer replay intake carries logged dispellable debuff applications so the normal native dispel action can act on them.
- [x] Recorded intake remains the sole damage source: replayed debuffs retain dispel/control/stat state without duplicating native periodic damage. A native dispel earlier than the logged removal suppresses subsequent recorded ticks of that debuff episode, and each seed reports the dropped count.

## How it works

- [Aura runtime](../wiki/systems/auras.md)
- [Healer replay](../wiki/systems/healer-tank-replay.md)

## Implementation inventory

- `crates/server-combat/src/auras/dispellable.rs` — native spell/aura eligibility facts.
- `crates/server-combat/src/simc_apl_native.rs` — action-target fact operand.
- `crates/server-combat/src/auras/mod.rs` — existing dispel removal and combat-log metadata.
- `scripts/healer_wcl_debuffs.py` — DB2-selected, source-qualified episodes and periodic-child hit links. Intake requires `--debuffs`.
- `crates/server-simulation/src/dungeon_simulator/healer_wcl_debuffs.rs` — native aura state with periodic timers disabled; exact instance-removal receipts distinguish dispels from expiry and other removals.
- Each seed's `wcl_intake.ticks_dropped_after_native_dispel` counts suppressed linked receipts. Recorded refreshes do not restore an early-dispelled episode; a new application starts a new episode.

## Tests asserting this spec

- `crates/server-combat/src/spell_cast/dispel_fact_tests.rs` — native masks, explicit healthy target, actual Purify removal and controls.
- `scripts/tests/test_healer_wcl_debuffs.py` — source/instance/target-qualified child ticks, refresh/stack state, reapplications and ambiguous-link rejection.
- Native `logged_debuff_state_preserves_control_without_damage_and_drops_only_dispelled_episode_ticks` — Paralyzing Shots slow and Magic state, zero native ticks, recorded pressure, actual Purify, other-source controls and new-episode recovery across seeds1/2.

## Known gaps (current cycle)

- [ ] Per-healer usage, delay, removal share and ready-but-not-cast evidence.
- [ ] Coordinate action/binding/party-target integration with the healer-list owners.
- [x] Timed dispellable debuffs in matched replay intake.

## Out of scope

- Enemy buff purges and non-healer dispel policy are not part of this task.
