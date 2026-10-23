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
- [ ] Expose target-specific readonly resource-limited admission for a chosen power pool. Compare that pool's native maximum locally; never grant or temporarily replace resources. Preserve other pools, health and every other native cast constraint; an unaffordable maximum cannot qualify.
- [x] Evaluate concrete default WW and Ret source lines, including resource/buff stacks, Execution Sentence and trinket synchronization.

## Native action permission

- [ ] Ordinary follower kits and literal fixtures use the same native action admission. A row is allowed only by the actor's actual known spell, selected trait visible/override action, or an explicitly sourced known-parent action policy.
- [ ] Admission returns the actual cast spell, preserving selected trait replacements and native form/aura overrides; passive talent IDs must not be cast instead of their visible actions. No controller row grants new KnownSpells membership and NPC identity does not bypass the controller gate.
- [ ] All11native starter profiles across levels1–90 retain their published cast actions, including Feral Berserk/Cat Swipe, Demonology Hand of Gul'dan/Demonbolt and Elemental Ascendance. Unselected Incarnation remains rejected. Exact custom reference loadouts retain their own selected books, not the default kit's talents.

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

`crate::simc_apl_native` binds gameplay state to the engine-free traits without moving the interpreter into the server binary. Explicit alias registries identify native actions, buffs, talents and resource types; absent aliases fail, never become false. Buff up/stack/remains, real cooldowns/charges/durations, displayed resources, action-bound GCD and engaged enemy counts come from native state. The fixed-time parity encounter supplies elapsed/fight-remains/target-TTD and fight style; dungeon TTD/fight-remains remain explicit unavailable predicates, not estimates or a fallback. Spec adapters add only spec-specific facts.

Native behavioral tests: `simc_apl_native/tests.rs` (earned Hit Combo and Chi, real Provoke cooldown, real Roll charges, actual Charge Rage display conversion, fixture clock and unavailable dungeon predicates). Six pushed ignored native cases produce genuine6RED at5924496f9 (13.24s test/8m18compile/exit101; three scaffold dead-field warnings). Atdc1ee9781 the same six native cases pass6/6 in17.40s (7m37compile),exit0,zero compiler warnings; tests enabled afterward without body changes ([native proof ledger](../handoffs/shared-native-apl-proof-ledger.md)). Current native fact seam is fixture-only (`cfg(test)`), matching the approved explicit parity Fixture driver boundary; live dungeon APL/TTD is not enabled.

## Known gaps (current cycle)

- [ ] Wire native kit adapters; library expression/variable/list decisions have11-case GREEN, not full native rotation proof.
- [ ] Native WW and Ret fact adapters and literal per-spec action policies; Ret adapter belongs to14752.

## Out of scope

- Player AssistedCombat adapter, native-book duplication, invented facts or spell coefficients, and heuristic substitutes for unsupported conditions.
