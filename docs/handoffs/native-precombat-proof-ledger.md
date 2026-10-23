# Shared literal precombat lifecycle

2026-10-23; implementation/runtime RED pending. [Contract](../specs/dps-parity.md#literal-precombat-execution).

## Observed boundary and scope

Current prepare_reference advances four native frames before explicit fixture engagement. measure then engages before advance_combat loads the WW controller/precombat. Feral's ordinary kit can cast1126 at0.05s before takeover. Pinned optimal_raid0 has no such buff; capture must precede warmup, not erase its aura later. Current Profile also ignores top-level warlock.soul_shards0/default_petfelguard, leaving native initial3 shards.

Pinned player.cpp6138–6183 defines one ordered precombat pass, eligible nonharmful actions and the first ready harmful action. Native implementation must use real cast completion/GCD progression, out of combat before explicit fixture CombatRefs and300s clock. No cast/time/permission/resource bypass. Native pet already matching the configured default makes summon_pet a once-only no-op. WW snapshot/item behavior remains a control.

## First native RED packet

prepare_literal is an explicit entry scaffold mirroring the current late handoff; it still delegates to prepare before capturing. It is not claimed implemented. precombat_tests exercises actual103/266 prepared profiles and current native data:

- Literal Feral must have neither1126 nor an earlier kit1126 CastSuccess at pull; current late capture should fail.
- Nonliteral Feral retains1126/native kit, proving controller isolation.
- Literal Demo must honor declared initial0 shards before precombat and retain native selected Felguard17252; no grants or after-cast resets.
- Native Demo ST must deal first damage within10s after the pull, with one target and no add-wave/re-engagement loop. Parent relayed14861's fresh-canonical0DPS ordering: Felguard30146's6s cast is only0.15s complete at engagement; owner/dummy refs expire at5.2s before completion. The test retains Follower/NPC identity and the normal native controller, so literal takeover cannot itself manufacture a failure. Runtime RED pending;14861's0–10s proof is pending.

All four are opt-in with prepared profile/native source-view reasons. Runtime RED pending before implementation. These cases do not yet qualify ordered multi-action selection, cast/GCD completion at pull, all start options or complete precombat execution. Native first-damage control complements rather than replaces the cast/GCD boundary assertions.

## Dependencies

pd1035 is landed at2333d924b and merged locally; exact-target Prepared/Player dispatch remains14775-owned. Resource query010f7f761 is GREEN9/9 on arms-literal-resource-hooks; it ignores only the requested PowerType affordability while retaining other native checks. Integrate its shared API after pd, not the old613 scaffold. No unselected legacy kit fallback and no WW-specific tier2pc path.
