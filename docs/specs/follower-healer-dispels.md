# Follower healer dispels

Healer APLs must recognize debuffs their actual native dispel spell can remove. Facts live in `server-combat/src/auras/dispellable.rs` and `simc_apl_native.rs`; see [auras](../wiki/systems/auras.md) and [healer replay](healer-tank-replay.md).

## What it must do

- [x] `target.dispellable_debuffs` evaluates the selected action target, not only the current healing target or the lowest-health party member.
- [x] Eligibility comes from native dispel effects and aura DispelType, with no debuff spell-ID allowlist. Helpful, passive and expired auras are not eligible.
- [x] Facts and real native dispel removal agree. Dispel logs retain the removed aura spell ID; unrelated debuffs and helpful auras remain.
- [ ] Healer action priority is labelled `INFERRED_WCL` and sourced from current-build observations across several players, with at least fifty successful dispels per healer.
- [ ] Matched healer replay intake carries logged dispellable debuff applications so the normal native dispel action can act on them.
- [ ] Recorded intake remains the sole damage source: replayed debuffs retain dispel/control/stat state without duplicating native periodic damage. A native dispel earlier than the logged removal suppresses subsequent recorded ticks of that debuff episode, and each seed reports the dropped count.

## How it works

- [Aura runtime](../wiki/systems/auras.md)
- [Healer replay](../wiki/systems/healer-tank-replay.md)

## Implementation inventory

- `crates/server-combat/src/auras/dispellable.rs` — native spell/aura eligibility facts.
- `crates/server-combat/src/simc_apl_native.rs` — action-target fact operand.
- `crates/server-combat/src/auras/mod.rs` — existing dispel removal and combat-log metadata.

## Tests asserting this spec

- `crates/server-combat/src/spell_cast/dispel_fact_tests.rs` — native masks, explicit healthy target, actual Purify removal and controls.

## Known gaps (current cycle)

- [ ] Per-healer usage, delay, removal share and ready-but-not-cast evidence.
- [ ] Coordinate action/binding/party-target integration with the healer-list owners.
- [ ] Timed dispellable debuffs in matched replay intake.

## Out of scope

- Enemy buff purges and non-healer dispel policy are not part of this task.
