# Literal Feral APL qualification

## Scope

Fixture-only spec103 driver using shared `Program`, native facts, selected action resolution and target/resource readiness. Preserve the exact pinned profile, native follower identity, costs, damage coefficients and production kit. Missing native mechanisms remain named; this ledger does not certify DPS parity.

`feral-literal-apl` is separate from the fixed Bite candidate `feral-druid-parity` (runtime7ea788a8b, docs121a3b77e). Parent requested master→Bite only after ft1083/770 lands. Literal development merged master5d39f490af at5ad76e8de and picked approved min/max06c99c40c/45bae2933/585e2ca42 as96d0f6f0c/8d2a28e0d/c35c1d7ab. The merged base and bindings compiled at e3648f4f7; one native binding test passed17.05s. Compiler reported one inherited unused `navigation` import, removed at3c51106ad without suppression; that removal awaits the next compiler run. The server String adapter delegates the canonical typed optional resolver, preserving one admission policy and the `server-simulation → server` dependency.

## Pinned source

- SimC76f8118cf3f7f41dad20708e8328e1f4ab2deebc; MID2_Druid_Feral_Wildstalker, level90/night_elf; profile SHA9f419caddee5e8cae94856a05b27bd276b390c6ac6c83d4d6449b1eccd093797.
- Generated `engine/class_modules/apl/druid/feral_apl.inc`:73rows, SHA d4a1274f99c10bdde08d5a36832b2b20b898a09b35d2d10258aca3fa7b6fcbfb.
- Actual prepared profile:74rows, adding leading `actions.precombat=snapshot_stats`. Normalized remaining73rows match the generated source exactly. Preserve this addition; it is not an adapter-invented action.
- Tracked `crates/server-simulation/src/dungeon_simulator/dps_parity/feral_rotation/76f8118.simc` is the exact74row export, SHA7c9ca9bb542a8cd7ab1b5b0e9ee4135c592d64f4c634f18a007167b4cf99eb09. Eight list counts: precombat10, default14, aoe_builder11, aoe_finisher6, builder6, cd_variable10, cooldown14, finisher3.

Artifacts: engine `data/diagnostics/feral-parity-2026-10-23/literal-source/{inventory,profile-source}.proof.json`. Source counts are inventory evidence, not behavioral tests.

## Capability matrix

| Source surface | Current state | Proof level / owner |
|---|---|---|
|15variable,6call and1pool rows|Canonical floor/pool/readonly-max interfaces consumed from master1fb; Feral consumer hook still pending|Source inspection;14766 owns extensions|
|10target_if rows:5min,5boolean|Canonical FIRST/min/native-target context consumed from master1fb; original parser RED preserved|Inherited ta1068;14766 owns boolean selection|
|39native spell/racial rows|Aliases and Rake/Moonfire refresh bindings added; native target geometry/consumer not yet wired|Feral-owned; native binding1PASS17.05s, no literal execution claim|
|11item/potion/external rows|Shared equipped metadata available; no provisioned potion/external provider|Consume shared equipment; report provider gaps explicitly|
|2autoattack/snapshot rows|Shared execution exists; Feral consumer not wired|Driver integration behavioral RED below|

Numeric `<?`/`>?` are present on development base from bt1082, not a private expression implementation. `max_energy=1` and `pool_resource,for_next=1` remain14766-owned; master1fb now supplies their canonical implementation plus c4f2e3212's reporter resource hooks. No unsupported token may disappear behind a false branch or silently become zero.

Fact dependencies:

- `dot.primal_wrath` aliases actual owned Rip1079 per SimC druid4745; test-onlyffa74a027 reproduces missing alias1FAIL17.37s before this one-entry binding. Parent285381 is not a separate dot aura. Rip/Primal fresh refresh policies remain explicit gaps until shared CP-prepared readers/source policy land; no half-duration constant or fake aura. At db56e7e68 the owned-Rip alias plus existing binding/refusal control passes1case11.63s with clean22.32s compile (`primal-dot-alias-green/`).
- e3648f4f7 binds Rake155722 and Moonfire155625 through `native_dot_spell`/Pandemic. Exact-profile owned-aura test proves absent→fresh refreshability and active_dot; Rip1079 deliberately lacks a refresh resolver and returns `NATIVE_DOT_REFRESH_UNAVAILABLE`, because native duration669 is4000base/24000max/4000perResource. The canonical CP-prepared reader is14775-owned.
- Master ta1068 supplies explicit ordered targets, owned dots/refresh policies, active_dot and actual fixture add-event components. Bind Feral aliases and consume these; do not fabricate wave timers or global aura matches.
- hero_tree:14116/hf1080 candidate6e4427191, parent gate pending. No local hero-tree fact.
- dot/action persistent multipliers:14775 owns the shared snapshot/API (frozen RED2bc981067); Feral eligibility stays separate. No new persistent snapshot fields here.
- 14781: prev_gcd lands with hs1063 and reads successful GCD root identities through action bindings. Exact consumers are prev_gcd.1.rip, prev_gcd.1.ferocious_bite and prev_gcd.1.primal_wrath. Pending additions: buff.clearcasting.react, buff.tigers_fury.duration and action.convoke_the_spirits.execute_time.

