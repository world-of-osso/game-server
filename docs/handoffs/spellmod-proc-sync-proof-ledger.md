# TC-synchronous procs: Stage2 proof ledger

## Scope and base

Main approved [Stage1](spellmod-proc-scheduling-decision.md#4-stage1-design-on-pe1098) with the [Stage2 rulings](spellmod-proc-scheduling-decision.md#5-stage2-rulings). Branch `spellmod-proc-sync` starts at exact origin/master13d2f96338e57e3141f01b7d611e2822e905182b in the existing isolated spellmod worktree. Old applied-set work remains on its separate branch, not imported wholesale. pe1098 is unlanded; no merge of it is authorized until main lands reconciliation15650. Adds checkout stays frozenb5d6a4a54.

## Commit1: full shared definitions, not dispatch

Reuse the qualified importer4fb117492 and effective-definition implementation1c98c106a, not another parser. The existing authoritative `tdb_spell_proc` now stores every audited18-field value atomically. The native/serverside loader binds explicit-before-generated effective entries and preserves original DB2 options. Shared existing Dance/Inundate masks are factored without changing their runtime meaning. No runtime proc matcher, aura charge initialization, cast masks or generic spellmod drain is switched in this commit. Presence-only or missing full-row schemas fail; canonical data publication remains main-owned.

Fresh RED scope on13d2f9633 plus recorded test/scaffolding hashes:

| Proof | Result | Receipt |
| --- | --- | --- |
| Real CLI complete BodyAndSoul/Expurgation persistence, idempotence and missing-phase rejection |3assertionFAIL/0setup errors0.339s; only IDs persisted and missing phase accepted | `proc-sync-c1-importer-red-v1.proof.json` |
| Public native loader16cases with typed None scaffolding |16actual missing-entry/provenance assertionFAIL5.41s; compile4m33s/zero warnings/exit101 | `proc-sync-c1-definitions-red-v1.proof.json` |
| Importer GREEN atb70076ea3 |8/8PASS0.900s/exit0, full persisted values and all hash/idempotence/rollback controls | `proc-sync-c1-importer-green-v1.proof.json` |
| Native definition GREEN atb70076ea3 |Unchanged16/16PASS10.72s, compile1m20s/zero warnings/exit0 | `proc-sync-c1-definitions-green-v1.proof.json` |

Artifacts are under engine `data/diagnostics/arms-parity-2026-10-23/`. Native input resolves to private `tdb-proc-values/world.tdb-values-import.db`, freshly rehashed **60132e07ddc5ded0fc73b3de2b26ab866b0e87b9d976af7688792146e4c3be62**,887570432bytes. No canonical DB writes. Native RED source hashes, command, exact base, binary and logs are in its receipt; setup/compilation/zero tests never count as RED. Scope is metadata only: historical BodyAndSoul/Expurgation dispatch failures are not claimed fixed.

## Commit2: captured inputs / applied identities

Current capture implementation is not yet qualified GREEN. Fresh original7 native cases atfdc9f31a4 plus recorded None-scaffolding hashes produced **7actual missing-context assertionRED21.56s**, zero-warning2m12compile/exit101 (`proc-sync-c2-capture-red-v1.*`). Ordinary Shield/Fortitude, actual used/unused operations, real instant crit, actual delayed285452 impact and actual BodyAndSoul65081 gain all reached the intended missing-context boundary; setup failures were not counted.

C2 carries an actual shared cast receipt through scopes and delayed clones; new/nested casts restore the outer receipt instead of recording into it. Frozen modifier candidates retain their actual source IDs, but only evaluation registers nonzero used operations. The legacy consume_charges body remains unchanged. Raw operation outcomes flow separately from damage/heal sums; nullification flags are captured at block/absorb boundaries. Scoped chain counters label children without activating the hard limit or changing the current queued dispatch. Prepared costs and actual item API contexts stay distinct from unavailable triggered/tick receipts. Ret Sentence's existing inline record_damage hook in this13d base is preserved before observation/queueing.

| Captured surface | Proof / current limitation |
| --- | --- |
| Ordinary CAST, direct damage/heal and confirmed landed aura-only HIT | Native capture proof pending; actualcrit and rawpre-overheal data, not log-derived metadata. Other unresolved no-damage/immunity outcomes remain unavailable. |
| Generic/Bool/costed/destination triggered and delayed ownership | Own receipt and invocation chain retained; native parent/child and chain cases pending. |
| Periodic/melee/KILL-DEATH/knockback/reflect | Known source fields forwarded, no new producers. Null-Spell schoolNONE is explicit TC Unit.cpp292–300; reflect is TYPE5/PHASE0/HIT_REFLECT, knockback TYPE7/PHASE2/HIT_NONE. No blanket NORMAL is substituted. |
| Emission-time observer | Opt-in existing test-support observation only, not a compatibility dispatch queue; identity values frozen at emission. |
| Scheduling/charge/masks | Unchanged legacy transport, low CAST flags, charging, target batching and effect timing. Actual atomic cutover remains4; FINISH absent. |

Fixture qualification before GREEN:28815 is still **not** a TC-positive charge oracle; its source is explicitly cancelled before the second cast, so the provenance test does not require legacy wrong consumption. The delayed child case now gains native378269 after actual launch/before impact. Giving it before launch while retaining C2's legacy drain would delete the source before the child evaluates; that is C4's separate charge-retention boundary, not permission to fabricate an application or change C2 math. The existing provenance assertions are unchanged; these setup differences mean the updated cases are not presented as byte-identical to the original7 RED bodies. An eighth actual mid-hardcast77762 capture case tests whether finish-time readiness/facing queries incorrectly register an unused admission modifier; its proof is still pending.

First committed integration attempt40b37a94c was **compile-only E0609** at five old `HitFrame.damage` consumer paths (batch6/8/10/14/17); no tests ran and it is not behavioral RED/GREEN. Paths are updated to the same retained damage vector under `results`, without changing their first-target/order logic. Receipt: `proc-sync-c2-capture-integrated-v1.*`.

C2 input is a fresh read-only canonical `.backup`: private `world.proc-sync-c2-canonical-backup.db` SHA**7bad5477f2c6da08a24ea95645621ecdf0f0dfc42008393fef922aedaac0bc7f**,1291×18,887570432bytes. `proc-sync-c2-input.proof.json` links it to main's canonical4442d87f publish. Only this worktree's alias changed; no canonical writes. C1 proof remains scoped to its immutable60132e07 input/revision and was not rerun.

## Charge-policy and data boundaries

191634 has native modifier108 effects and DB2 flags/charge metadata, but no qualifying generated entry, audited TDB row or pinned TC script. Main explicitly assigned **SIMC_SHAMAN_STORMKEEPER_CONSUMPTION** instead: an ordinary post-CAST owner, source SimC76f8118 `sc_shaman.cpp6443–6449,7427–7433` (NORMAL only, protect midcast gain). This is not a TDB row, a generic entry or a fallback. Implementation belongs to atomic4; generic charge draining will be deleted.

53390 qualification is held: effective charge1/stack-cap2/CAST-HEAL/attribute0 does not directly implement the existing literal stacks2→1→0 contract. Pinned charge preparation/removal would remove the aura at charge0, not decrement stacks. An independent read found51564→`spell_sha_tidal_waves`; full current script-binding provenance and actual heal/driver filtering still need qualification. No source-qualified fixture-only repair identified; no assertion, entry or data changes made. This ambiguity was reported to main; activation is held, while independent commits1/2 continue. An unrun backend failure is not claimed.

## Remaining gates

- [x] Commit1 b70076ea3 focused importer/definition GREEN; tip and exact lead-only publication plan sent. Canonical publication and full gate remain main-owned. `proc-sync-c1-publication-plan.json` records verified eight sources/audit SHA, absolute C1 importer command and expected1291×18 schema; no canonical writes by worker.
- [ ] Commit2 real context/shared receipts with behavioral proof; no inferred modifiers/hit data.
- [ ] Landed pe1098 integration before dependent commit3; qualify53390 before4.
- [ ] Commit3 touched native DPS specs once with prior-value deltas, no retuning.
- [ ] Atomic4 entry eligibility/preparation/synchronous effects/finalization/guards and remove duplicate drains; kept consumers, approved eligible fixtures and scoped two-crate check once, then main full gate. FINISH/absent producers remain gaps.
