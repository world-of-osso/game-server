# Downstream spell scripts

`server-spell-scripts` depends on `server-combat`, not the reverse. The server and transport-free simulation install `SpellScriptsPlugin` before Startup. Combat plugins initialize an empty `SpellScriptRegistry` without replacing a host-installed resource.

## Registry

Each downstream module exports a const `HOOKS` slice. Add a new file and one alphabetical `register_modules!` line in `src/modules.rs`; existing policy additions change only their module. Each registration carries its historical dispatch position, so declaration order does not reorder policy groups. New modules use an unused position after existing modules unless their stage requires another explicit boundary. Const assembly rejects duplicate positions and preserves the ordered slice of slices. The authoritative lookup and dispatch-order contract is documented once in [`script_registry.rs`](../../../crates/server-combat/src/spell_cast/script_registry.rs). Policy rows retain their original traversal order, including the separated Windwalker groups.

The registry carries static slices and function pointers, not a dependency from combat to script implementations. Iterators copy static registrations out of the resource before callbacks mutate the World. Relationship execution remains in combat; all 37 former `policies.rs` rows live in downstream per-module tables.

## First slice

Complete moved modules: cast resets, after-cast native aura triggers, weighted aura rolls, cast grants and paid-resource cooldowns. Their policy rows, native-value reads and callbacks move together. Essence Burst and Havoc's Demonic transformation also live downstream: the former uses first-match native after-cast dispatch; Demonic is the last registered OnCast callback, before combat's combo tracking and elemental launch. Other `spell_cast/scripts` implementations remain in combat for subsequent slices; the full directory move is not complete.

## Target and damage stages

MM Explosive owns a downstream target-plan lookup and primary-periodic-damage callback. Combat calls those at the original selection/tick positions, including the primary tick's amount and trigger depth. Giantkiller's multiplier runs at its original effects boundary. Evoker tier runs after resource-value after-cast callbacks; Divine Resonance uses native after-cast lookup. These modules own their native IDs and tables; extending them does not edit combat dispatch. Target-plan lookup is first-match, while damage callbacks and post-resource callbacks retain registration order.

## Aura application and spender stages

Germination's redirect runs before a planned aura application; a consumed application stops at the first redirect. Ret Divine Purpose runs after foreground Crusade spender stacks and at the corresponding background Storm callback, before Legacy/echo dispatch. The policy still reads actual Holy Power cost-pool presence, selected traits, and native aura chance downstream.

## Tests

The server's public native harness and simulation's harness alias install the plugin before the harness runs Startup. Simulation installs it on every new app, including apps that reuse cached aura catalogs. Combat's own unit-test build compiles the exact downstream module sources against its local test-crate types; it does not add a production dependency or duplicate policy data. Ordinary simulation builds do not compile those test-only inclusions.

## Sources

- [Behavior and build-speed contract](../../specs/spell-script-crate.md)
- `crates/server-spell-scripts/src/{lib.rs,modules.rs,test_support.rs}`
- `crates/server-combat/src/spell_cast/script_registry{.rs,/policy.rs}`
- `crates/server-simulation/src/dungeon_simulator/runtime.rs`

## See Also

- [[spell-casting]] — retained execution engine and native relationships
