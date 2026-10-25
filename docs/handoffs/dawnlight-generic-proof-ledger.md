# Dawnlight explicit generic policies — 2026-10-24

## Goal and authority

Branch `dawnlight-generic-7b` starts on exact master7b10a2c9b. Parent approved `SIMC_RET_DAWNLIGHT_WAKE_PRODUCER` and `SIMC_RET_DAWNLIGHT_SPENDER_CONSUMER` as explicit source-qualified policy-table entries, with generic execution and no core spell-ID conditionals. Preserve native modified count3, counter maximum6 and431380 periodic payload. Order is strict producer RED→GREEN, then strict consumer RED→GREEN. Before handoff run combined three-crate test-target check once, plain production bins check once and fmt. Frozen hostile-primaryd264 is untouched; its later landing still gates the independent post-primary ST/3T pair.

## Cause and source precedence

Native431377 and431522 effects are Dummy/zero child triggers. They have native proc flags/chance, but no child edge; full18-column `tdb_spell_proc` has no431377/431522/431380 row. Native431380 owns periodic-damage Aura3, SP coefficient0.637499988 and2000ms period. No native metadata is invented or replaced. Pinned TC a352b1fa `spell_paladin.cpp` has no handler for these Dawnlight IDs. Therefore the missing relationships are named SimC policies, not generic native trigger inference.

Pinned SimC76f8118 `sc_paladin_retribution.cpp846–876` executes Wake then grants Dawnlight uses. It reads raw431377 effect0=2; prior approved native qualification instead resolves actual Ret137027 effect25/Aura219/label6787 flat+1, giving3. Native3 wins; native counter cap6 remains unchanged. The already-qualified Ret development producer is not landed in7b, so this branch must first reproduce its missing-producer boundary.

Pinned `sc_paladin.hpp1512–1535` applies Dawnlight only on a successful foreground spender’s first target. On one target it can refresh; with multiple targets it first uses an unticking primary, otherwise the first unticking actual action target. It decrements one actual budget only when applying. `sc_paladin.cpp3002–3054` supplies the Dawnlight action context; native431380 remains payload authority. Secondary Avatar/AoE effects are not silently included.

## Proof matrix

|Boundary|Required behavior|Status|
|---|---|---|
|Producer|Actual selected Wake damages/earns3HP then arms native3, not max6|Strict RED pending; no production change|
|NoWake/noHero|Native Judgment or legal unspent Hero points cannot manufacture budget|Controls pending|
|Consumer|Actual Wake budget3 followed by funded foreground hit applies owned431380/tick and reduces3→2|Not reached; producer must be GREEN first|
|Background/target selection|Sourced exclusions and once-per-cast/native recipient behavior|Consumer-stage proof pending|

Input is a fresh canonical4442-derived3d2d private snapshot with only the approved Garrick66→70 surrogate; source hash/schema/profile/trait receipts remain in engine `ability-gap-post-primary-4442d87f/source-inputs.proof.json`. No power, budget, timer, coefficient or metadata grants substitute for the observed boundary.
