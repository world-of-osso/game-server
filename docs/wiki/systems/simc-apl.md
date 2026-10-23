# Shared SimC APL evaluation

`server::simc_apl` supplies shared expressions, variables and source-ordered call lists. `crate::simc_apl_native` supplies shared native gameplay facts for explicit parity fixtures; spec adapters add only spec predicates plus action/target bindings and castability. Neither layer invents missing combat data.

## Capability / proof matrix

| Capability | Representation / boundary | Proof |
|---|---|---|
| OR/AND/NOT, strict comparisons, numeric truth, arithmetic | Frozen precedence; `%` division and `%%` signed remainder; numeric short circuit | Concrete WW/Ret11RED atc0cf6eed6→11GREEN ataff61cbab; enabled afterward |
| Buff/cooldown/resource/talent/fixture facts | Shared native adapter reads actual earned state, selected allocation, display units and native timer helpers; unknown aliases fail | Six actual native6RED→6GREEN17.40s atdc1ee9781, zero compiler warnings; equipment and spec predicates remain caller-owned |
| Variables | First declaration's numeric default (native default0); source `set`, `setif`, `value_else`, `reset`; per-actor `Program::state()` | Ret default/setif/reset decisions GREEN |
| Conditional call lists | Literal row order, first native-usable action; exhausted/explicit empty child returns to caller; unknown call rejected and recursion errors explicit | WW ST/3T and Ret finisher unit decisions GREEN; recursion control added afterward |
| Run lists and target selection | Run exhaustion cannot resume caller; min/max selects candidate before if, strict ties preserve native order. A run selector belongs only to its control row; children keep their own targets | Pinned native SimC invalidated old inherited-child oracle; replacement23PASS/1 genuineRED at2d6a66259 →24/24GREEN atad720fc3c,28.61s compile/0compiler warnings/0ignored. [Proof](../../handoffs/shared-run-target-proof-ledger.md) |
| Per-action facts | `ActionFacts::value_for(&Action, operand)` is mandatory for Program; no actor-wide fallback. SimC combo_strike and plain gcd are action-bound | Two genuine context REDs at2e6f242ce→13-case GREEN atcd3c7e427, including11 unchanged controls; both context cases enabled afterward |
| Native follower execution | Caller-owned spellbook, target semantics and `creature_cast` integration | Literal WW/Ret adapters remain open. Lead approved only explicit parity Fixture drivers; Stockade retains its independent native kit and named DUNGEON_TTD_UNAVAILABLE gap |

`Facts` remains the standalone expression interface. `Program::select` requires `ActionFacts`; actor-only snapshots cannot represent proposed-action combo checks or call-control gcd0. `BTreeMap` explicitly models context-independent numeric fixtures.

`Program::parse` takes `(list, literal)` rows; an empty literal declares an intentionally empty list. `Program::select` updates variables and returns an `Action` with its non-`if` options intact. `target_if=min/max:<expr>` is interpreted by Program using explicit `ActionFacts::target_ids(&Action)` per-action eligible order and action-bound facts; the selected identity is returned in `Action.target`, separately from preserved literal options. Untargeted actions retain the caller's native current target. Native adapters must honor that identity in readiness/dispatch; actor-only adapters return an explicit target-candidate error. Cancellation and item options remain native-adapter responsibilities, never silently dropped. Unsupported expression syntax, control options and variable operations fail explicitly.

## Native binding boundary

`NativeFacts::new(&World, actor, &Bindings, Encounter, active_enemies)` implements both shared traits. Bindings register explicit native action/buff/talent IDs and power types plus the actor's native GCD spell. `Encounter::Fixture` uses one simulation-clock300s deadline for fight and every target; `Encounter::Dungeon` fails TTD/fight-remains explicitly. Native cooldown wall-clock timing stays separate from aura/encounter simulation time. No native-book helper dependency or generic kit-membership-as-knowledge claim. [Native proof ledger/capability limits](../../handoffs/shared-native-apl-proof-ledger.md).

## Sources

- Pinned SimC76f8118 `engine/sim/expressions.cpp:22–154,333–440,1081–1130` — lexer, arithmetic/boolean decisions and precedence.
- `engine/action/variable.cpp:19–133` — defaults and variable construction; `engine/player/action_variable.cpp:19–26` — initial values.
- `engine/action/action.cpp:4648–4689,4742–4763` — conditional calls versus non-returning run-list switches; `engine/player/player.cpp:14178–14265` — ordered variable/call selection and cycle rejection.
- `engine/action/action.cpp:2435–2468,2619,5362–5453` — target choice before if, min/max and strict candidate-order ties; [Arms source ledger](../../handoffs/arms-source-census.md).
- `profiles/MID2/MID2_Monk_Windwalker.simc:69`; `engine/class_modules/apl/apl_paladin.cpp:21–27,43,46–49` — concrete tests.
- [Contract](../../specs/simc-apl.md); [WW census](../../handoffs/ww-source-census.md).
