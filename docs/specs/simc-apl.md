# Literal SimC APL decisions

Shared native follower decision evaluation interprets pinned SimC76f8118 conditions and source-ordered action lists. Native spellbook, castability and numeric combat data remain authoritative and caller-owned. [Implementation details](../wiki/systems/simc-apl.md).

## What it must do

- [ ] Interpret numeric truth, OR/AND/NOT, arithmetic and comparisons with pinned SimC precedence. Single `%` divides; `%%` is modulo.
- [ ] Read explicit native facts for buff stacks/remains/up, cooldown remains/charges/duration, resources, talents, equipment/trinkets, target count/TTD, fight remains, raid-event adds, dots/debuffs and gcd. Unknown operands fail explicitly, including in otherwise inactive condition branches; never silently become false.
- [ ] Execute declared variables using source `value`, `setif`, `value_else`, `reset` and numeric `default` semantics.
- [ ] Preserve literal action order and conditional `call_action_list` decisions; native castability selects the first usable action. An exhausted child list returns control to its caller.
- [ ] Evaluate concrete default WW and Ret source lines, including resource/buff stacks, Execution Sentence and trinket synchronization.

## How it works

- [Shared APL evaluator](../wiki/systems/simc-apl.md).
- [WW source census](../handoffs/ww-source-census.md).

## Implementation inventory

- `crates/server/src/simc_apl.rs` — shared interpreter entry point; currently RED scaffold only.
- `crates/server/src/simc_apl/tests.rs` — concrete WW/Ret behavioral expressions.
- `crates/server/src/lib.rs` — engine-free evaluator export.

## Tests asserting this spec

`crates/server/src/simc_apl/tests.rs`; six ignored RED cases pushed before implementation. Runtime result pending.

## Known gaps (current cycle)

- [ ] Expression parser/evaluator, variable/list execution and native kit integration.
- [ ] Native WW and Ret fact adapters and literal per-spec action policies; Ret adapter belongs to14752.

## Out of scope

- Player AssistedCombat adapter, native-book duplication, invented facts or spell coefficients, and heuristic substitutes for unsupported conditions.
