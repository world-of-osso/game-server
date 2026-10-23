# Literal SimC APL decisions

Shared native follower decision evaluation interprets pinned SimC76f8118 conditions and source-ordered action lists. Native spellbook, castability and numeric combat data remain authoritative and caller-owned. [Implementation details](../wiki/systems/simc-apl.md).

## What it must do

- [x] Interpret numeric truth, OR/AND/NOT, arithmetic and comparisons with pinned SimC precedence. Single `%` divides; `%%` is modulo.
- [ ] Read explicit native facts for buff stacks/remains/up, cooldown remains/charges/duration, resources, talents, equipment/trinkets, target count/TTD, fight remains, raid-event adds, dots/debuffs and gcd. Unknown operands fail explicitly, including in otherwise inactive condition branches; never silently become false.
- [x] Bind action-dependent facts (`combo_strike`, plain `gcd`, action minimum GCD) to each source row, including variable/call control actions. An actor-wide value is not equivalent.
- [x] Execute declared variables using source `value`, `setif`, `value_else`, `reset` and numeric `default` semantics.
- [x] Preserve literal action order and conditional `call_action_list` decisions; native castability selects the first usable action. An exhausted child list returns control to its caller.
- [x] Evaluate concrete default WW and Ret source lines, including resource/buff stacks, Execution Sentence and trinket synchronization.

## How it works

- [Shared APL evaluator](../wiki/systems/simc-apl.md).
- [WW source census](../handoffs/ww-source-census.md).

## Implementation inventory

- `crates/server/src/simc_apl.rs` — shared interpreter entry point and explicit fact binding.
- `crates/server/src/simc_apl/expression.rs` — pinned lexical/operator semantics.
- `crates/server/src/simc_apl/program.rs` — source-order variables and call lists.
- `crates/server/src/simc_apl/tests.rs` — concrete WW/Ret behavioral expressions.
- `crates/server/src/simc_apl/program_tests.rs` — variable lifetimes, source-order calls and native castability decisions.
- `crates/server/src/simc_apl/context_tests.rs` — proposed-action combo decisions and Ret call-control GCD.
- `crates/server/src/lib.rs` — engine-free evaluator export.

## Tests asserting this spec

`crates/server/src/simc_apl/tests.rs`; eleven expression/program cases pushed ignored before implementation: genuine11RED atc0cf6eed6→11GREEN ataff61cbab (exit0, zero compiler warnings,1m03 compile). Cases enabled afterward; invalid-control case additionally checks sourced call recursion. Two action-context REDs at2e6f242ce→all13GREEN atcd3c7e427, zero compiler warnings; context cases enabled afterward.

## Shared native facts

`crate::simc_apl_native` binds gameplay state to the engine-free traits without moving the interpreter into the server binary. Explicit alias registries identify native actions, buffs, talents and resource types; absent aliases fail, never become false. Buff up/stack/remains, real cooldowns/charges/durations, displayed resources, action-bound GCD and engaged enemy counts come from native state. The fixed-time parity encounter supplies elapsed/fight-remains/target-TTD and fight style; dungeon TTD/fight-remains remain explicit unavailable predicates, not estimates or a fallback. Spec adapters add only spec-specific facts.

Native behavioral tests: `simc_apl_native/tests.rs` (earned Hit Combo and Chi, real Provoke cooldown, real Roll charges, actual Charge Rage display conversion, fixture clock and unavailable dungeon predicates). Six pushed ignored native cases produce genuine6RED at5924496f9 (13.24s test/8m18compile/exit101; three scaffold dead-field warnings). Atdc1ee9781 the same six native cases pass6/6 in17.40s (7m37compile),exit0,zero compiler warnings; tests enabled afterward without body changes ([native proof ledger](../handoffs/shared-native-apl-proof-ledger.md)). Current native fact seam is fixture-only (`cfg(test)`), matching the approved explicit parity Fixture driver boundary; live dungeon APL/TTD is not enabled.

## Known gaps (current cycle)

- [ ] Wire native kit adapters; library expression/variable/list decisions have11-case GREEN, not full native rotation proof.
- [ ] Native WW and Ret fact adapters and literal per-spec action policies; Ret adapter belongs to14752.

## Out of scope

- Player AssistedCombat adapter, native-book duplication, invented facts or spell coefficients, and heuristic substitutes for unsupported conditions.
