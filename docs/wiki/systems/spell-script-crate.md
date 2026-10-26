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

## Ret damage, tier and echoes

Ret damage, Empyrean Legacy, tier and echo policies own their rows and per-entity state downstream. Admission snapshots still precede generic OnCast/payment. The mastery correction remains between Giantkiller and Shadow Weaving. Echoes, tier consumption, Purpose generation and deferred Legacy execution retain distinct foreground/background boundaries. Script-owned driver/charge IDs use the native proc-ownership gate. Consecrated Blade still calls combat's existing ground-field helper until the ground-pulse module moves; there is no copied field implementation.

New dispatch stages use independent callback and lookup aliases, even when their current signatures match, so changing one stage cannot silently change another stage's callback contract.

## Frost and empower releases

Frost DK's earned Killing Machine/Rime/Obliteration state and Frost Mage's owned Freezing/Shatter plans live downstream. Frost DK's white-hit callback follows Ret's reset callback; launch consumption stays immediately after Elemental. Freezing's direct/periodic callbacks retain their original positions. Evoker empower release construction, Animosity progress and converted-tick bonus also live downstream; APL/fixture previews query the installed registry for the same native release metadata used by completion, without a second implementation.

## Elemental launch state

Master of the Elements damage rules, consumption and Elemental Blast's missing-buff selection live downstream. Pet-set capture still precedes the consuming modifier snapshot; completion consumes the captured actual aura instance before Frost/pet-set launch callbacks. Elemental Blast's pre-launch grant still follows combo tracking and precedes native judgment handling, while ordinary post-launch grants retain their original position. Native proc veto registration remains before late relationship registration.

## Restoration healing scripts

Clearcasting/Omen, Soul of the Forest and Swiftmend live downstream together. Omen's proc filter retains its native registration slot. Swiftmend consumes the original ordered owned HoT before granting the next-heal buff; Regrowth/Rejuvenation consume that buff only after the heal. Direct healing and periodic aura snapshots retain separate stages so the persistent bonus survives later ticks without rereading a consumed aura.

## Guardian impacts and stacking summons

Guardian impact target caps/child edges and successful-cast counter/summon tables live downstream. Guardian callbacks query the original innermost resolving spell ID rather than holding or copying combat's frame state. Native after-cast dispatch still selects one handler; summon counters, refunds, random choices and duplicate redirects retain their original traversal and RNG calls.

## Fire proc state

Hot Streak's producer table, Heating Up protection, admitted-proc consumption and Pyroclasm snapshots live downstream with Fire's Sphere/Fired Up/Cascade/Hyperthermia feedback. The same registered producer table classifies native in-flight actions and actual damage outcomes. Flamestrike still snapshots chance before consumption and rolls once after the cast. Fire feedback and Meteorites retain their separate actual-hit stages; native aura lifecycle registration remains after Ret Sentence.

## Pet-set relationship tables

The last class-specific table nested under generic relationship execution, pet-set Stomp/Cobra Fang rules, lives downstream. Generic policies retain only ordered relationship execution and call the registered tail handler at the original cast/first-hit boundary. Cobra Fang still snapshots before consuming damage modifiers and removes the earned aura after successful launch, after Frost callbacks and before execution counters.

## Native resource and effect overrides

Barbed Shot's focus children, Stormkeeper's authored use count, Feral Berserk's extra combo point, Rewind's non-raid healing factor and Enduring Torment's specialization filter live downstream. Their stages retain the original aura-creation, positive root self-energize, named-healing and effect-filter boundaries. Charge transforms preserve the native input, point bonuses add, healing factors multiply, and effect filters can only veto an effect. Unregistered stages have neutral results.

## Linked Enhancement runtime

Maelstrom Weapon, native weapon imbues, Doom Winds pulses, Thorim selection, spaced spending decks, Conductive Energy, Voltaic/Fire Nova, Burning Core and delayed Storm Unleashed pulses live together under the downstream Enhancement module. Their original aura-registration order, successful foreground consumption, per-cast snapshot markers, post-resolution Rod selection and tier-before-Ride launch order remain unchanged. Combat exposes its resolved-damage view and generic proc-admission snapshot helper rather than copying resolving frames or charge handling downstream.

## Spellslinger conjures

Spellslinger's Arcane/Frost source tables, delayed per-conjure rolls, Ray tick window and actual Orb recharge adjustment live downstream. The conjure cast handler remains after all generic relationship cast handlers and before ground-field handlers. Splinter direct damage still adjusts Orb before melee/combo feedback; Ray ticks still conjure after periodic relationships and before item feedback. This source-execution module is separate from the already-moved Splinter impact module.

## Frost launch procs and delayed supply

`modules/frost_runtime/` owns Frost launch-aura snapshots, Flurry's inherited child scalar, Ice Lance's native payload, Orb and cast proc producers, Augury's PRD state, and delayed Brain Freeze/Splinter callbacks. Native after-cast handling still precedes proc consumption, supply generation and generic cast policies. Launch supply remains before execution counters; delayed callbacks retain their original queued-trigger position and current-target rule. Each new stage has its own alias. The existing Spellslinger impact module is unchanged.

A warm `server-simulation --no-run` touch of the moved supply source rebuilt combat, scripts and simulation in87.72s before extraction (including a package-cache wait), then only scripts and simulation in5.45s after extraction. These are compile observations, not gameplay acceptance; native Frost and registry gates run in the merge train.

## Holy Words and Prayer runtime

`modules/holy_runtime/` owns Surge of Light, Apotheosis charge restoration, Flash/Prayer cooldown reductions, Prayer of Healing's injured-ally plan, and Prayer of Mending's native stacks, OriginalCaster and random injured/player-priority jumps. Support aura registration stays after Abundance and before Earthen Rage. Named aura fields classify only the same authored effect indices; they do not add effects. Native registry lookups replace the original first-match cases without changing payment, flight or healing.

## Shared Evoker flames

`modules/evoker_flames.rs` and its Chrono leaf own Living Flame's ally/enemy children and Energizing Flame refund, plus Chrono's per-caster effective health history, native child validation and capped repeat. Health history observes each combat-log event at the original post-meter/arena slot. Effect-hit and target-validation lookup retain their original behavior. Combat exposes its existing power, versatility and fixed-heal helpers; no damage or healing calculation is duplicated.

## Bounded native projectile scripts

`hunter_bounces.rs` owns shared Hunter bounce policy data, native target round-robin order, delays and ramp scalars. `shadow_bolts.rs` owns Voidform's initial Volley, Volley's primary/caster plan and native50ms bolt queue. `unholy_eruption.rs` owns Scourge Strike's owned periodic snapshot reads, native pre-mitigation calculation and missing-only Virulent Plague application. They use the existing effect-handler, effect-hit and target-plan slots; already-moved spec modules are unchanged. The generic `DamageInput` and pre-mitigation helper remain in combat.

## Tests

The server's public native harness and simulation's harness alias install the plugin before the harness runs Startup. Simulation installs it on every new app, including apps that reuse cached aura catalogs. Combat's own unit-test build compiles the exact downstream module sources against its local test-crate types; it does not add a production dependency or duplicate policy data. Ordinary simulation builds do not compile those test-only inclusions.

## Sources

- [Behavior and build-speed contract](../../specs/spell-script-crate.md)
- `crates/server-spell-scripts/src/{lib.rs,modules.rs,test_support.rs}`
- `crates/server-combat/src/spell_cast/script_registry{.rs,/policy.rs}`
- `crates/server-simulation/src/dungeon_simulator/runtime.rs`

## See Also

- [[spell-casting]] — retained execution engine and native relationships
