# Downstream spell scripts

`crates/server-spell-scripts/` holds source-qualified spell policies downstream of `server-combat`. This is a compile-boundary refactor, not a gameplay change. See [assembly and remaining slices](../wiki/systems/spell-script-crate.md).

## What it must do

- [ ] Preserve policy traversal, first-match lookup, additive hook order, cast depth, resource payment, RNG, targets and native payloads.
- [ ] Install the same registry in the server and transport-free simulation, including native fixture harnesses and cached simulation apps.
- [ ] Permit an existing module's policies and hooks to change without editing combat or a central policy table. A new downstream module needs only its file and one alphabetical registration line; registration must not reorder existing policy traversal.
- [ ] Keep combat independent of the downstream crate in the production dependency graph.
- [ ] Keep new dispatch-stage callback/lookup signatures independent, even when their current signatures match.
- [ ] Measure a moved-file touch using warm `cargo test --locked -p server-simulation --lib --no-run -j2` before and after the move; keep the same worktree-local target directory.

- Ordered downstream effect-handler lookup replaces native per-unit dispatch at the same handled/launch/impact boundaries. Additive target filters retain caster and immutable primary context after existing policy filters; they do not replace native target selection.

## How it works

- [Registry assembly and test boundaries](../wiki/systems/spell-script-crate.md)

## Implementation inventory

- `crates/server-combat/src/spell_cast/script_registry{.rs,/policy.rs}` — registry resource, hook signatures and relationship data types.
- `crates/server-combat/src/spell_cast/scripts{.rs,/policies.rs}` — retained combat dispatch and relationship execution.
- `crates/server-spell-scripts/src/{lib.rs,modules.rs,modules/}` — host plugin, ordered module slices and policy implementations.
- `crates/server-spell-scripts/src/test_support.rs` — real combat harness with the downstream registry installed before Startup.
- `crates/{server,server-simulation}/src/` — production and simulation assembly.

## Tests asserting this spec

- `crates/server-spell-scripts/src/tests.rs` — first-match, additive order, event arguments, empty registration and host installation.
- `crates/server-spell-scripts/src/runtime_stage_tests.rs` — native/proc/supply/policy order, launch snapshots carried to child and delayed stages, and exact named-aura-field classification.
- `crates/server-combat/src/spell_cast/{holy_words,prayer_of_mending,dummy3}_tests.rs` — native moved Holy charges, Surge usage, Prayer healing and jump behavior.
- `crates/server-simulation/src/dungeon_simulator/dps_parity/frost_mage_{proc,supply,modifiers,lance_cdr,shatter_followups}_tests.rs` — native migrated Frost execution boundaries.
- `crates/server-simulation/src/dungeon_simulator/dps_parity/{ret_auto_proc_tests.rs,outlaw_tests.rs,fury_rotation/mechanics_tests.rs}` — native migrated-hook behavior.
- `crates/server-combat/src/spell_cast/chrono_flame_tests.rs` and `crates/server-spell-tests/src/spell_cast/new_class_tests.rs` — history, native replacement, Living Flame healing/damage branches and refund behavior. All `devastation_` native cases gate shared Evoker flame moves.
- Native Hunter bounce, Shadow Volley and Scourge eruption fixtures gate their corresponding downstream modules. BM/Hunter, Affliction and Unholy probes use separate spec inputs; one entry's first measurement supplies its test environment.
- Existing native Ret/BM/Devastation/Windwalker/Warrior/Rogue policy fixtures continue to exercise relationship execution.

## Known gaps (current cycle)

- [ ] Move the remaining spell script implementations in later slices. Downstream modules now include all 37 relationship-policy rows, five original leaf modules, Essence Burst, Demonic, Divine Resonance, Evoker tier, Giantkiller and MM Explosive; typed dispatch preserves their original target-selection, damage, and after-resource boundaries.

## Out of scope

Gameplay changes, numerical DPS parity, build-profile changes, and migration of unrelated combat systems.
