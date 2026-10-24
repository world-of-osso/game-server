# Literal SimC APL decisions

Shared native decision evaluation interprets pinned SimC76f8118 conditions and source-ordered action lists. Native spellbook, castability and numeric combat data remain authoritative and caller-owned. [Implementation details](../wiki/systems/simc-apl.md).

## What it must do

- [x] Interpret numeric truth, OR/AND/NOT, arithmetic and comparisons with pinned SimC precedence. Single `%` divides; `%%` is modulo.
- [ ] Read explicit native facts for buff stacks/remains/up, cooldown remains/charges/duration, resources, talents, equipment/trinkets, target count/TTD, fight remains, raid-event adds, dots/debuffs and gcd. Unknown operands fail explicitly, including in otherwise inactive condition branches; never silently become false.
- [x] Bind action-dependent facts (`combo_strike`, plain `gcd`, action minimum GCD) to each source row, including variable/call control actions. An actor-wide value is not equivalent.
- [x] Execute declared variables using source `value`, `setif`, `value_else`, `reset` and numeric `default` semantics.
- [x] Preserve literal action order and conditional `call_action_list` decisions; native castability selects the first usable action. An exhausted child list returns control to its caller.
- [x] Implement conditional `run_action_list` switches that never resume the caller after child exhaustion; reject unknown lists.
- [x] Select explicit native targets with `target_if=min/max:<expr>` before the row's `if`; preserve native tie order. Unknown target operands/adapters fail explicitly.
- [x] Scope a run row's selector to that control action. Child actions retain their own targets; run selection must not retarget them.
- [x] Sample native cast readiness independently for each action/target pair. Harmful precasts use the actor's actual selected enemy when combat candidates have not yet been supplied.
- [x] Deliver Player literal spells through ordinary Player requests with the selected explicit target, rechecking knowledge and rejecting expired identities rather than redirecting. Preserve selected and AA targets; retain native creature dispatch for WW followers.
- [x] `retarget_auto_attack` changes only the running AA victim, preserving both swing timers and the actor's selection. An already-running AA must not restart onto the primary target.
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
- `crates/server/src/simc_apl/run_tests.rs`, `target_tests.rs` — non-returning runs and explicit min/max target decisions.
- `crates/server/src/lib.rs` — engine-free evaluator export.

## Tests asserting this spec

`crates/server/src/simc_apl/tests.rs`; eleven expression/program cases pushed ignored before implementation: genuine11RED atc0cf6eed6→11GREEN ataff61cbab (exit0, zero compiler warnings,1m03 compile). Cases enabled afterward; invalid-control case additionally checks sourced call recursion. Two action-context REDs at2e6f242ce→all13GREEN atcd3c7e427, zero compiler warnings; context cases enabled afterward.

Run/target source extension: genuine14PASS/4RED at a446f0634; explicit `Action.target` and candidate-ID API scaffold then genuine16PASS/8RED at2088713a6 (no compiler errors). Ate7fdc4dd1 all24 shared interpreter tests passed, but the real pinned SimC run-selector probe invalidated the inherited-child oracle; that packet is withdrawn. Corrected binary-backed oracle:23PASS/1 genuine assertion RED at2d6a66259 →24/24GREEN atad720fc3c (28.61s compile,0compiler warnings/0ignored). Native per-target dispatch remains caller-owned; [isolated proof packet](../handoffs/shared-run-target-proof-ledger.md).

