# Shared native target context API

Main requested this API for14781's literal Dev adapter. Lifecycle217f38d8f stays frozen in bt1056; tests-only acceptance e3ad46ee6 has6follow-upGREEN/15unique cases with prior14. New branch `arms-native-target-api` includes current master8e7c7a685 via5d9f60fc5; conflict resolution retained both wiki index entries.14781's72f993fdf visibility seam cherry-picked as61b9fc3dd; it changes no native cast behavior.

## Active scope / proof matrix

| Capability | Intended native boundary | Proof |
|---|---|---|
| Explicit current target separate from proposed Action.target | Actual entity supplied by controller; invalid/None context cannot redirect to actor selection | RED probe committed; runtime pending |
| Ordered per-action candidates | Supplied native entity order, ordinary native target checks (range etc), never readiness borrowed from first target | RED probe committed; runtime pending |
| active_dot alias | Bound native aura ID; actual owned active auras across supplied native universe, excludes foreign caster | RED probe committed; runtime pending |
| Movement event absent in this fixture | Pinned no-event values, not inferred from unit movement; dungeon timing stays named unavailable | RED probe committed; runtime pending |
| Casting predicate | Actual selected target has no native CastState: known false, not an invented blanket constant; active reaction policy still requires source qualification | No-cast RED probe committed; runtime pending |
| Fire Breath refreshable | Adapter must supply native freshly resolved refresh window; raw357209 duration/max are2s but native empower release rewrites them by rank + Blast Furnace | Pending14781 native alias/window data; no fabricated timer |

`with_targets(current: Option<Entity>, ordered: &[Entity])` currently exists only as a compile scaffold for real behavioral RED; it does not install context yet. `Bindings.dot_refresh` adds explicit freshly resolved native SpellInfo callbacks and sourced Pandemic/final-tick/always/never policies; fields are scaffolding until the property consumer is implemented. Two further probes assert actual owned Rend policy and real native protocol rank1 Fire Breath20s versus raw3572092s refresh windows. No API completion claim. Existing constructors/actual actor selection remain usable; explicit context will never fall back when invalid.

Sources: existing `Action.target`, `ActionFacts::target_ids`; pinned `action.cpp2435–2508,4630–4645,5362–5453`, `dot.cpp458–477`, `raid_event.cpp3068–3141`; native target validator and `scripts/empower_evoker.rs::build_fire_breath_release`. Native `auras::remaining_periodic_ticks` is already on merged master and must be reused rather than reconstructing its phase.

## Invalid initial runner / correction

At68e24f49c the old `--bin game-server` command compiled11m44 then executed0tests after the merged catalog/main split. This is NOT behavioral RED. The runner's count guard rejected it; logs/receipt retained as `target-context-red.*`. Correct target is now `--lib native_target_context_ -- --ignored --nocapture --test-threads=1`, expected8cases. No source fix was inferred from the0-test result.

The reported first-target dispatch bug was retracted by14781 after source review: c7e118a75 already keys Prepared readiness and execution by `(action,target)`. No redundant patch or Living Flame test was added. No DPS, reference, canonical DB or template changes.
