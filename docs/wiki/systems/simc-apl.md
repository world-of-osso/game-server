# Shared SimC APL evaluation

`server::simc_apl` supplies shared expressions, variables and source-ordered call lists. Spec adapters own native facts, action/target bindings and castability; neither this library nor a missing operand invents combat data.

## Capability / proof matrix

| Capability | Representation / boundary | Proof |
|---|---|---|
| OR/AND/NOT, strict comparisons, numeric truth, arithmetic | Frozen precedence; `%` division and `%%` signed remainder; numeric short circuit | Concrete WW/Ret11RED atc0cf6eed6→11GREEN ataff61cbab; enabled afterward |
| Buff/cooldown/resource/talent/equipment/target facts | `Facts::value` supplies explicit numeric paths; every referenced operand is bound before evaluation, including inactive branches | WW Tiger Palm, Ret ES/trinket conditions GREEN; native adapters still pending |
| Variables | First declaration's numeric default (native default0); source `set`, `setif`, `value_else`, `reset`; per-actor `Program::state()` | Ret default/setif/reset decisions GREEN |
| Conditional call lists | Literal row order, first native-usable action; exhausted/explicit empty child returns to caller; unknown call rejected and recursion errors explicit | WW ST/3T and Ret finisher unit decisions GREEN; recursion control added afterward |
| Per-action facts | `ActionFacts::value_for(&Action, operand)` is mandatory for Program; no actor-wide fallback. SimC combo_strike and plain gcd are action-bound | Two genuine context REDs at2e6f242ce→13-case GREEN atcd3c7e427, including11 unchanged controls; both context cases enabled afterward |
| Native follower execution | Caller-owned spellbook, facts, target semantics and `creature_cast` integration | Not implemented here yet; Ret adapter belongs to14752, WW adapter remains open |

`Facts` remains the standalone expression interface. `Program::select` requires `ActionFacts`; actor-only snapshots cannot represent proposed-action combo checks or call-control gcd0. `BTreeMap` explicitly models context-independent numeric fixtures.

`Program::parse` takes `(list, literal)` rows; an empty literal declares an intentionally empty list. `Program::select` updates variables and returns an `Action` with its non-`if` options intact. Target selectors, cancellation and item options are not silently interpreted or dropped: native adapters must implement them or report cited gaps. Unsupported expression syntax, control options and variable operations fail explicitly.

## Sources

- Pinned SimC76f8118 `engine/sim/expressions.cpp:22–154,333–440,1081–1130` — lexer, arithmetic/boolean decisions and precedence.
- `engine/action/variable.cpp:19–133` — defaults and variable construction; `engine/player/action_variable.cpp:19–26` — initial values.
- `engine/action/action.cpp:4648–4689` — conditional calls and unknown-list rejection; `engine/player/player.cpp:14178–14265` — ordered variable/call selection and cycle rejection.
- `profiles/MID2/MID2_Monk_Windwalker.simc:69`; `engine/class_modules/apl/apl_paladin.cpp:21–27,43,46–49` — concrete tests.
- [Contract](../../specs/simc-apl.md); [WW census](../../handoffs/ww-source-census.md).
