# Shared native target context API

Main requested this API for14781's literal Dev adapter. Lifecycle217f38d8f stays frozen in bt1056; tests-only acceptance e3ad46ee6 has6follow-upGREEN/15unique cases with prior14. Branch `arms-native-target-api` includes master8e7c7a685 via5d9f60fc5; merge retained both wiki index entries.14781's72f993fdf visibility seam was cherry-picked as61b9fc3dd; it changes no native cast behavior.

## Behavioral RED

At4be688315: `--lib native_target_context_ -- --ignored --nocapture --test-threads=1` reaches8genuine assertion failures in15.86s, exit101. Runtime premises pass, including actual protocol rank1 Fire Breath aura20s. Current target80% instead of explicit25%, stale/None context borrows selection, target candidates/active_dot/movement/casting/refreshable are unavailable. Two scaffold dead-field warnings are removed by implementing field consumers, never suppressed. Diagnostics: `arms-parity-2026-10-23/target-context-lib-red.{log,proof.json}`, same read-onlydd1de231e32c snapshot as earlier native proofs.

At68e24f49c the historical `--bin game-server` command compiled11m44 then executed0tests after the merged catalog/main split. NOT behavioral RED. Count guard rejected it; `target-context-red.*` retained. Current library runner requires all8cases; no stale executable fallback.

## Capability / proof matrix

| Capability | Native representation | Proof |
|---|---|---|
| Current versus proposed row target | `with_targets(current: Option<Entity>, ordered: &[Entity])`; explicit Action.target wins, invalid/None context cannot redirect | Genuine RED; implementation GREEN pending |
| Ordered per-action candidates | Actual universe order plus ordinary native target validator/range/LOS/life/map checks, not power/cooldown readiness | Genuine RED; implementation GREEN pending |
| active_dot alias | Bound actual aura ID; owned unexpired instances across supplied living targets, excludes foreign caster | Genuine RED; implementation GREEN pending |
| Movement event absent in dummy fixture | Pinned no-event values; not inferred from unit movement; dungeon timing unavailable | Genuine RED; implementation GREEN pending |
| casting.react with no native cast | Actual selected target's CastState absence means known false | Genuine RED; implementation GREEN pending; active cast reaction remains NATIVE_CASTING_REACTION_UNAVAILABLE, no invented delay |
| Refreshable | Explicit native metadata resolver plus Pandemic/final-tick/always/never policy; ordinary native duration modifiers; actual unpaid-tick helper | Rend + real20s Fire Breath reach genuine unsupported-property RED; consumer GREEN pending |

## Consumer API

Construct ordinary facts, then install actual controller context:

```rust
NativeFacts::new(world, actor, bindings, encounter, targets.len())
    .with_targets(Some(current), targets)
```

An explicitly configured `None` or expired current target is not replaced by the actor's CombatTarget. Without explicit context, existing consumers still read actual native selection; candidate/active_dot queries require a real universe. `ActionFacts::target_ids` preserves order and uses target-only admission, leaving ordinary cast readiness in Prepared. Unknown/unsupported native eligibility errors propagate.

Aura aliases remain in `Bindings.buffs`. `Bindings.dot_refresh[alias] = DotRefresh { resolve, policy }` supplies `ResolveDotSpell = fn(&World, Entity, u32) -> Result<SpellInfo,String>` and `DotRefreshPolicy::{Pandemic,FinalTick,Always,Never}`. Resolver identity must match the alias. `native_dot_spell` supplies ordinary native metadata; empowered adapters resolve an actually eligible stage with the native release helper. Shared `auras::fresh_duration_ms` applies ordinary caster/target modifiers to that resolved SpellInfo. Raw357209's2s row is not Fire Breath rank1's20s refresh duration. FinalTick reuses `auras::remaining_periodic_ticks`; no reconstructed scheduler phase or snapshot-by-division.

Sources: pinned `action.cpp2435–2508,4630–4645,5362–5453`, `dot.cpp458–477`, `raid_event.cpp3068–3141`; native target validator, aura duration producer and `scripts/empower_evoker.rs::build_fire_breath_release`.

The reported first-target dispatch bug was retracted by14781: c7e118a75 already keys Prepared readiness/execution by `(action,target)`. No redundant dispatch patch or Living Flame test was added. No DPS, reference, canonical DB or template changes. GREEN/integration remains pending; this is not an API-ready claim.
