# Downstream spell scripts

`server-spell-scripts` depends on `server-combat`, not the reverse. The server and transport-free simulation install `SpellScriptsPlugin` before Startup. Combat plugins initialize an empty `SpellScriptRegistry` without replacing a host-installed resource.

## Registry

Each downstream module exports a const `HOOKS` slice. `src/modules.rs` supplies the ordered slice of slices; existing policy additions change only their module. The authoritative lookup and dispatch-order contract is documented once in [`script_registry.rs`](../../../crates/server-combat/src/spell_cast/script_registry.rs). Policy rows retain their original traversal order, including the separated Windwalker groups.

The registry carries static slices and function pointers, not a dependency from combat to script implementations. Iterators copy static registrations out of the resource before callbacks mutate the World. Relationship execution remains in combat; all 37 former `policies.rs` rows live in downstream per-module tables.

## First slice

Complete moved modules: cast resets, after-cast native aura triggers, weighted aura rolls, cast grants and paid-resource cooldowns. Their policy rows, native-value reads and callbacks move together. Other `spell_cast/scripts` implementations remain in combat for subsequent slices; the full directory move is not complete.

## Tests

The server's public native harness and simulation's harness alias install the plugin before the harness runs Startup. Simulation installs it on every new app, including apps that reuse cached aura catalogs. Combat's own unit-test build compiles the exact downstream module sources against its local test-crate types; it does not add a production dependency or duplicate policy data. Ordinary simulation builds do not compile those test-only inclusions.

## Sources

- [Behavior and build-speed contract](../../specs/spell-script-crate.md)
- `crates/server-spell-scripts/src/{lib.rs,modules.rs,test_support.rs}`
- `crates/server-combat/src/spell_cast/script_registry{.rs,/policy.rs}`
- `crates/server-simulation/src/dungeon_simulator/runtime.rs`

## See Also

- [[spell-casting]] — retained execution engine and native relationships