Player execution boundary: seven actual native cases at4f91688e8 give1WW controlPASS/6genuine assertionRED (34.94s runtime). At e99b879e8 after hp1026, all7 pass (27.17s runtime,18m08 compile,0compiler warnings). [Scoped proof](../handoffs/literal-player-dispatch-proof-ledger.md); broader integration remains lead-owned. [Shared machinery](../wiki/systems/simc-apl.md#shared-literal-fixture-machinery).

## Shared native facts

`crate::simc_apl_native` binds gameplay state to the engine-free traits without moving the interpreter into the server binary. Explicit alias registries identify native actions, buffs, talents and resource types; absent aliases fail, never become false. Buff up/stack/remains, real cooldowns/charges/durations, displayed resources, action-bound GCD and engaged enemy counts come from native state. The fixed-time parity encounter supplies elapsed/fight-remains and fight style; main and timed adds expose their explicit individual remaining lifetime/role. Target Health reads the selected native entity; action-explicit targets override selection. Bound dots/debuffs read only the actor's own actual auras on that target. Explicit fixture add-event state supplies timing/count; absent-event operands retain pinned max-timespan semantics. Dungeon TTD/fight-remains/add timing remain explicit unavailable predicates, not estimates or a fallback. [Lifecycle/target14-case GREEN and remaining coverage](../handoffs/add-lifetimes-target-facts-proof-ledger.md). Spec adapters add only spec-specific facts.

Native behavioral tests: `simc_apl_native/tests.rs` (earned Hit Combo and Chi, real Provoke cooldown, real Roll charges, actual Charge Rage display conversion, fixture clock and unavailable dungeon predicates). Six pushed ignored native cases produce genuine6RED at5924496f9 (13.24s test/8m18compile/exit101; three scaffold dead-field warnings). Atdc1ee9781 the same six native cases pass6/6 in17.40s (7m37compile),exit0,zero compiler warnings; tests enabled afterward without body changes ([native proof ledger](../handoffs/shared-native-apl-proof-ledger.md)). Current native fact seam is fixture-only (`cfg(test)`), matching the approved explicit parity Fixture driver boundary; live dungeon APL/TTD is not enabled.

## Assigned native fact extensions

- [ ] `talent.<name>.rank` reads selected native total ranks, not learned-spell membership.
- [ ] `buff.<name>.at_max_stacks` follows earned native stacks and the instance's modified cap.
- [ ] `spell_haste` is the actual derived spell-duration factor; `execute_time` observes ordinary cast/GCD or channel duration.
- [ ] Contextual `charges`, `charges_fractional` and `full_recharge_time` follow the proposed action's actual native category.
- [ ] Contextual `ticks_remain` counts future scheduled native ticks; `action.<name>.ready` includes native admission, while `usable_in` is remaining cooldown time.

Ignored native regressions in `simc_apl_native/{extension_tests,action_extension_tests}.rs` precede implementation. At89f985420 all seven reach genuine unknown-operand/property RED in18.52s with native prerequisite state intact; GREEN remains pending. Source: pinned SimC76f8118 `player.cpp11822,12369`, `buff.cpp457–462`, `action.cpp3266–3333` and `dot.cpp526–532`. These shared predicates remain fixture-only; no Dev-local duplicate resolver.

## Native haste composition boundary

- [ ] Native haste rating and independent combat-speed aura193 effects compose multiplicatively, including ordinary cast time and GCD consumers. Removing actual buffs restores the original rating-derived speed. Keep existing native rating diminishing returns and data amounts.

TC a352b1fa `Player.cpp5334–5361` applies old/new haste rating through `ApplyCastTimePercentMod`; `SpellAuraEffects.cpp4531–4551` applies aura193 through the same multiplicative cast/attack speed paths. Regression `dps_parity/haste_tests.rs` uses actual1467 BiS gear plus native Power Infusion10060 and Heroism32182 aura amounts; no injected ratings, stats or coefficients. Feral's separate100%-uptime Frantic Momentum/Savage Fury proc eligibility belongs to its spec owner, not to rating conversion.

## Explicit native target context

- [x] Native fact adapters accept an actual current target and ordered native target universe, separate from an explicit proposed `Action.target`. Invalid/absent explicit context fails rather than borrowing another selection. Candidate selection preserves native order and ordinary per-action target eligibility, not just first-target readiness.
- [x] Owned `active_dot` counts use actual active native aura instances across that universe; foreign caster instances do not count. Unknown aliases fail.
- [x] Fixture movement-event absence uses pinned no-event values; actual movement never invents a configured raid event. Dungeon timing remains explicitly unavailable.
- [x] Fire Breath refresh decisions consume freshly resolved native empower-duration data and a sourced refresh policy, not raw child duration or current aura carry. Actual rank1 native20s versus raw3572092s refresh-window case is qualified.
- [ ] Positive selected-target casting reaction requires a qualified native snapshot. Actual absence of CastState is known false; active casting stays NATIVE_CASTING_REACTION_UNAVAILABLE rather than inventing a timer.

[Target-context API proof and exact remaining gaps](../handoffs/native-target-context-proof-ledger.md).

## Configured movement distance

- [x] `movement.distance` reads the actor's explicit fixture `distance_to_move`, matching pinned SimC76f8118 `player.cpp12146–12148`. Never substitute target range, actor position or native travel distance.
- [x] Patchwerk parity setup explicitly configures zero when no movement event exists, matching `player.cpp1288`; explicitly supplied positive values and subsequent state updates remain distinct from geometry.
- [x] Missing configuration fails `NATIVE_MOVEMENT_DISTANCE_UNAVAILABLE`; dungeon callers fail `DUNGEON_MOVEMENT_DISTANCE_UNAVAILABLE` rather than borrowing fixture metadata.

- [ ] `movement.remains` divides positive configured distance by current native run speed; with zero distance and the admitted no-movement-event fixture it is zero. `movement.speed` reports native base run speed times current native run/aura modifiers even while idle, not last movement stride speed.
- [ ] Missing/dungeon movement state remains an explicit error for remains/speed as for distance; no target range or invented timing.

`MOVEMENT_EVENT_SCHEDULING_UNREFERENCED` remains open: all pinned references are Patchwerk single-target or default-style adds with no movement events. Do not add an unpaired moving-style selector, event scheduler or active movement-buff model until a pinned reference supplies that contract.

## Known gaps (current cycle)

- [ ] Wire native kit adapters; library expression/variable/list decisions have11-case GREEN, not full native rotation proof.
- [ ] Native WW and Ret fact adapters and literal per-spec action policies; Ret adapter belongs to14752.

## Out of scope

- Player AssistedCombat adapter, native-book duplication, invented facts or spell coefficients, and heuristic substitutes for unsupported conditions.
