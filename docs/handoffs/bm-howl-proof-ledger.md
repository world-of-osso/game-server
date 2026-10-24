# BM Howl — authentic closure before lifecycle

## Governing policy

Parent2026-10-24 approves `SIMC_BM_HOWL_OF_THE_PACK_LEADER_CYCLE`:471876 links to Wyvern1222271, Boar472020 and Bear471993 using pinned SimC76f8118 hunter3776–3840/7830–7834/8240–8290. Native DB2 duration, amount and cap win; Cry must be earned from an actual native Wyvern summon, not granted as SimC's buff-only shortcut. Ordinary KC consumes naturally earned readiness. No fabricated stats. Parent publishes importer delta only after code lands. [Contract](../specs/spell-dummy-scripts.md#bm-howl-summon-cycle).

TC a352b1fa `spell_hunter.cpp` has no471876/471877/471878/471881/472324/472325 binding; the entire script tree has no named Howl policy. Native471876 has Dummy aura plus values1/10 but no EffectTriggerSpell links. SimC supplies order, combat-entry timer and consume/rearm policy, not native numeric replacements. Its combat_begin8699 starts the cooldown; consume3776–3822 uses ready states and expires them; callbacks8240–8270 rearm/cycle. The native actor requirement intentionally goes beyond SimC's buff-only Wyvern model.

## Native/TDB closure

Read-only current backupcd723 contains authentic TDB1210 entries234018 `Bear` and234170 `Wyvern`; neither exists in runtime `content_creature_template`. Native1222271 effect28 targets234170/props6178. Bear471993 aura428 triggers471990, whose effect28 targets234018/props6125. Both SummonProperties exist. Boar472020 is a dummy action; no creature ID is inferred from its name. Exact rows/masks/descriptions are retained in diagnostics `bm-howl-native-source-census.json`.

Native471877 duration30000ms; Wyvern ready471878, Boar472324 and Bear472325 also have30000ms. Native Cry471881 duration12000ms, base1% and stackcap20;471876 effect2 base10 selects initial Cry stacks. These are source facts, not tested lifecycle outcomes.

## Importer RED/GREEN and delta

`CLASS_SUMMON_ENTRIES` previously only followed native EffectTriggerSpell roots. Approved named policy edges now join that recursive closure; no spell row, coefficient or creature row is manufactured. Normal `add_class_summon_templates.py` owns difficulty/content-tuning transformation, models and addons.

| Revision | Scope | Result |
|---|---|---|
|3842f60a9|Real native root reachability and normal copy importer|2FAIL6.273s: both actor IDs unreachable and output actor rows empty. Importer RED, not gameplay RED.|
|ba3bc5df8|Same importer tests after sourced links|2GREEN3.917s: authentic names/model rows copied, world spawns unchanged.|

Diagnostics root `game-engine/data/diagnostics/dps-parity-fix-2026-10-21/`:
- Source private active-kit worldf10a75e6; normal fixture preparation adds exactly234018/234170 into new `howl-inputs/world.howl-current-private.db`, SHA256 `0297adfd466e85ce812f7821aa3788823adcf06867a9bd6999207bad3a75d83a`.
- Parent publication delta `howl-inputs/howl-summon-import.sql`, SHA256 `736cc3435a4ec42f9efb45b232322417be970380ec095dbee95e1007e475c2ee`; complete source/command/output receipt `import-receipt.json`.
- Replay on another fresh private copy matches all three complete content template/model/addon tables in both EXCEPT directions; integrity_check ok and world spawn count unchanged. Native TDB baselevels1 and modifiers remain intact; no guessed level90 stats.
- Canonical runtime and every previous frozen baseline remain unmodified. Parent owns publication; no runtime lifecycle change has been made yet.

## Native qualification — not yet gameplay RED

`howl_tests.rs` observes actual `Npc` plus public `SummonedCreature` owner/spell metadata and native expiry, not an asserted private class selection. No timers/resources/stats/readiness assigned.

| Revision | Actual boundary | Qualification |
|---|---|---|
|bb548b511|Four initial probes|Direct child produces0entities; Player ordinary883 returns Pet(NoPetAvailable). These are unresolved prerequisites, not accepted lifecycle RED.|
|111139e7a|Public FULL_MASK child admission result|Actual1222271 refuses `Unsupported("target48 requires collision data")`. Parity App lacks native LOS/ground; no range/target bypass added.|
|9c94ae19e|Attempt to load real LOS/ground from simulation crate|CompileE0603: core modules private. Not gameplay RED; runner refuses stale artifact execution.|

