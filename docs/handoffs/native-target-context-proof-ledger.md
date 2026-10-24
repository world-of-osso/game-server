# Shared native target context API

Main requested this API for14781's literal Dev adapter. Lifecycle217f38d8f stays frozen in bt1056; tests-only e3ad46ee6 has6follow-upGREEN/15unique cases with prior14. API branch includes master8e7c7a685 via5d9f60fc5.14781's72f993fdf metadata visibility seam was cherry-picked as61b9fc3dd; native cast behavior is unchanged.

## Proof ledger

Diagnostics: `/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/arms-parity-2026-10-23/`; native input is recorded read-only snapshotdd1de231e32c, not a later DPS measurement freeze.

| Revision / scope | Result | Receipt |
|---|---|---|
|68e24f49c / historical --bin game-server |11m44compile then0tests after catalog/main→lib split; INVALID, not RED | `target-context-red.*` retained; count guard rejected it |
|4be688315 / --lib native_target_context_ --ignored |8genuine assertionRED15.86s; real protocol rank1 Fire Breath20s premise passes.2scaffold dead-field warnings | `target-context-lib-red.{log,proof.json}` |
|6b4a2adb1 / same8cases |7PASS/1invalid range-oracle FAIL35.44s;18m49compile/0warnings. Seven passing cases retained | `target-context-green.{log,proof.json}`, binarySHA49a2e5b4… |
|6b compiled binary / normal simc_apl_native:: |11PASS20.32s,0FAIL,15ignored in-progress cases; no rebuild. Raw receipt HEADf66 was test-only, production/normal11 scope identical6b | `target-context-native-controls.*`; `.binary-source.proof.json` pins actual6b source and exact hash |
|f66d30902 / corrected range case only |1PASS24.84s;15m42compile/0warnings. No disjoint seven-case or11-control rerun | `target-range-green.{log,proof.json}`, `target-range-green-binary.proof.json`; SHA c207c42bf30fa16517f86bd9b0ed0f7bebe5bf4fd107efeeed5951508c54f191 |

All8valid API cases are qualified across the seven retained passes plus corrected range1;11disjoint existing-fact controls also pass. Eight tests enabled afterward without body/production changes. Lead integration remains separate.

### Withdrawn range oracle

Native DB2 167105 is caster-area targets47/16 and self RangeIndex1; its far explicit identity is not a direct-target range failure. That expectation is WITHDRAWN, not patched into production. Native12294 has direct enemy6 and RangeIndex2/five yards; corrected test uses that actual native spell. Audit: `target-range-oracle-audit.json`. Original unsupported-candidate API RED remains genuine; the later range expectation was invalid. No coefficient, template, range or native target rule changed.

## Capability / proof matrix

| Capability | Native representation | Proof / boundary |
|---|---|---|
| Current versus proposed row target | with_targets(current,ordered); Action.target wins; invalid/None explicit context cannot redirect | Native current25% versus actor-selected80%, explicit row80% and expired/None refusal GREEN |
| Ordered per-action candidates | Actual universe order and ordinary native target-only validator; no power/cooldown readiness borrowed from first target | Corrected native12294 near/far/order GREEN |
| active_dot alias | Actual owned/unexpired auras across supplied living targets | Two own/one foreign native Rend instances GREEN |
| Movement event absent in dummy fixture | Pinned no-event values, not inferred from unit movement | exists/remains/in/unknown-property and named dungeon refusal GREEN; configured movement events not implemented |
| casting.react with no native cast | Actual selected target CastState absence is known false | GREEN; active cast reaction remains NATIVE_CASTING_REACTION_UNAVAILABLE, no invented delay |
| Refreshable | Explicit native metadata resolver + Pandemic/final-tick/always/never policy; ordinary native duration mods/unpaid ticks | Owned native Rend policy cases and real protocol rank1 Fire Breath20s versus raw2s window GREEN |

## Consumer API

```rust
NativeFacts::new(world, actor, bindings, encounter, targets.len())
    .with_targets(Some(current), targets)
```

Explicit None/expired current is not replaced by actor selection. Without explicit context, existing consumers read actual native selection; candidate/active_dot queries require a real universe. Forward wrapper `ActionFacts::target_ids` and target-bound `value_for` to this provider. Native target-only validation preserves order; cast readiness stays in Prepared. Unsupported eligibility errors propagate.

Aura aliases stay in `Bindings.buffs`. Register `Bindings.dot_refresh[alias] = DotRefresh { resolve, policy }`; `ResolveDotSpell = fn(&World, Entity, u32) -> Result<SpellInfo,String>`, policy `DotRefreshPolicy::{Pandemic,FinalTick,Always,Never}`. Resolver identity must match alias. `native_dot_spell` resolves ordinary metadata; empowered adapters use an actually eligible stage through the native release helper. Shared `auras::fresh_duration_ms` applies ordinary caster/target duration mods. Raw3572092s is not rank1's20s; FinalTick reuses `auras::remaining_periodic_ticks`. No scheduler reconstruction or snapshot-by-division.

Sources: pinned `action.cpp2435–2508,4630–4645,5362–5453`, `dot.cpp458–477`, `raid_event.cpp3068–3141`; native target validator, aura duration producer and `scripts/empower_evoker.rs::build_fire_breath_release`.

14781 retracted the first-target dispatch report: c7e118a75 already keys Prepared by(action,target). No redundant patch/Living Flame test. No DPS, reference, canonical DB or template changes. Persistent snapshot metadata, FIRST/cycle and composite selection remain open; this packet qualifies only the matrix above.
