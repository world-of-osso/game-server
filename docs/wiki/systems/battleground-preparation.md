# Ordinary battleground preparation

The ordinary-match readiness check previously called `BgInstance::start` as soon as enough accepted players arrived. It skipped the preparation clock completely. Arena preparation remains separate.

## Clock and gates

`InvitedBattleground.preparation` holds the first-arrival epoch, current stage, actual third-stage time and actual engagement time. `pvp_objective::update_match` advances it using fixed server time and participants in the exact copy, then applies existing roster readiness. No client countdown field is added.

`pvp_bg_preparation.rs` names sourced `(map, entry, action)` tuples. The ordinary GameObject loader includes those authored placements despite difficulty0. Map creation projects the closed animation. Normal fixed updates apply the corresponding source open/delete deadline, authoritative GameObjectState/collision and animation; delayed removals continue after engagement. All object queries match the exact `InMap`, not just the map ID. Source behavior and missing placements are specified in the [PvP contract](../../specs/pvp-progression.md#ordinary-battleground-preparation).

## Proof boundary

Native channel regressions cover concrete/Random roster arrival and objective refusal through the preparation interval. Source-backed gate fixtures exercise map-copy collision/animation/lifetime. The [proof ledger](../../handoffs/pvp-bg-preparation-proof-ledger.md) separates partial retained cohorts, prerequisite failures and final focused passes; Lead final integration remains pending. CPU-only evidence does not establish UDP delivery, rendered collision or deployment. Resource/reset policy remains separate. Preparation44521 now has a bounded role lifecycle: successful ordinary Waiting ACK uses normal AuraApplication; the natural start clears accepted/current-copy actors after the PvP state borrow is released; successful ordinary exit uses the existing BG cleanup boundary. It never invokes arena resets. Native cost/affordability and lifecycle receipts are tracked in the [aura ledger](../../handoffs/pvp-bg-preparation-aura-proof-ledger.md). `AURA_INTERRUPT_LEAVE_ENTER_WORLD_SEMANTICS_UNQUALIFIED` excludes generic map-entry/removal, persistence and logout aura purges.

## Sources

- [Contract](../../specs/pvp-progression.md#ordinary-battleground-preparation)
- TC a352b1fa `Battleground.cpp`100–103/334–445 and each map's OnStart; cached source/hash/placement inventory under `data/diagnostics/pvp-bg-preparation-2026-10-19/gate-inventory.json` in the canonical engine data tree.
- [Implementation](../../../crates/server/src/pvp_bg_preparation.rs)

## See Also

- [GameObjects](game-objects.md) — authored copied-map content and collision.
