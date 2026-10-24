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
| Native follower execution | Caller-owned spellbook, target semantics and `creature_cast` integration | WW132literal rows/native adapter5GREEN35.19s at32e4610b7: action context, real ST/3T casts/recipients, dungeon rejection/non-Fixture kit. Damage/proc qualification and Ret integration remain open. Only explicit parity Fixture captures decisions; Stockade retains its independent native kit and named DUNGEON_TTD_UNAVAILABLE gap |

`Facts` remains the standalone expression interface. `Program::select` requires `ActionFacts`; actor-only snapshots cannot represent proposed-action combo checks or call-control gcd0. `BTreeMap` explicitly models context-independent numeric fixtures.

`Program::parse` takes `(list, literal)` rows; an empty literal declares an intentionally empty list. `Program::select` updates variables and returns an `Action` with its non-`if` options intact. `target_if=min/max:<expr>` is interpreted by Program using explicit `ActionFacts::target_ids(&Action)` per-action eligible order and action-bound facts; the selected identity is returned in `Action.target`, separately from preserved literal options. Untargeted actions retain the caller's native current target. Native adapters must honor that identity in readiness/dispatch; actor-only adapters return an explicit target-candidate error. Cancellation and item options remain native-adapter responsibilities, never silently dropped. Unsupported expression syntax, control options and variable operations fail explicitly.

## Native binding boundary

`NativeFacts::new(&World, actor, &Bindings, Encounter, active_enemies)` implements both shared traits. Bindings register explicit native action/buff/talent IDs and power types plus the actor's native GCD spell. `Encounter::Fixture` uses one simulation-clock300s deadline for fight and every target; `Encounter::Dungeon` fails TTD/fight-remains explicitly. Native cooldown wall-clock timing stays separate from aura/encounter simulation time. No native-book helper dependency or generic kit-membership-as-knowledge claim. [Native proof ledger/capability limits](../../handoffs/shared-native-apl-proof-ledger.md).

## Shared literal fixture machinery

After the explicit target-candidate interpreter landed, WW's count-only adapter rejected actual ST/3T runs atd45fe30e8 with `target_if requires explicit native target candidates`. WW now carries the actual supplied entity slice into target_ids; shared pd1035 Prepared/Execution preserves selected identities. This integration is not a first-target fallback, and per-fixture TTD still follows one global fixed-time clock. Same-binary ff8ba4206 actual300s ST/3T now pass the target seam at47064.44/81373.16, still−77.20%/−78.39% versus the requested reference, NOT parity; [exact failures/current receipts](../../handoffs/ww-source-census.md).

`dungeon_simulator::dps_parity::literal_rotation` owns reusable native item/readiness/execution state; WW source/bindings/facts remain spec-local. Post-extraction original5GREEN at3e71b8aa2; FoF facing reproduction then5GREEN/1RED19.49s→6GREEN39.89s at0e34d8666. Native auto-attack must also put an existing CreatureState into Pursuing: native chase supplies facing, otherwise117418's native cone misses despite valid periodic metadata. No synthetic facing or FoF effect change; [proof and300s gaps](../../handoffs/ww-source-census.md#fof-literal-controller-root-cause). Player spells use the ordinary `SpellCastIntent`/`request_cast` path with an explicit target; NPC/follower spells retain `creature_cast`. A stale explicit Player target fails `NoTarget`, never becomes the primary target. Seven real native Human/WW cases at4f91688e8 produce1WW controlPASS/6genuine assertionRED →7GREEN27.17s at e99b879e8 after hp1026 (18m08 compile/0compiler warnings). [Scoped proof](../../handoffs/literal-player-dispatch-proof-ledger.md). The profile's Troll race is not asserted to be Human: the native actor is explicitly created/asserted as Human1. Readiness cannot authorize a later no-longer-known Player cast.

- `equipment::Equipment::load(world, actor, &mut Bindings)` reads equipped native identities/item levels/on-use definitions, registers slot cooldown aliases; `fact(name, &NativeFacts)` reads qualified trinket operands.
- `execution::Prepared::read(world, actor, targets, &Equipment, actions, needs_snapshot)` samples actual native readiness per `(action, target)` before immutable facts borrowing; explicit target lookup never substitutes primary admission. Untargeted actions retain their own native default, and harmful precasts without supplied candidates read the actor's real `CombatTarget`. `retarget_auto_attack` changes only `AutoAttack.victim`; an active AA is not restarted back onto primary. Each `NativeAction { name, spell, talent: Option<u32>, self_target }` comes from the spec adapter; inventory/provider gaps remain adapter-owned. Actual selected talents, KnownSpells and overrides gate casts.
- `Driver::new(Program)` owns variable state, snapshot, decisions and channel cancellation. `select(list, &dyn ActionFacts, &Prepared, &Equipment)` returns the source-selected `(Action, Execution)`; release immutable facts, then `execute(world, actor, action, execution)`. `should_cancel(world, actor, facts)` evaluates an active native channel's saved `cancel_if`; caller then cancels the actual cast.
- `capture_fixture(world, actor, Encounter)` rejects dungeon clocks before inserting `LiteralParityFixture`; the native kit yields while this marker exists, preserving Follower/NPC identity. Adapters retain explicit precombat/default ordering and spec actor checks.

Native dungeon execution remains separate: a [level32 Bo269 Stockade receipt](../../handoffs/ww-stockade-proof-ledger.md) now records one complete seed11 clear with actual WW damage and native kit casts. It is not a BIS90/literal dungeon APL or dummy-parity qualification.

## Precombat lifecycle —2026-10-23, partially qualified

