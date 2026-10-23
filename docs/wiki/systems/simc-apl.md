# Shared SimC APL evaluation

`server::simc_apl` supplies shared expressions, variables and source-ordered call lists. `crate::simc_apl_native` supplies shared native gameplay facts for explicit parity fixtures; spec adapters add only spec predicates plus action/target bindings and castability. Neither layer invents missing combat data.

## Capability / proof matrix

| Capability | Representation / boundary | Proof |
|---|---|---|
| OR/AND/NOT, strict comparisons, numeric truth, arithmetic | Frozen precedence; `%` division and `%%` signed remainder; numeric short circuit | Concrete WW/Ret11RED atc0cf6eed6→11GREEN ataff61cbab; enabled afterward |
| Buff/cooldown/resource/talent/fixture facts | Shared native adapter reads actual earned state, selected allocation, display units and native timer helpers; unknown aliases fail | Six actual native6RED→6GREEN17.40s atdc1ee9781, zero compiler warnings; equipment and spec predicates remain caller-owned |
| Variables | First declaration's numeric default (native default0); source `set`, `setif`, `value_else`, `reset`; per-actor `Program::state()` | Ret default/setif/reset decisions GREEN |
| Conditional call lists | Literal row order, first native-usable action; exhausted/explicit empty child returns to caller; unknown call rejected and recursion errors explicit | WW ST/3T and Ret finisher unit decisions GREEN; recursion control added afterward |
| Per-action facts | `ActionFacts::value_for(&Action, operand)` is mandatory for Program; no actor-wide fallback. SimC combo_strike and plain gcd are action-bound | Two genuine context REDs at2e6f242ce→13-case GREEN atcd3c7e427, including11 unchanged controls; both context cases enabled afterward |
| Native follower execution | Caller-owned spellbook, target semantics and `creature_cast` integration | WW132literal rows/native adapter5GREEN35.19s at32e4610b7: action context, real ST/3T casts/recipients, dungeon rejection/non-Fixture kit. Damage/proc qualification and Ret integration remain open. Only explicit parity Fixture captures decisions; Stockade retains its independent native kit and named DUNGEON_TTD_UNAVAILABLE gap |

`Facts` remains the standalone expression interface. `Program::select` requires `ActionFacts`; actor-only snapshots cannot represent proposed-action combo checks or call-control gcd0. `BTreeMap` explicitly models context-independent numeric fixtures.

`Program::parse` takes `(list, literal)` rows; an empty literal declares an intentionally empty list. `Program::select` updates variables and returns an `Action` with its non-`if` options intact. Target selectors, cancellation and item options are not silently interpreted or dropped: native adapters must implement them or report cited gaps. Unsupported expression syntax, control options and variable operations fail explicitly.

## Native binding boundary

`NativeFacts::new(&World, actor, &Bindings, Encounter, active_enemies)` implements both shared traits. Bindings register explicit native action/buff/talent IDs and power types plus the actor's native GCD spell. `Encounter::Fixture` uses one simulation-clock300s deadline for fight and every target; `Encounter::Dungeon` fails TTD/fight-remains explicitly. Native cooldown wall-clock timing stays separate from aura/encounter simulation time. No native-book helper dependency or generic kit-membership-as-knowledge claim. [Native proof ledger/capability limits](../../handoffs/shared-native-apl-proof-ledger.md).

## Shared literal fixture machinery

`dungeon_simulator::dps_parity::literal_rotation` owns reusable native item/readiness/execution state; WW source/bindings/facts remain spec-local. Extraction preserves the32e4610b7 five-test behavior; post-extraction GREEN pending.

- `equipment::Equipment::load(world, actor, &mut Bindings)` reads equipped native identities/item levels/on-use definitions, registers slot cooldown aliases; `fact(name, &NativeFacts)` reads qualified trinket operands.
- `execution::Prepared::read(world, actor, targets, &Equipment, actions, needs_snapshot)` samples actual native readiness before immutable facts borrowing. Each `NativeAction { name, spell, talent: Option<u32>, self_target }` comes from the spec adapter; inventory/provider gaps remain adapter-owned. Actual selected talents, KnownSpells and overrides gate casts.
- `Driver::new(Program)` owns variable state, snapshot, decisions and channel cancellation. `select(list, &dyn ActionFacts, &Prepared, &Equipment)` returns the source-selected `(Action, Execution)`; release immutable facts, then `execute(world, actor, action, execution)`. `should_cancel(world, actor, facts)` evaluates an active native channel's saved `cancel_if`; caller then cancels the actual cast.
- `capture_fixture(world, actor, Encounter)` rejects dungeon clocks before inserting `LiteralParityFixture`; the native kit yields while this marker exists, preserving Follower/NPC identity. Adapters retain explicit precombat/default ordering and spec actor checks.

## Sources

- Pinned SimC76f8118 `engine/sim/expressions.cpp:22–154,333–440,1081–1130` — lexer, arithmetic/boolean decisions and precedence.
- `engine/action/variable.cpp:19–133` — defaults and variable construction; `engine/player/action_variable.cpp:19–26` — initial values.
- `engine/action/action.cpp:4648–4689` — conditional calls and unknown-list rejection; `engine/player/player.cpp:14178–14265` — ordered variable/call selection and cycle rejection.
- `profiles/MID2/MID2_Monk_Windwalker.simc:69`; `engine/class_modules/apl/apl_paladin.cpp:21–27,43,46–49` — concrete tests.
- [Contract](../../specs/simc-apl.md); [WW census](../../handoffs/ww-source-census.md).
