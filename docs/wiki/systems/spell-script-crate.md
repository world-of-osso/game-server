# Downstream spell scripts

`server-spell-scripts` depends on `server-combat`, not the reverse. The server and transport-free simulation install `SpellScriptsPlugin` before Startup. Combat plugins initialize an empty `SpellScriptRegistry` without replacing a host-installed resource.

## Registry

Each downstream module exports a const `HOOKS` slice. Add a new file and one alphabetical `register_modules!` line in `src/modules.rs`; existing policy additions change only their module. Each registration carries its historical dispatch position, so declaration order does not reorder policy groups. Independent new modules can share a trailing position without coordinating a counter; equal positions retain alphabetical declaration order. Use distinct positions only when callback order requires it. Const assembly preserves the historical ordered slice of slices. The authoritative lookup and dispatch-order contract is documented once in [`script_registry.rs`](../../../crates/server-combat/src/spell_cast/script_registry.rs). Policy rows retain their original traversal order, including the separated Windwalker groups.

Run `python3 scripts/format-spell-scripts.py` to format, or add `--check` to verify formatting. Cargo's formatter does not discover declarations inside `register_modules!`; this command runs Cargo formatting plus an explicit rustfmt pass over leaf and nested module sources. `run-tests.sh` enforces the check before running tests.

The registry carries static slices and function pointers, not a dependency from combat to script implementations. Iterators copy static registrations out of the resource before callbacks mutate the World. Relationship execution remains in combat; all 37 former `policies.rs` rows live in downstream per-module tables.

## First slice

Complete moved modules: cast resets, after-cast native aura triggers, weighted aura rolls, cast grants and paid-resource cooldowns. Their policy rows, native-value reads and callbacks move together. Essence Burst and Havoc's Demonic transformation also live downstream: the former uses first-match native after-cast dispatch; Demonic is the last registered OnCast callback, before combat's combo tracking and elemental launch. Other `spell_cast/scripts` implementations remain in combat for subsequent slices; the full directory move is not complete.

## Target and damage stages

MM Explosive owns a downstream target-plan lookup and primary-periodic-damage callback. Combat calls those at the original selection/tick positions, including the primary tick's amount and trigger depth. Giantkiller's multiplier runs at its original effects boundary. Evoker tier runs after resource-value after-cast callbacks; Divine Resonance uses native after-cast lookup. These modules own their native IDs and tables; extending them does not edit combat dispatch. Target-plan lookup is first-match, while damage callbacks and post-resource callbacks retain registration order.

## Aura application and spender stages

Germination's redirect runs before a planned aura application; a consumed application stops at the first redirect. Ret Divine Purpose runs after foreground Crusade spender stacks and at the corresponding background Storm callback, before Legacy/echo dispatch. The policy still reads actual Holy Power cost-pool presence, selected traits, and native aura chance downstream.

## Hit and admission stages

Elemental Ascendance uses ordinary hit lookup; Voltaic Blaze owns effect-hit lookup, contextual impact, target limiting and guaranteed critical chance. Touch of Death owns admission and damage amount; Snakeskin Quiver rolls after actual Auto Shot launch. Target handling copies the installed registry so a moved effect-hit callback is still recognized when selecting effect targets, not only when executing impacts. Combat retains native lookup order around each registry boundary.

## Aura registration

Feral/Frantic Frenzy and periodic Icicle counters register their complete native aura lifecycle callbacks through `EarlyAuraRegistration`, at the original start of aura-script registration. Their effect IDs, per-entity state, target ownership and scheduled children live downstream. Divine Toll's target cap and contextual child dispatch also live downstream and use the existing area-filter and contextual-hit slots.

## Shadow launch and mastery

Shadow's complete module owns Tentacle Slam's effect-hit dispatch, consumed effect77 data, 500 ms launch override, and Shadow Weaving's target-owned-DoT multiplier. Combat retains its native delay when no module registers an override. The mastery factor stays at the original bonus-caster damage boundary, separate from Giantkiller's factor position.

## Proc admission and periodic completion

Lava Surge owns normalized Flame Shock tick rolls and deferred Lava Burst charge restoration downstream. Its proc IDs register admission snapshot and unadmitted-charge-preservation tables; combat still checks native cast-time class masks and actual aura instance IDs. The normalized tick callback runs only after periodic damage and before retained Ret periodic hooks.

## Execution and landed-melee proc stages

Foreground special procs, execution counters, landed-impact PRD and repeating-melee resets own their tables, counters and RNG downstream. Producers still run before foreground counter consumption; execution counters retain the original post-launch position. Ret's white/replacement-hit reset stages stay separate from arbitrary direct hits. Static proc ownership prevents generic proc hints from consuming a script-owned ICD.

## Effective spend and residual copies

Effective resource-value rules and secondary residual-copy tables live downstream. Paid resource remains unchanged; a selected value rule can supply the native pool maximum solely to effective-spend consumers. Producers retain both ordinary after-cast and triggered-cast positions. Consumed value sources are removed only at the original successful-execution boundary. Residual copies keep separate direct-spell and landed-white-hit callbacks and retain the source's actual amount and cast depth.

## Successful casts and Marksmanship

Successful-cast relationship rows, Marksmanship target plans/residuals and Sentinel marks live downstream together. Successful-cast hooks retain their foreground-only position, and Sentinel's forced mark still precedes the relationship row traversal. Cast readiness, hardcast crit guarantees and consumed-modifier snapshots query downstream predicates without changing native payment or cooldown storage. Rapid Fire's original periodic and natural-end aura callbacks now register with the module; its native period still supplies seven shots. Resolved damage visits Master Marksman before Sentinel and before the retained Explosive primary-tick stage.

## Complete Havoc runtime

Havoc's Initiative targets, Burning Blades residuals, Demonsurge uses and Meta expiry hooks live under the downstream Havoc module. Its runtime pre-cast callbacks still precede the Demonic transformation, and channel-duration extension still follows transformation haste. The same downstream component state supplies native APL availability queries. Eye Beam/Gaze periodic child dispatch and learned-rank critical chance also live downstream; their callbacks retain the real channel-state guard and native child geometry.

## Tests

The server's public native harness and simulation's harness alias install the plugin before the harness runs Startup. Simulation installs it on every new app, including apps that reuse cached aura catalogs. Combat's own unit-test build compiles the exact downstream module sources against its local test-crate types; it does not add a production dependency or duplicate policy data. Ordinary simulation builds do not compile those test-only inclusions.

## Sources

- [Behavior and build-speed contract](../../specs/spell-script-crate.md)
- `crates/server-spell-scripts/src/{lib.rs,modules.rs,test_support.rs}`
- `crates/server-combat/src/spell_cast/script_registry{.rs,/policy.rs}`
- `crates/server-simulation/src/dungeon_simulator/runtime.rs`

## See Also

- [[spell-casting]] — retained execution engine and native relationships
