# Generic ordered native composite selection

## Goal / boundary

Adapter supplies `NativeComposite { name, steps: &[NativeAction] }`. Shared Prepared selection exposes `Execution::Composite(Vec<Execution>)` as an ordered pending plan, not a synchronous cast loop.14766 owns real precombat clock advancement, per-step revalidation/completion and final accounting. Main/14766 explicitly require empty/all-active plans to be unusable and world-only execute never to recurse into Composite.

At4ad3b8d41 the native six-case cohort produces3controlsPASS/3genuine selectionFAIL26.39s after18m29compile (one scaffold unused-field warning). All four ordinary native admission plans, normal Deadly cast completion and Warrior6673 readiness premises pass; no setup failure is counted. `composite-red.{log,proof.json}` records exact revision/current-backed input. Implemented `with_composites(world,actor,targets,metadata)` now plans ordered pending work; GREEN pending. World-only execute rejects Composite before any step. No completion-scheduler claim. NativeAction is Copy/Clone metadata; composite cannot encode class policy in shared dispatcher.

Consumer staging: [native precombat ledger](native-precombat-proof-ledger.md#composite-lifecycle-integration-red--staged-unexecuted) records the separate clock/effect integration test. It remains ignored and unexecuted; no-op selection failure does not qualify consumption.

## Integrated runtime atd49f4951c

Actual selection packet runs7 cases in24.62s:6PASS/1FAIL. Bare four-step order/no dispatch, all-active, empty, unsupported selected step, non-poison and world-only rejection controls pass. Existing-coating case fails its own native Amplifying admission premise, not the selection assertion. Its helper returned when the aura appeared without requiring GCD completion. Integration now uses the shared real cast/GCD/effect drain before asserting owned aura/return; selection logic is unchanged, GREEN pending. Adjacent manual four-coating full-drain control passes28.20s; it does not alone prove this failure's exact refusal cause.

## Source / native fixture

13120 packet: `/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/assassination-parity-2026-10-23/apply-poison-source-packet-v1.json`; pinned `sc_rogue.cpp3164–3330`. Bare selected259 profile constructs Deadly2823,Crippling3408,DTB Amplifying381664,DTB Atrophic381637 in that order; no Numbing. Native casting-time16 base1500ms for each; shared code must not hardcode those IDs or erase actual cast time. Source profile/trait copy provenance: `arms-parity-2026-10-23/composite-selection-inputs.proof.json`. Fixture native actor is explicitly Human1/class4 (reference race metadata is not asserted to be Human).

## Targeted proof matrix

| Case | Actual boundary | Expected proof |
|---|---|---|
| Bare apply_poison | Four actual native per-step admission plans; no cast/aura during selection | Genuine no-composite RED; implementation GREEN pending |
| Existing Deadly | Normal native Player cast really completes/applies owned aura before selection | Real normal-cast Deadly premise passes; genuine no-composite RED; GREEN pending |
| All active | Single-step actual owned Deadly plan | Native all-active controlPASS; remains required GREEN |
| Empty metadata | Zero selected native steps | Empty controlPASS; remains required GREEN |
| World-only execute | Valid native coating steps supplied as Composite | World-only rejection controlPASS; no first cast/aura; remains required GREEN |
| Non-poison | Actual Human1/class1 native6673 Battle Shout admission | Native6673 admission premise passes; genuine no-composite RED; GREEN pending |

A seventh control removes one actual selected spell from the real book (no grants), verifies the other three native plans remain admitted, and requires the whole selected composite to remain unavailable instead of silently dropping that step. No unrelated source changed; Moving/GCD snapshot fields and Driver gating belong to14766.

Tests check real native knowledge/readiness premises before calling a selection failure behavioral RED; setup refusals are not RED. Actor is not engaged before precombat. Native durations/resources/knowledge are not patched or granted. No DPS, canonical DB, template or reference edits. API861b655a3 stays frozen for ta1068; selection work lives on separate `arms-literal-composites` branch.