`Program::select_next` consumes direct top-level precombat rows once, retaining source variable assignment and selected-target logic. `Driver` keeps a precombat cursor separate from combat priority scanning. Nested precombat call/run controls fail explicitly; pinned MID2 source has none. Three actual library REDs→four GREEN at171c8ec5c qualify this evaluator boundary, not native cast timing.

Fixture preparation now separates spawn from warmup: explicit literal capture precedes every setup frame, never deletes unauthorized kit auras afterward, and preserves Follower/NPC identity. Native nonliteral Feral still casts its class buff. Raw profile class options survive deserialization; declared resources resolve native PowerType names/display units and initialize before setup. Configured pets resolve learned native summon names and SUMMON_PET entries. Missing/different pets use normal cast permission; a living matching pet is a no-op. Native affordability waits advance actual OOC regeneration, not grants or cost waivers.

WW measurement runs its ordered precombat before explicit engagement and the encounter clock; each native execution settles queued effects and real cast/all-GCD state. Literal engagement retains native precast impacts. Player dispatch and explicit target validation stay on their existing paths.

| Boundary | Exact proof / remaining gap |
|---|---|
| Early handoff, native-kit isolation, declared0shards | Native pass ated76/f5; prior real REDs retained |
| Configured pet real cast/no resummon/pet at pull | Native pass after waiting existing OOC shard regeneration; no pool reset |
| WW snapshot while OOC and fight time0 | Native pass before opponent refs are established |
| Full acceptance / composites | Overall6PASS/2FAIL: Demo damage and first-harmful counterfactual blocked by copied dungeon-graph movement;14755 owns fixture geometry. Four native poison casts/composite14775 seam still unqualified |

[Precombat ledger](../../handoffs/native-precombat-proof-ledger.md) owns exact commands, hashes and current blockers. The bounded native pet control does not claim full literal Demonology APL integration; first-harmful eligibility and poison selection/timing require their separate end-to-end controls.
`NativeComposite { name, steps }` metadata feeds `Prepared::with_composites(world, actor, targets, metadata)`. It preserves pending native step order, skips actual owned active self auras, and rejects the whole plan when a selected pending step is unready. Empty/all-active plans are unusable. `Execution::Composite(Vec<Execution>)` is pending work only: world-only execute rejects it before dispatch;14766 owns real precombat completion. Native259 four-coating/6673 source tests at4ad yield3controlsPASS/3genuine selectionRED26.39s; implementation GREEN pending. [Scoped proof/API](../../handoffs/literal-composite-proof-ledger.md); no poison IDs or instant scheduling in shared code.

Clock-aware `preparation::execute_precombat_plan` now consumes selected native plans one child at a time with full App cast/GCD/effect completion between dispatches and before returning. The actual259 lifecycle first reached an explicit unconsumed-Composite RED atd49f4951c; implementation GREEN remains pending. World-only execute still rejects Composite. [Exact proof](../../handoffs/native-precombat-proof-ledger.md#composite-lifecycle-integration--genuine-red-green-pending).

[Mystic Touch producer](../../handoffs/ww-mystic-touch-proof-ledger.md) reached real native learned TP/white2RED and unlearned/unrelated2GREEN atd49f4951c. Named producer now reads the actual damage actor's learned8647 and native family/white-hit eligibility, then triggers native113746; its physical vulnerability remains native data. GREEN and first/later-hit numerical ordering remain open.

## Literal native row eligibility — GREEN pending

[Native row-gating packet](../../handoffs/literal-row-gating-proof-ledger.md) reaches4RED/1GREEN atd49f4951c. Prepared now samples actual Moving and active native GCD; Driver honors moving0/1 and explicit off-GCD row permission without changing native cast admission. Configured movement.distance remains a named unavailable gap, never zero. Runtime GREEN/Ele item activation remain unqualified.

## Assigned extension qualification

Seven native regressions at89f985420 reach unknown requested predicates (18.52s): actual selected multi-rank talent, earned Hit Combo cap/loss, native Martial Agility haste, contextual Roll charges/recharge, real Flame/channel execution duration, unpaid Disintegrate ticks and Provoke recovery versus target range. The extension implementation reuses selected source config and the aura refresh scheduler's tick counter; GREEN remains pending. Action readiness reads native recovery, caster restrictions, movement, power and target checks independently of the current GCD/executing action; actual dispatch still uses full native admission. Missing source metadata remains an error. Replacement-spell identity integration awaits the shared actor-action resolver.

[Native haste composition qualification](../../specs/simc-apl.md#native-haste-composition-boundary) is a separate RED-before-fix gate; the seven fact regressions do not prove haste composition.

## Sources

- Pinned SimC76f8118 `engine/sim/expressions.cpp:22–154,333–440,1081–1130` — lexer, arithmetic/boolean decisions and precedence.
- `engine/action/variable.cpp:19–133` — defaults and variable construction; `engine/player/action_variable.cpp:19–26` — initial values.
- `engine/action/action.cpp:4648–4689,4742–4763` — conditional calls versus non-returning run-list switches; `engine/player/player.cpp:14178–14265` — ordered variable/call selection and cycle rejection.
- `engine/action/action.cpp:2435–2468,2619,5362–5453` — target choice before if, min/max and strict candidate-order ties; [Arms source ledger](../../handoffs/arms-source-census.md).
- `profiles/MID2/MID2_Monk_Windwalker.simc:69`; `engine/class_modules/apl/apl_paladin.cpp:21–27,43,46–49` — concrete tests.
- [Contract](../../specs/simc-apl.md); [WW census](../../handoffs/ww-source-census.md).
