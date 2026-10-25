# Dawnlight explicit generic policies — 2026-10-24

## Goal and authority

Branch `dawnlight-generic-7b` starts on exact master7b10a2c9b. Parent approved `SIMC_RET_DAWNLIGHT_WAKE_PRODUCER` and `SIMC_RET_DAWNLIGHT_SPENDER_CONSUMER` as explicit source-qualified policy-table entries, with generic execution and no core spell-ID conditionals. Preserve native modified count3, counter maximum6 and431380 periodic payload. Order is strict producer RED→GREEN, then strict consumer RED→GREEN. Before handoff run combined three-crate test-target check once, plain production bins check once and fmt. Frozen hostile-primaryd264 is untouched; its later landing still gates the independent post-primary ST/3T pair.

## Cause and source precedence

Native431377 and431522 effects are Dummy/zero child triggers. They have native proc flags/chance, but no child edge; full18-column `tdb_spell_proc` has no431377/431522/431380 row. Native431380 owns periodic-damage Aura3, SP coefficient0.637499988 and2000ms period. No native metadata is invented or replaced. Pinned TC a352b1fa `spell_paladin.cpp` has no handler for these Dawnlight IDs. Therefore the missing relationships are named SimC policies, not generic native trigger inference.

Pinned SimC76f8118 `sc_paladin_retribution.cpp846–876` executes Wake then grants Dawnlight uses. It reads raw431377 effect0=2; prior approved native qualification instead resolves actual Ret137027 effect25/Aura219/label6787 flat+1, giving3. Native3 wins; native counter cap6 remains unchanged. The already-qualified Ret development producer is not landed in7b, so this branch must first reproduce its missing-producer boundary.

Pinned `sc_paladin.hpp1512–1535` applies Dawnlight only on a successful foreground spender’s first target. On one target it can refresh; with multiple targets it first uses an unticking primary, otherwise the first unticking actual action target. It decrements one actual budget only when applying. `sc_paladin.cpp3002–3054` supplies the Dawnlight action context; native431380 remains payload authority. Secondary Avatar/AoE effects are not silently included.

## Current boundaries

Both named relationships are rows of the same `POLICIES` table (`Policy` plus `Relationship` variants); all specific IDs/spec/talent bindings stay in those rows. Generic core code captures ordinary launch inputs and invokes generic logical-hit dispatch. Native payload admission and `spend_stack` keep target rules, stack cap and remaining duration; no fallback primary is selected.

The qualified receiver is an actual native NPC actor. The literal self-target/hostile-primary caller integration remains the separately frozen hp1109 dependency; this component’s AoE tests provide the real native hostile primary explicitly, not a local fact substitute. Player-policy, natural Purpose-producer rate, Sun’s Avatar/secondary Dawnlight damage and full DPS parity are not claimed by these scoped tests. The post-primary300s pair remains deferred until hp1109 lands.

## Proof matrix

|Boundary|Required behavior|Status|
|---|---|---|
|Producer|Actual selected Wake damages/earns3HP then arms native3, not max6|29a75ec42 genuine missing431522 RED67.06s after raw/live count, real Wake damage and3HP prerequisites pass;2negative controls PASS|
|NoWake/noHero|Native Judgment or legal unspent Hero points cannot manufacture budget|NoWake/noHero2PASS in the strict producer RED run; unchanged producer suite3GREEN40.94s at60f30134b|
|Consumer|Actual Wake budget3 followed by funded foreground hit applies owned431380/tick and reduces3→2|de01f4c4b genuine missing-owned431380 RED after budget3, real paid3HP and actual FV damage; native payload/noWake2PASS|
|Background/target selection|Sourced exclusions and once-per-cast/native recipient behavior|Native background hit, STrefresh,3Tsecondary/once, all-ticking no-spend, zero-cost pool and actual immunity PASS; initial Dodge receipt retained|

Producer implementation ports the previously approved Ret behavior into `scripts/policies.rs`: explicit named data owns the selected spec/talent/parent/counter/effect relationship; generic lookup/selection/native-count execution contains no hardcoded spell-ID branch. Native triggered aura application retains its maximum6. At60f30134b the unchanged producer suite is **3/3GREEN40.94s**, warning-free1m06s compile (`producer-green-v1.*`): native3/max6 and both negatives pass. Consumer code remains absent; its tests are added only after this producer GREEN.

Consumer `consumer-red-v1.*` reaches the strict positive boundary in50.02s suite runtime; it also records one background-control setup failure, not a consumer RED. `background-receipt-v1.*` proves that attempt was native Miss(Dodge), not zero payload or missing admission. The corrected control retains all budget/power/no-DoT assertions after each real background attempt and requires a positive hit within8attempts. Native payload control proves real431380 periodic damage without a numeric shim. At1bad913ec the corrected background control PASS34.25s with an actual native hit, still before consumer implementation. Consumer code follows the strict de01 RED through the same policy table: ordinary launch captures actual applicable resource pools (including zero final cost) and original selected primary/list; canonical logical hit dispatch only considers that first primary. Full-mask catalog clones carry no context. Native payload target relation/life checks filter recipients; missing primary is refused, never substituted. Single-target refresh, multi-target first-unticking selection and no-use-when-all-ticking use live caster-owned periodic state. Accepted native child launch spends one native counter stack without timer refresh. No payload, amount, period or maximum is edited. Atb930a4b5e the unchanged strict positive, corrected background hit, native payload/noWake and producer controls are **7/7GREEN56.75s**, warning-free2m07s compile (`consumer-green-v1.*`). Additional target/refresh/zero-cost/immune cases now isolate the source branches with actual native pre-existing payload auras and the already-qualified native Purpose modifier, never injected Dawnlight budget or power. Atdc207983d those5native controls PASS48.06s, zero warnings. Following the final helper-only production refactor28bf70027, all12Dawnlight cases plus3unchanged native tier controls pass together: **15/15GREEN69.75s**, warning-free49.53s compile (`native-integration-v1.*`). No later runtime change invalidates this receipt.

Producer RED receipt: engine `dawnlight-generic-7b/producer-red-v1.{log,proof.json}`; zero warnings, compile1m14s. The failure is the actual missing budget, not a raw2/live3 oracle or admission error.

Input is a fresh canonical4442-derived3d2d private snapshot with only the approved Garrick66→70 surrogate; source hash/schema/profile/trait receipts remain in engine `ability-gap-post-primary-4442d87f/source-inputs.proof.json`. No power, budget, timer, coefficient or metadata grants substitute for the observed boundary.