## Native Bite maximum-Energy reader

Parent approved exactly one public read-only native reader: actual modified22568 Energy cost plus its existing extra-Energy cap. At test-onlyd987d0c69, the real selected-profile `maximum_energy` request fails with unknown native operand: genuine1FAIL10.69s, clean5m12s build (`bite-maximum-energy-red/`). The case also defines the native391882 cost-modifier control and checks repeated reads preserve all pools/history; no proc generation is inferred.

`spell_costs` delegates numeric calculation to `applicable_costs`, then checks affordability. The reader reuses that same numeric producer directly, so low current Energy/CP does not prevent reading the cost. The exact existing cap computation is extracted once and shared with Bite's existing damage snapshot; no new formula or pool mutation. `spell_cast::read_ferocious_bite_maximum_energy` is the single cross-crate public export; the implementation module stays private. Feral action facts delegate to it only for the proposed Bite action.

Existing Bite tests are unchanged. Incarnation native-versus-SimC maximum-Energy semantics stay an explicit gap; this is a native numeric reader, not a reference-policy override. GREEN pending.

## Full literal source consumer

Development ae6c03e62 merges exact master1fb0230ad, retaining canonical generic FIRST/floor/pool/precombat and readonly-max APIs plus the approved min/max operators. `feral_rotation/source.rs` loads the unchanged74row profile through shared `Program`; it validates only this adapter's declared actions/options, never interprets expressions or rewrites priorities. Concrete source-order finisherRip/builderRake decisions pass2cases0.00s at20eb39646; canonical merged generic reporters pass4cases0.00s. Source build wrapper614.49s, `literal-source-consumer/` preserves command/exit/count receipts. No DB fixture is required by these pure decision tests, and they do not certify native action execution. The native driver is still unwired; persistent multiplier, CP-prepared refresh and remaining published fact dependencies stay explicit.

The clean Rake branch394369da5 is frozen and parent-owned (rk1097); it does not depend on this development merge or source consumer.

## Engine-free reporter REDs

`crates/server/src/feral_apl_reporter.rs` compiles the actual shared Program without Bevy/DB; four tests in `simc_apl/feral_reporter_tests.rs` use exact pinned source rows. Reporter36fc81e68 plus action-bound maximum-energy addendd101caf70 are pushed. Compile exit0, four behavioral failures/exit101/0.00s; `literal-reporter-red/{compile.proof.json,compile.output.log,proof.json,output.log}` preserves evidence. Filter `feral_reporter_`.

| Exact surface | Observed failure |
|---|---|
|`prowl,target_if=dot.rake.refreshable,if=!buff.shadowmeld.up`|Parser demands min/max mode|
|`variable,name=convokeCountRemaining,value=floor(...)`|Parser reports unexpected trailing token|
|`pool_resource,for_next=1` followed by exact max-Energy Bite row|Program wrongly dispatches the control row|
|`ferocious_bite,max_energy=1,if=combo_points>=5`|Program selects Bite at25Energy instead of waiting|

The concrete no-Incarnation/no-Apex reporter exposes `maximum_energy=50` only in the proposed Bite action context, from native25ordinary+25extra cap (SimC druid4253–4268); `energy.max=100` remains distinct. Numbers are fixture facts, not Program constants. Pool/max currently fail semantics, not nonexistent syntax errors.14766 owns the implementation; no private Program patch here.

## Native Feral area geometry

Atf90635634, actual3T profile plus mutual combat refs reaches `spell_targets.swipe_cat` and fails with unknown-native-operand: genuine1FAIL13.32s, clean1m00compile (`feral-geometry-red/`). Consumer57f46c965 reads native school-damage recipients through public `targets::select_targets` for Swipe106785 and Primal Wrath285381, intersects actual supplied fixture targets and uses authored target limits/radii. Moving one real target outside the area must reduce3to2; no cast or power change is involved. Unknown area aliases remain errors. Generic target/dot/resource facts still delegate unchanged. GREEN3PASS17.59s includes geometry plus retained Lunar/owned-dot controls; clean1m21simulationcompile, `feral-geometry-green/{proof.json,stdout.log}`. Literal controller integration remains open.

SimC `action.cpp3817–3900` uses the referenced action's target list when distance targeting is enabled, but falls back to active-enemy count when disabled. This native-geometry fact explicitly models reachable native recipients rather than asserting those two reference modes are identical. Fixture comparison must retain that distinction. At286c7ae74, bare `spell_targets` reproduces an unknown-operand RED1FAIL12.82s while named area counts still resolve; `feral-encounter-count-red/`. The bare operand now delegates actual shared `active_enemies`, preserving the pinned distance-disabled encounter count even when one fixture target is outside native area reach. No new count state or target substitution. Atfb4d4b8fb the actual near/far distinction passes1case14.08s, clean22.60s compiler run; `feral-encounter-count-green/{proof.json,stdout.log,stderr.log}`. This is not a literal300s proof.