Parent approved the narrow fixture boundary.519339c08 adds `test_data::install_native_collision(&mut App)->Result<(),String>` gated `test`/`test-support`: actual source GroundMap/LOS from explicit GAME_SERVER_GROUND_DIR, no exposed private types. Root/KC now uses actual Austin's existing normal883 pet, per-frame naturally earned readiness and observed ordinary KC/summon; no invented Player stable. Four native qualification cases building/running; no gameplay RED inferred yet. No synthetic terrain, teleport, fake ready aura or direct Cry grant.

## Actual-summon→Cry slice

Parent explicitly authorizes qualifying this slice separately while root/KC stays open.519339c08 native4 yields2PASS/2FAIL22.68s after clean15m13 compilation: actual1222271 creates authentic entity and expires at native duration; removed passive still permits actual summon without Cry. Selected actual summon produces0Cry stacks versus native471876 effect2=10: genuine gameplay RED. Root case executes0KC under actual LOS and is a separate unresolved fixture/admission boundary, not accepted readiness RED.

The narrow production hook runs after real native wild summon metadata/source registration, not at cast launch or in a log formatter. Only actual1222271 metadata owned by the selected471876 caster earns471881 via existing triggered stack override; DB2 owns resulting amount/duration/cap. No direct grant replaces a missing entity. No root timer, KC consume, cycle state, Pack Mentality or creature AI is implemented in this slice. c7c33ace7 passes all three relevant native child/lifetime/absence cases in17.84s; clean12m52s compilation. The root case was explicitly skipped under parent authorization and remains open, not silently waived. Actual selected summon now earns native10Cry stacks; actual entity/native expiry and removed-passive summon/noCry controls pass. This is not combat-ready/KC/cycle or whole-BM qualification.

## Root admission diagnosis on separate branch

Parent froze `dps-parity-bm-owner` atb9c71fba2 for rg1074. Root follow-ups use separate `dps-parity-bm-howl-root`; no owner-ref changes.

ad61e63c0 records one native diagnostic PASS15.14s: KC34026 and Cobra193455 both return LineOfSight before engagement and at5s/35s (brief GCD at first1s masks target checks). Austin stays(16.000002,0,0), the old dummy stays(1.5,0,0), Focus100 and its real pet remain present, KC0. Runtime model IDs113756/27510 match actual standing eye rows in the real bake. Missing pet/resource/standing-eye prerequisites are ruled out; native LOS rejects that fixture input. This is not missing-readiness gameplay RED. Compile20m13s exposed the stale unused `navigation` import from the master test split;151af01ab removes it on this separate branch, without warning suppression. The rg1074 server-lib proof had no such warning.

781761df8 initial-spawn falsifier PASS: a new native31146 dummy born at(16.000002,0,1.5) through ordinary `zones::summon_creature` admits KC/Cobra beside unchanged Austin; the old unengaged target is retired. No existing Position is written; geometry/normal spell checks stay enabled. The root case now executes ordinary KC>0, then fails because actual combat entry plus native cooldown never earns the first ready aura: genuine gameplay RED. Native1PASS/1FAIL12.27s, clean compile57.17s; exact `howl-root-spawn-red-781761df8-{compile.log,executable.json,native.log}` receipts live under the same diagnostics root. No production readiness/cycle patch yet.

Next pre-implementation controls require the real native-clock/KC sequence Wyvern1222271 summon → Boar471936 native action → Bear471990 summon, and no cycle after removing the root passive. Boar is literally a direct damage action, not an invented creature. SimC4797–4848 supplies that action linkage only; its estimated travel speed50, bug-toggle target cap and ranged-attack wrapper must not replace native data/payload handling. The independent source read confused the missing Howl aura cooldown with KC spell history; rejected that diagnosis because ordinary KC already executes. No generic spell-history change follows from this RED.

## Open

- [x] Actual native child entity/lifetime and selected/removed-passive Cry slice qualified separately.
- [x] Native valid root admission at fixture target birth and genuine combat-readiness RED781761df8 before production implementation.
- [ ] Native GREEN for cycle/rearm/absence/ownership boundaries and parent broad integration.
- [ ] Bounded same-input cadence/DPS measurement only after qualification; whole-BM parity remains open.
