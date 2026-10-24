# Generic ordered native composite selection

## Goal / boundary

Adapter supplies `NativeComposite { name, steps: &[NativeAction] }`. Shared Prepared selection exposes `Execution::Composite(Vec<Execution>)` as an ordered pending plan, not a synchronous cast loop.14766 owns real precombat clock advancement, per-step revalidation/completion and final accounting. Main/14766 explicitly require empty/all-active plans to be unusable and world-only execute never to recurse into Composite.

Current `with_composites(world,actor,targets,metadata)` is a compile scaffold until native behavioral RED. World-only execute explicitly rejects Composite. No ready/completion claim yet. NativeAction is Copy/Clone metadata; composite cannot encode class policy in shared dispatcher.

Consumer staging: [native precombat ledger](native-precombat-proof-ledger.md#composite-lifecycle-integration-red--staged-unexecuted) records the separate clock/effect integration test. It remains ignored and unexecuted; no-op selection failure does not qualify consumption.

## Source / native fixture

13120 packet: `/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/assassination-parity-2026-10-23/apply-poison-source-packet-v1.json`; pinned `sc_rogue.cpp3164–3330`. Bare selected259 profile constructs Deadly2823,Crippling3408,DTB Amplifying381664,DTB Atrophic381637 in that order; no Numbing. Native casting-time16 base1500ms for each; shared code must not hardcode those IDs or erase actual cast time. Source profile/trait copy provenance: `arms-parity-2026-10-23/composite-selection-inputs.proof.json`. Fixture native actor is explicitly Human1/class4 (reference race metadata is not asserted to be Human).

## Targeted proof matrix (pending actual run)

| Case | Actual boundary | Expected proof |
|---|---|---|
| Bare apply_poison | Four actual native per-step admission plans; no cast/aura during selection | Ordered2823/3408/381664/381637 pending plan RED→GREEN |
| Existing Deadly | Normal native Player cast really completes/applies owned aura before selection | Pending3408/381664/381637, unchanged order |
| All active | Single-step actual owned Deadly plan | Unusable, not empty ready Composite |
| Empty metadata | Zero selected native steps | Unusable |
| World-only execute | Valid native coating steps supplied as Composite | Error before first cast/aura; no recursive dispatch |
| Non-poison | Actual Human1/class1 native6673 Battle Shout admission | Generic ordered pending plan, no poison special case |

Tests check real native knowledge/readiness premises before calling a selection failure behavioral RED; setup refusals are not RED. Actor is not engaged before precombat. Native durations/resources/knowledge are not patched or granted. No DPS, canonical DB, template or reference edits. API861b655a3 stays frozen for ta1068; selection work lives on separate `arms-literal-composites` branch.
