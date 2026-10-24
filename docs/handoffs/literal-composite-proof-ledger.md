# Generic ordered native composite selection

## Goal / boundary

Adapter supplies `NativeComposite { name, steps: &[NativeAction] }`. `Prepared::with_composites(world,actor,targets,metadata)` exposes `Execution::Composite(Vec<Execution>)` as ordered pending work, never a synchronous cast loop.14766 owns real precombat clock advancement, each normal native cast/revalidation/completion, Moving/GCD snapshots and Driver gating. Empty/all-active plans are unusable; world-only execute rejects Composite before its first step.

Shared code has no poison IDs/policy. Self steps inspect only actual owned unexpired self auras; active steps are skipped without reordering. Every remaining selected step must be native-permitted/known/admitted or the whole candidate plan is unavailable. Existing Prepared fields survive the builder. Harmful/mixed steps retain native candidate target identities; no first-target readiness borrowing or partial plan.

## Source / fixture provenance

13120 packet: `/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/assassination-parity-2026-10-23/apply-poison-source-packet-v1.json`; pinned `sc_rogue.cpp3164–3330`. Bare selected259 profile constructs Deadly2823,Crippling3408,DTB Amplifying381664,DTB Atrophic381637 in that order; no Numbing. Native casting-time16 base1500ms for each. Source profile/trait copy: `arms-parity-2026-10-23/composite-selection-inputs.proof.json`. Native actors explicitly Human1/class4 and Human1/class1; reference race metadata is not asserted to be Human. They are not engaged before precombat.

## Behavioral proof

Diagnostics root: `/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/arms-parity-2026-10-23/`.

- 4ad3b8d41: native6case cohort3controlsPASS/3genuine selectionFAIL26.39s,18m29compile,1scaffold unused-field warning. Four normal admissions, actual native Deadly cast completion and6673 admission premises all pass; setup failures are not counted. `composite-red.{log,proof.json}`.
- 7cfc2e4e6: native7GREEN26.72s,16m03compile/0warnings,0ignored within filtered opt-in run. Command: helper `--build-host native --test -j 2 -p server --lib literal_composite_ -- --ignored --nocapture --test-threads=1`. `composite-green.{log,proof.json}`; exact library executable SHA256 `231876d16f98da6d7fc063f3c13d62a9d0b204c4651e0aab3296fd335a8920eb` in `composite-green-binary.proof.json`. Same recorded read-onlydd1de231e32c native input; no DPS run.

## Capability / proof matrix

| Case | Actual native boundary | Result |
|---|---|---|
| Bare apply_poison | Four actual per-step admissions, no cast/aura during selection | Ordered2823/3408/381664/381637 pending plan GREEN |
| Existing Deadly | Normal Player cast completes and applies owned aura before selection | Pending3408/381664/381637, unchanged order GREEN |
| All active | Single-step actual owned Deadly plan | Unusable, not an empty ready Composite GREEN |
| Empty metadata | Zero selected native steps | Unusable GREEN |
| One selected step unavailable | Remove only selected381637 from actual book; other three native plans remain ready | Whole composite unavailable; no quiet three-of-four plan GREEN |
| World-only execute | Valid coating steps supplied as Composite | Error before first cast/aura, no recursive/partial dispatch GREEN |
| Non-poison | Actual Human/class1 native6673 admission | Generic ordered pending plan, no poison special case GREEN |

Tests remain explicitly opt-in because they need private259/71profile files; final proof/docs changes do not enable them into unconfigured broad CI. Native durations/resources are not granted or patched; one negative control removes actual knowledge without granting spells. No DPS, canonical DB, template or reference edits. API861b655a3 stays frozen for ta1068; selection branch is `arms-literal-composites`.

## Integration ownership / remaining work

14766 consumes Composite one real cast at a time and owns scheduler completion/accounting; this selection proof is not four-cast serial execution proof. No Moving/GCD fields or Driver gating were changed here. Lead owns integration gate. FIRST/cycle, applied persistent multipliers and Arms numeric/DPS qualification remain separate open work.
