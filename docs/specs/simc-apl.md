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

## Known gaps (current cycle)

- [ ] Wire native kit adapters; library expression/variable/list decisions have11-case GREEN, not full native rotation proof.
- [ ] Native WW and Ret fact adapters and literal per-spec action policies; Ret adapter belongs to14752.

## Out of scope

- Player AssistedCombat adapter, native-book duplication, invented facts or spell coefficients, and heuristic substitutes for unsupported conditions.