## Native Lunar admission dependency

Exact reference traits select Lunar Inspiration155580/entry103170, not Chomp. At test-onlyfd16c1384 actual prepared actor has155580 but lacks its155627 override: genuine1FAIL15.50s after15m06s clean compiler run; `lunar-native-red/{proof.json,stdout.log,stderr.log}`. TC `spell_druid.cpp1543–1567` casts155627 on apply and removes it on talent removal. Portf30c7ca20 registers those two aura hooks, changing no KnownSpells or damage coefficients. Test retains removal/reapplication and ordinary8921→155625 resolution checks after the bootstrap assertion. GREEN2PASS13.58s includes the owned Rake/Moonfire binding control; both libraries compile6m10s/zero warnings (`lunar-native-green/{proof.json,stdout.log}` plus compiler output in the synchronous tool receipt). Native controller admission dependency is qualified; literal execution and parent integration remain pending.

## Selected-profile Rake fixture blocker

No production Rake edit. Three test-only attempts have not reached the damage-factor boundary:19414dc94 fails ordinary Rake with OnGlobalCooldown;977b42579 waits on actual native GCD but target becomes OutOfRange;f4d8aa5a8 waits/repositions the real target but native Prowl is gone after the wait. These are fixture failures, not three confirmations of the legacy-rank hypothesis. Receipts `rake-selected-red/`, `rake-selected-red-gcd-fixed/`, `rake-selected-red-range-fixed/` preserve each result.

Rake edits stopped; independent diagnosis requested from parent/shared harness owner. Questionable assumption: prepared follower remains quiet/stealthed while native GCD advances. `spawn_parity_attacker` parks living owner atx20; `LiteralParityFixture` suppresses kit selection, not all movement. Native5215 AuraInterruptFlags0=134233092 lacks movement0x8, so movement alone is not evidence for removal. Approved observer363816cd5 supplied the cause in one run: atnative1.55s actor cast1126 Mark of the Wild on its owner; requested/matched interrupt flags[4,0], `RemoveReason::Interrupted`, last successful cast1126, AutoAttack=null, CombatStatus=null. Frame receipts show1126CastSuccess→AuraApplied→5215AuraRemoved. `rake-prowl-removal-observer/{proof.json,stdout.log,stderr.log,removal-receipt.json}` preserves9.98s failure/clean9m24compile. Engine interrupt behavior is correct; parent approved fixture ownership/order correction: capture the existing literal controller, ordinarily apply required Mark buffs before Prowl and wait real GCD, with no flag/stat shim. Approved owned-precombat testda45833b1 reaches genuine damage RED: ordinary Rake7077, real Prowl Rake7077, authored1822effect3 factor1.6; both casts land, Prowl survives native GCD and neither legacy aura exists. OneFAIL14.86s, clean48.81scompile (`rake-selected-owned-precombat-red/`). The ensuing eligibility fix removes only the legacy rank requirement from the shared Rake actual-stealth predicate; initial/stun and existing bleed callback retain native amounts. Shred's separate predicate and persistent multiplier fields are untouched. Per explicit parent approval, the obsolete no-rank assertion is renamed `dummy7_rake_stealth_without_legacy_rank_gets_authored_bonus` and cites TC2040–2078; its old behavior/name is not preserved. Ata5b2fce9f all seven requested targeted cases pass:4initial/rank/nonstealth14.68s,2existing dummy12snapshot/removal/reapply10.51s and1actual selected-profile12.46s. Both packages compile36m09s/exit0/zero warnings; simulation's0filtered cases in the core invocation are not runtime coverage. CoreSHA0e8aa9580e6b8ba479c4bc81a986c12b29e08b4f85abff97dcc1c8425c6d9201, simulationSHAc761552aab35a2f0575d4673f4c49659914a7d3854347814d3fe412a8b085ba3. `rake-selected-green/` contains per-command exit/count/environment/hash receipts and full compiler stderr. Parent integration/full literal300s qualification remain open. Persistent pmultiplier fields remain14775-owned.

## Behavioral evidence

At code7ea788a8b/docs121a3b77e, existing `feral_literal_fixture_records_native_bleeds_for_300s` runs300s and fails the external driver contract: actual `NATIVE_FOLLOWER_KIT`, expected `LITERAL_SIMC_FERAL_76f8118`. Genuine1FAIL28.88s/exit101; no compile or fixture error. `literal-driver-red/{proof.json,output.log}` records binary6c20c1d68e8e8f7d99e03ead5bd4d10bb6b6777fc16fe87e52ad4128a4f20e54 and frozen privateDB environment. Its emitted filename `103.feral-literal.1.json` does not make the contents literal; the report correctly identifies the native-kit driver.

Priority comes from the qualified a7 KIT observation:3T Swipe79.03%/Rip1.60%, while reference Rip23.89%/Rake14.33%; ST Shred40.86% versus reference7.14%. These are rotation-distribution evidence, not a diagnosis of a generic damage scalar. [Source/mechanism census](feral-source-census.md) retains Unseen/Frenzy/Lunar/Rake gaps and authoritative references.
